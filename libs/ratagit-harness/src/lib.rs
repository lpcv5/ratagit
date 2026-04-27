use std::collections::VecDeque;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use ratagit_core::{Action, AppContext, Command, UiAction, update};
use ratagit_git::{GitBackend, MockGitBackend, execute_command};
use ratagit_ui::{
    RenderedFrame, TerminalBuffer, TerminalSize, buffer_contains_batch_selected_text,
    buffer_contains_selected_text, render, render_terminal_buffer, render_terminal_text,
};

mod async_runtime;
mod scheduler;
pub use async_runtime::AsyncRuntime;

use scheduler::CommandScheduler;

#[derive(Debug)]
pub struct Runtime<B: GitBackend> {
    state: AppContext,
    backend: B,
    terminal_size: TerminalSize,
    scheduler: CommandScheduler,
}

impl<B: GitBackend> Runtime<B> {
    pub fn new(state: AppContext, backend: B, terminal_size: TerminalSize) -> Self {
        Self {
            state,
            backend,
            terminal_size,
            scheduler: CommandScheduler::default(),
        }
    }

    pub fn with_debounce_window(mut self, debounce_window: Duration) -> Self {
        self.scheduler.set_debounce_window(debounce_window);
        self
    }

    pub fn state(&self) -> &AppContext {
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
        let now = Instant::now();
        for command in initial {
            self.scheduler.enqueue_at(command, &mut queue, now);
        }
        self.process_immediate_queue(queue);
    }

    fn process_immediate_queue(&mut self, mut queue: VecDeque<Command>) {
        while let Some(command) = queue.pop_front() {
            let git_result = execute_command(&mut self.backend, command);
            let follow_up = update(&mut self.state, Action::GitResult(git_result));
            let now = Instant::now();
            for command in follow_up {
                self.scheduler.enqueue_at(command, &mut queue, now);
            }
        }
    }

    fn flush_due_debounced(&mut self) {
        let queue = self.scheduler.flush_due_at(Instant::now());
        self.process_immediate_queue(queue);
    }
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
        AppContext::default(),
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
    app_context_dump: &str,
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
    let _ = write(base.join("app_context.txt"), app_context_dump);
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
            AppContext::default(),
            MockGitBackend::new(fixture_dirty_repo()),
            TerminalSize {
                width: 100,
                height: 30,
            },
        );

        runtime.dispatch_ui(UiAction::RefreshAll);

        let operations = runtime.backend().operations();
        assert!(operations.iter().any(|op| op == "refresh-files"));
        assert!(operations.iter().any(|op| op == "details-diff:README.md"));
    }

    #[test]
    fn files_details_diff_is_debounced_to_latest_command() {
        let mut runtime = Runtime::new(
            AppContext::default(),
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
        assert!(
            operations_before_tick
                .iter()
                .any(|op| op == "refresh-files")
        );
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
}
