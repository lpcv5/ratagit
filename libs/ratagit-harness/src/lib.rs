use std::collections::VecDeque;
use std::fs::{create_dir_all, write};
use std::path::PathBuf;

use ratagit_core::{Action, AppState, Command, UiAction, update};
use ratagit_git::{GitBackend, MockGitBackend, execute_command};
use ratagit_ui::{RenderedFrame, TerminalSize, render};

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

pub fn run_mock_scenario(
    scenario_name: &str,
    fixture: ratagit_core::RepoSnapshot,
    inputs: &[UiAction],
    expected_ui_contains: &[&str],
    expected_git_ops_contains: &[&str],
) -> Result<(), ScenarioFailure> {
    let mut runtime = Runtime::new(
        AppState::default(),
        MockGitBackend::new(fixture),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    for action in inputs {
        runtime.dispatch_ui(action.clone());
    }

    let frame = runtime.render();
    let frame_text = frame.as_text();
    let operations = runtime.backend().operations().join("\n");

    let mut errors = Vec::new();
    for needle in expected_ui_contains {
        if !frame_text.contains(needle) {
            errors.push(format!("UI missing expected text: {needle}"));
        }
    }
    for needle in expected_git_ops_contains {
        if !operations.contains(needle) {
            errors.push(format!("Git ops missing expected text: {needle}"));
        }
    }

    if errors.is_empty() {
        return Ok(());
    }

    let artifact_dir = write_failure_artifacts(
        scenario_name,
        &frame_text,
        &format!("{:#?}", runtime.state()),
        &operations,
        &format!("{inputs:#?}"),
    );
    Err(ScenarioFailure {
        message: errors.join(" | "),
        artifact_dir,
    })
}

fn write_failure_artifacts(
    scenario_name: &str,
    frame_text: &str,
    app_state_dump: &str,
    operations_text: &str,
    input_text: &str,
) -> PathBuf {
    let base =
        PathBuf::from(std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string()))
            .join("harness-artifacts")
            .join(sanitize_name(scenario_name));
    let _ = create_dir_all(&base);
    let _ = write(base.join("buffer.txt"), frame_text);
    let _ = write(base.join("app_state.txt"), app_state_dump);
    let _ = write(base.join("git_state.txt"), operations_text);
    let _ = write(base.join("input_sequence.txt"), input_text);
    base
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}
