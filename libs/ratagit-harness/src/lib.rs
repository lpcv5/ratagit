use std::collections::{HashMap, VecDeque};
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratagit_core::{Action, AppState, Command, UiAction, debounce_key_for_command, update};
use ratagit_git::{GitBackend, MockGitBackend, execute_command};
use ratagit_ui::{
    RenderedFrame, TerminalBuffer, TerminalSize, buffer_contains_batch_selected_text,
    buffer_contains_selected_text, render, render_terminal_buffer, render_terminal_text,
};

mod async_runtime;
pub use async_runtime::AsyncRuntime;

#[derive(Debug)]
pub struct Runtime<B: GitBackend> {
    state: AppState,
    backend: B,
    terminal_size: TerminalSize,
    debounce_window: Duration,
    debounced: HashMap<&'static str, DebouncedCommand>,
}

#[derive(Debug, Clone)]
struct DebouncedCommand {
    due_at: Instant,
    command: Command,
}

impl<B: GitBackend> Runtime<B> {
    pub fn new(state: AppState, backend: B, terminal_size: TerminalSize) -> Self {
        Self {
            state,
            backend,
            terminal_size,
            debounce_window: Duration::default(),
            debounced: HashMap::new(),
        }
    }

    pub fn with_debounce_window(mut self, debounce_window: Duration) -> Self {
        self.debounce_window = debounce_window;
        self
    }

    pub fn state(&self) -> &AppState {
        &self.state
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    pub fn dispatch_ui(&mut self, action: UiAction) {
        let initial_commands = update(&mut self.state, Action::Ui(action));
        self.process_commands(initial_commands);
    }

    pub fn tick(&mut self) {
        self.flush_due_debounced();
    }

    pub fn render(&self) -> RenderedFrame {
        render(&self.state, self.terminal_size)
    }

    pub fn render_terminal_text(&self) -> String {
        render_terminal_text(&self.state, self.terminal_size)
    }

    pub fn render_terminal_buffer(&self) -> TerminalBuffer {
        render_terminal_buffer(&self.state, self.terminal_size)
    }

    fn process_commands(&mut self, initial: Vec<Command>) {
        let mut queue = VecDeque::new();
        for command in initial {
            self.enqueue_command(command, &mut queue);
        }
        self.process_immediate_queue(queue);
    }

    fn process_immediate_queue(&mut self, mut queue: VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            let git_result = execute_command(&mut self.backend, command);
            let follow_up = update(&mut self.state, Action::GitResult(git_result));
            for command in follow_up {
                self.enqueue_command(command, &mut queue);
            }
        }
    }

    fn enqueue_command(&mut self, command: Command, queue: &mut VecDeque<Command>) {
        if self.debounce_window > Duration::ZERO
            && let Some(key) = debounce_key_for_command(&command)
        {
            self.debounced.insert(
                key,
                DebouncedCommand {
                    due_at: Instant::now() + self.debounce_window,
                    command,
                },
            );
            return;
        }
        enqueue_coalesced_command(queue, command);
    }

    fn flush_due_debounced(&mut self) {
        if self.debounced.is_empty() {
            return;
        }

        let now = Instant::now();
        let due_keys = self
            .debounced
            .iter()
            .filter_map(|(key, pending)| (pending.due_at <= now).then_some(*key))
            .collect::<Vec<_>>();
        if due_keys.is_empty() {
            return;
        }

        let mut queue = VecDeque::new();
        for key in due_keys {
            if let Some(pending) = self.debounced.remove(key) {
                enqueue_coalesced_command(&mut queue, pending.command);
            }
        }
        self.process_immediate_queue(queue);
    }
}

fn enqueue_coalesced_command(queue: &mut VecDeque<Command>, command: Command) {
    let search_start = queue
        .iter()
        .rposition(command_is_mutation)
        .map_or(0, |index| index + 1);
    match command {
        Command::RefreshAll => {
            if !queue
                .iter()
                .skip(search_start)
                .any(|queued| matches!(queued, Command::RefreshAll))
            {
                queue.push_back(Command::RefreshAll);
            }
        }
        Command::RefreshFilesDetailsDiff { .. } => {
            if let Some(index) =
                queue
                    .iter()
                    .enumerate()
                    .skip(search_start)
                    .find_map(|(index, queued)| {
                        matches!(queued, Command::RefreshFilesDetailsDiff { .. }).then_some(index)
                    })
            {
                queue.remove(index);
            }
            queue.push_back(command);
        }
        Command::RefreshBranchDetailsLog { .. } => {
            if let Some(index) =
                queue
                    .iter()
                    .enumerate()
                    .skip(search_start)
                    .find_map(|(index, queued)| {
                        matches!(queued, Command::RefreshBranchDetailsLog { .. }).then_some(index)
                    })
            {
                queue.remove(index);
            }
            queue.push_back(command);
        }
        _ => queue.push_back(command),
    }
}

