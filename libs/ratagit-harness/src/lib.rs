use std::collections::VecDeque;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;

use ratagit_core::{Action, AppState, Command, UiAction, update};
use ratagit_git::{GitBackend, MockGitBackend, execute_command};
use ratagit_ui::{
    RenderedFrame, TerminalBuffer, TerminalSize, buffer_contains_selected_text, render,
    render_terminal_buffer, render_terminal_text,
};

#[derive(Debug)]
pub struct Runtime<B: GitBackend> {
    state: AppState,
    backend: B,
    terminal_size: TerminalSize,
}

impl<B: GitBackend> Runtime<B> {
    pub fn new(state: AppState, backend: B, terminal_size: TerminalSize) -> Self {
        Self {
            state,
            backend,
            terminal_size,
        }
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
        let mut queue = VecDeque::from(initial);
        while let Some(command) = queue.pop_front() {
            let git_result = execute_command(&mut self.backend, command);
            let follow_up = update(&mut self.state, Action::GitResult(git_result));
            for command in follow_up {
                queue.push_back(command);
            }
        }
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
    pub selected_screen_rows: &'a [&'a str],
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
    for needle in scenario.expectations.selected_screen_rows {
        if !buffer_contains_selected_text(&screen_buffer, needle) {
            errors.push(format!("screen row missing selected style: {needle}"));
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