fn command_is_mutation(command: &Command) -> bool {
    matches!(
        command,
        Command::StageFiles { .. }
            | Command::UnstageFiles { .. }
            | Command::StashFiles { .. }
            | Command::Reset { .. }
            | Command::Nuke
            | Command::DiscardFiles { .. }
            | Command::CreateCommit { .. }
            | Command::CreateBranch { .. }
            | Command::CheckoutBranch { .. }
            | Command::DeleteBranch { .. }
            | Command::RebaseBranch { .. }
            | Command::StashPush { .. }
            | Command::StashPop { .. }
    )
}

#[derive(Debug)]
pub struct ScenarioFailure {
    pub message: String,
    pub artifact_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct MockScenario<'a> {
    pub name: &'a str,
    pub fixture: ratagit_core::RepoSnapshot,
    pub inputs: &'a [UiAction],
    pub terminal_size: TerminalSize,
    pub expectations: ScenarioExpectations<'a>,
}

impl<'a> MockScenario<'a> {
    pub fn new(
        name: &'a str,
        fixture: ratagit_core::RepoSnapshot,
        inputs: &'a [UiAction],
        expectations: ScenarioExpectations<'a>,
    ) -> Self {
        Self {
            name,
            fixture,
            inputs,
            terminal_size: TerminalSize {
                width: 100,
                height: 30,
            },
            expectations,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ScenarioExpectations<'a> {
    pub screen_contains: &'a [&'a str],
    pub screen_not_contains: &'a [&'a str],
    pub selected_screen_rows: &'a [&'a str],
    pub batch_selected_screen_rows: &'a [&'a str],
    pub git_ops_contains: &'a [&'a str],
    pub git_state_contains: &'a [&'a str],
}

pub fn run_mock_scenario(scenario: MockScenario<'_>) -> Result<(), ScenarioFailure> {
    let mut runtime = Runtime::new(
        AppState::default(),
        MockGitBackend::new(scenario.fixture),
        scenario.terminal_size,
    );

    for action in scenario.inputs {
        runtime.dispatch_ui(action.clone());
    }

    let compatibility_frame = runtime.render();
    let frame_text = compatibility_frame.as_text();
    let screen_text = runtime.render_terminal_text();
    let screen_buffer = runtime.render_terminal_buffer();
    let operations = runtime.backend().operations().join("\n");
    let git_state = format!("{:#?}", runtime.backend().snapshot());

    let mut errors = Vec::new();
    for needle in scenario.expectations.screen_contains {
        if !screen_text.contains(needle) {
            errors.push(format!("screen missing expected text: {needle}"));
        }
    }
    for needle in scenario.expectations.screen_not_contains {
        if screen_text.contains(needle) {
            errors.push(format!("screen contains forbidden text: {needle}"));
        }
    }
    for needle in scenario.expectations.selected_screen_rows {
        if !buffer_contains_selected_text(&screen_buffer, needle) {
            errors.push(format!("screen row missing selected style: {needle}"));
        }
    }
    for needle in scenario.expectations.batch_selected_screen_rows {
        if !buffer_contains_batch_selected_text(&screen_buffer, needle) {
            errors.push(format!("screen row missing batch selected style: {needle}"));
        }
    }
    for needle in scenario.expectations.git_ops_contains {
        if !operations.contains(needle) {
            errors.push(format!("Git ops missing expected text: {needle}"));
        }
    }
    for needle in scenario.expectations.git_state_contains {
        if !git_state.contains(needle) {
            errors.push(format!("Git state missing expected text: {needle}"));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }

    let artifact_dir = write_failure_artifacts(
        scenario.name,
        &frame_text,
        &screen_text,
        &format!("{:#?}", runtime.state()),
        &operations,
        &git_state,
        &format!("{:#?}", scenario.inputs),
    );
    Err(ScenarioFailure {
        message: errors.join(" | "),
        artifact_dir,
    })
}

fn write_failure_artifacts(
    scenario_name: &str,
    frame_text: &str,
    screen_text: &str,
    app_state_dump: &str,
    operations_text: &str,
    git_state_text: &str,
    input_text: &str,
) -> PathBuf {
    let base = artifact_root()
        .join("harness-artifacts")
        .join(sanitize_name(scenario_name));
    let _ = create_dir_all(&base);
    let _ = write(base.join("buffer.txt"), frame_text);
    let _ = write(base.join("screen.txt"), screen_text);
    let _ = write(base.join("app_state.txt"), app_state_dump);
    let _ = write(base.join("git_ops.txt"), operations_text);
    let _ = write(base.join("git_state.txt"), git_state_text);
    let _ = write(base.join("input_sequence.txt"), input_text);
    base
}

fn artifact_root() -> PathBuf {
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        return PathBuf::from(target_dir);
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|libs_dir| libs_dir.parent())
        .map(|workspace_root| workspace_root.join("target"))
        .unwrap_or_else(|| PathBuf::from("target"))
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use ratagit_testkit::fixture_dirty_repo;

    use super::*;

    #[test]
    fn refresh_details_diff_runs_immediately_when_debounce_is_disabled() {
        let mut runtime = Runtime::new(
            AppState::default(),
            MockGitBackend::new(fixture_dirty_repo()),
            TerminalSize {
                width: 100,
                height: 30,
            },
        );

        runtime.dispatch_ui(UiAction::RefreshAll);

        let operations = runtime.backend().operations();
        assert!(operations.iter().any(|op| op == "refresh"));
        assert!(operations.iter().any(|op| op == "details-diff:README.md"));
    }

    #[test]
    fn files_details_diff_is_debounced_to_latest_command() {
        let mut runtime = Runtime::new(
            AppState::default(),
            MockGitBackend::new(fixture_dirty_repo()),
            TerminalSize {
                width: 100,
                height: 30,
            },
        )
        .with_debounce_window(Duration::from_millis(50));

        runtime.dispatch_ui(UiAction::RefreshAll);
        runtime.dispatch_ui(UiAction::MoveDown);
        runtime.dispatch_ui(UiAction::MoveDown);

        let operations_before_tick = runtime.backend().operations();
        assert!(operations_before_tick.iter().any(|op| op == "refresh"));
        assert!(
            !operations_before_tick
                .iter()
                .any(|op| op.starts_with("details-diff:"))
        );

        std::thread::sleep(Duration::from_millis(80));
        runtime.tick();

        let diff_operations = runtime
            .backend()
            .operations()
            .iter()
            .filter(|op| op.starts_with("details-diff:"))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(diff_operations, vec!["details-diff:src/lib.rs".to_string()]);
    }

    #[test]
    fn command_coalescing_preserves_mutation_boundaries() {
        let mut queue = std::collections::VecDeque::new();
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);
        enqueue_coalesced_command(
            &mut queue,
            Command::StageFiles {
                paths: vec!["a.txt".to_string()],
            },
        );
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);
        enqueue_coalesced_command(&mut queue, Command::RefreshAll);

        assert_eq!(
            queue.into_iter().collect::<Vec<_>>(),
            vec![
                Command::RefreshAll,
                Command::StageFiles {
                    paths: vec!["a.txt".to_string()]
                },
                Command::RefreshAll,
            ]
        );
    }

    #[test]
    fn command_coalescing_keeps_latest_details_after_last_mutation() {
        let mut queue = std::collections::VecDeque::new();
        enqueue_coalesced_command(
            &mut queue,
            Command::RefreshFilesDetailsDiff {
                paths: vec!["old.txt".to_string()],
            },
        );
        enqueue_coalesced_command(
            &mut queue,
            Command::StageFiles {
                paths: vec!["a.txt".to_string()],
            },
        );
        enqueue_coalesced_command(
            &mut queue,
            Command::RefreshFilesDetailsDiff {
                paths: vec!["stale.txt".to_string()],
            },
        );
        enqueue_coalesced_command(
            &mut queue,
            Command::RefreshFilesDetailsDiff {
                paths: vec!["latest.txt".to_string()],
            },
        );

        assert_eq!(
            queue.into_iter().collect::<Vec<_>>(),
            vec![
                Command::RefreshFilesDetailsDiff {
                    paths: vec!["old.txt".to_string()]
                },
                Command::StageFiles {
                    paths: vec!["a.txt".to_string()]
                },
                Command::RefreshFilesDetailsDiff {
                    paths: vec!["latest.txt".to_string()]
                },
            ]
        );
    }
}
