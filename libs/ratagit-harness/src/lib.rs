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
use serde::Serialize;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioAssertionKind {
    ScreenContains,
    ScreenNotContains,
    SelectedScreenRows,
    BatchSelectedScreenRows,
    GitOpsContains,
    GitStateContains,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScenarioAssertionFailure {
    pub kind: ScenarioAssertionKind,
    pub needle: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScenarioExpectationReport {
    pub screen_contains: Vec<String>,
    pub screen_not_contains: Vec<String>,
    pub selected_screen_rows: Vec<String>,
    pub batch_selected_screen_rows: Vec<String>,
    pub git_ops_contains: Vec<String>,
    pub git_state_contains: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ScenarioTerminalSizeReport {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScenarioArtifactFiles {
    pub compatibility_buffer: &'static str,
    pub screen: &'static str,
    pub app_context: &'static str,
    pub git_ops: &'static str,
    pub git_state: &'static str,
    pub input_sequence: &'static str,
    pub failure_report: &'static str,
}

impl Default for ScenarioArtifactFiles {
    fn default() -> Self {
        Self {
            compatibility_buffer: "buffer.txt",
            screen: "screen.txt",
            app_context: "app_context.txt",
            git_ops: "git_ops.txt",
            git_state: "git_state.txt",
            input_sequence: "input_sequence.txt",
            failure_report: "failure_report.json",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScenarioFailureReport {
    pub schema_version: u32,
    pub scenario_name: String,
    pub terminal_size: ScenarioTerminalSizeReport,
    pub inputs: Vec<String>,
    pub expectations: ScenarioExpectationReport,
    pub assertion_failures: Vec<ScenarioAssertionFailure>,
    pub git_operations: Vec<String>,
    pub app_context_debug: String,
    pub git_state_debug: String,
    pub artifacts: ScenarioArtifactFiles,
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
    let app_context_debug = format!("{:#?}", runtime.state());
    let input_debug = format!("{:#?}", scenario.inputs);

    let mut failures = Vec::new();
    for needle in scenario.expectations.screen_contains {
        if !screen_text.contains(needle) {
            failures.push(assertion_failure(
                ScenarioAssertionKind::ScreenContains,
                needle,
                format!("screen missing expected text: {needle}"),
            ));
        }
    }
    for needle in scenario.expectations.screen_not_contains {
        if screen_text.contains(needle) {
            failures.push(assertion_failure(
                ScenarioAssertionKind::ScreenNotContains,
                needle,
                format!("screen contains forbidden text: {needle}"),
            ));
        }
    }
    for needle in scenario.expectations.selected_screen_rows {
        if !buffer_contains_selected_text(&screen_buffer, needle) {
            failures.push(assertion_failure(
                ScenarioAssertionKind::SelectedScreenRows,
                needle,
                format!("screen row missing selected style: {needle}"),
            ));
        }
    }
    for needle in scenario.expectations.batch_selected_screen_rows {
        if !buffer_contains_batch_selected_text(&screen_buffer, needle) {
            failures.push(assertion_failure(
                ScenarioAssertionKind::BatchSelectedScreenRows,
                needle,
                format!("screen row missing batch selected style: {needle}"),
            ));
        }
    }
    for needle in scenario.expectations.git_ops_contains {
        if !operations.contains(needle) {
            failures.push(assertion_failure(
                ScenarioAssertionKind::GitOpsContains,
                needle,
                format!("Git ops missing expected text: {needle}"),
            ));
        }
    }
    for needle in scenario.expectations.git_state_contains {
        if !git_state.contains(needle) {
            failures.push(assertion_failure(
                ScenarioAssertionKind::GitStateContains,
                needle,
                format!("Git state missing expected text: {needle}"),
            ));
        }
    }

    if failures.is_empty() {
        return Ok(());
    }

    let failure_report = ScenarioFailureReport {
        schema_version: 1,
        scenario_name: scenario.name.to_string(),
        terminal_size: ScenarioTerminalSizeReport {
            width: scenario.terminal_size.width,
            height: scenario.terminal_size.height,
        },
        inputs: scenario
            .inputs
            .iter()
            .map(|input| format!("{input:?}"))
            .collect(),
        expectations: ScenarioExpectationReport::from(scenario.expectations),
        assertion_failures: failures,
        git_operations: operations.lines().map(str::to_string).collect(),
        app_context_debug: app_context_debug.clone(),
        git_state_debug: git_state.clone(),
        artifacts: ScenarioArtifactFiles::default(),
    };
    let artifact_dir = write_failure_artifacts(
        scenario.name,
        FailureArtifactPayload {
            frame_text: &frame_text,
            screen_text: &screen_text,
            app_context_dump: &app_context_debug,
            operations_text: &operations,
            git_state_text: &git_state,
            input_text: &input_debug,
            failure_report: &failure_report,
        },
    );
    Err(ScenarioFailure {
        message: failure_report
            .assertion_failures
            .iter()
            .map(|failure| failure.message.as_str())
            .collect::<Vec<_>>()
            .join(" | "),
        artifact_dir,
    })
}

impl ScenarioExpectationReport {
    fn from(expectations: ScenarioExpectations<'_>) -> Self {
        Self {
            screen_contains: strings(expectations.screen_contains),
            screen_not_contains: strings(expectations.screen_not_contains),
            selected_screen_rows: strings(expectations.selected_screen_rows),
            batch_selected_screen_rows: strings(expectations.batch_selected_screen_rows),
            git_ops_contains: strings(expectations.git_ops_contains),
            git_state_contains: strings(expectations.git_state_contains),
        }
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn assertion_failure(
    kind: ScenarioAssertionKind,
    needle: &str,
    message: String,
) -> ScenarioAssertionFailure {
    ScenarioAssertionFailure {
        kind,
        needle: needle.to_string(),
        message,
    }
}

struct FailureArtifactPayload<'a> {
    frame_text: &'a str,
    screen_text: &'a str,
    app_context_dump: &'a str,
    operations_text: &'a str,
    git_state_text: &'a str,
    input_text: &'a str,
    failure_report: &'a ScenarioFailureReport,
}

fn write_failure_artifacts(scenario_name: &str, payload: FailureArtifactPayload<'_>) -> PathBuf {
    let base = artifact_root()
        .join("harness-artifacts")
        .join(sanitize_name(scenario_name));
    let _ = create_dir_all(&base);
    let _ = write(base.join("buffer.txt"), payload.frame_text);
    let _ = write(base.join("screen.txt"), payload.screen_text);
    let _ = write(base.join("app_context.txt"), payload.app_context_dump);
    let _ = write(base.join("git_ops.txt"), payload.operations_text);
    let _ = write(base.join("git_state.txt"), payload.git_state_text);
    let _ = write(base.join("input_sequence.txt"), payload.input_text);
    let report_json = serde_json::to_string_pretty(payload.failure_report)
        .expect("failure report should serialize");
    let _ = write(base.join("failure_report.json"), report_json);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScenarioArea {
    Async,
    Branches,
    CommitFiles,
    Commits,
    Error,
    Files,
    Global,
    LargeRepo,
    Stash,
    Ui,
    Other,
}

impl ScenarioArea {
    const ORDER: [Self; 11] = [
        Self::Async,
        Self::Global,
        Self::Files,
        Self::Branches,
        Self::Commits,
        Self::CommitFiles,
        Self::Stash,
        Self::LargeRepo,
        Self::Ui,
        Self::Error,
        Self::Other,
    ];

    fn title(self) -> &'static str {
        match self {
            Self::Async => "Async",
            Self::Branches => "Branches",
            Self::CommitFiles => "Commit Files",
            Self::Commits => "Commits",
            Self::Error => "Error",
            Self::Files => "Files",
            Self::Global => "Global",
            Self::LargeRepo => "Large Repo",
            Self::Stash => "Stash",
            Self::Ui => "UI",
            Self::Other => "Other",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarnessScenarioEntry {
    pub name: String,
    pub area: ScenarioArea,
}

pub fn parse_harness_scenarios(source: &str) -> Vec<HarnessScenarioEntry> {
    source
        .lines()
        .filter_map(parse_harness_scenario_line)
        .map(|name| HarnessScenarioEntry {
            area: infer_scenario_area(name),
            name: name.to_string(),
        })
        .collect()
}

pub fn infer_scenario_area(name: &str) -> ScenarioArea {
    let name = name.strip_prefix("harness_").unwrap_or(name);
    if name.starts_with("async_") {
        ScenarioArea::Async
    } else if name.starts_with("global_") {
        ScenarioArea::Global
    } else if name.starts_with("files_") {
        ScenarioArea::Files
    } else if name.starts_with("branches_") || name.starts_with("branch_") {
        ScenarioArea::Branches
    } else if name.starts_with("commit_files_") {
        ScenarioArea::CommitFiles
    } else if name.starts_with("commits_") {
        ScenarioArea::Commits
    } else if name.starts_with("stash_") {
        ScenarioArea::Stash
    } else if name.starts_with("large_repo_") || name.starts_with("huge_repo_") {
        ScenarioArea::LargeRepo
    } else if name.starts_with("ui_") || name.starts_with("panel_") || name.contains("_ui_") {
        ScenarioArea::Ui
    } else if name.starts_with("error_") || name.contains("_error_") {
        ScenarioArea::Error
    } else {
        ScenarioArea::Other
    }
}

pub fn render_harness_scenario_manifest(source: &str) -> String {
    let scenarios = parse_harness_scenarios(source);
    let mut manifest = String::new();
    manifest.push_str("# Harness Scenarios\n\n");
    manifest.push_str("This file is generated from `libs/ratagit-harness/tests/harness.rs`.\n");
    manifest.push_str("Detailed assertions live in the Rust scenario definitions.\n\n");
    manifest.push_str(&format!("Total scenarios: {}\n", scenarios.len()));

    for area in ScenarioArea::ORDER {
        let names = scenarios
            .iter()
            .filter(|scenario| scenario.area == area)
            .map(|scenario| scenario.name.as_str())
            .collect::<Vec<_>>();
        if names.is_empty() {
            continue;
        }
        manifest.push_str(&format!("\n## {} ({})\n\n", area.title(), names.len()));
        for name in names {
            manifest.push_str(&format!("- `{name}`\n"));
        }
    }

    manifest
}

fn parse_harness_scenario_line(line: &str) -> Option<&str> {
    let line = line.trim();
    let signature = line.strip_prefix("fn harness_")?;
    let name_end = signature.find('(')?;
    Some(&line["fn ".len()..("fn harness_".len() + name_end)])
}

#[cfg(test)]
mod tests {
    use std::fs::read_to_string;
    use std::time::Duration;

    use ratagit_testkit::{fixture_dirty_repo, fixture_empty_repo};

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

    #[test]
    fn failed_scenario_writes_typed_json_report_and_text_artifacts() {
        let inputs = [UiAction::RefreshAll];
        let result = run_mock_scenario(MockScenario::new(
            "unit_failure_report_screen_contains",
            fixture_empty_repo(),
            &inputs,
            ScenarioExpectations {
                screen_contains: &["definitely not on screen"],
                screen_not_contains: &[],
                selected_screen_rows: &[],
                batch_selected_screen_rows: &[],
                git_ops_contains: &[],
                git_state_contains: &[],
            },
        ));

        let failure = result.expect_err("scenario should fail");
        assert!(failure.artifact_dir.join("buffer.txt").exists());
        assert!(failure.artifact_dir.join("screen.txt").exists());
        assert!(failure.artifact_dir.join("app_context.txt").exists());
        assert!(failure.artifact_dir.join("git_ops.txt").exists());
        assert!(failure.artifact_dir.join("git_state.txt").exists());
        assert!(failure.artifact_dir.join("input_sequence.txt").exists());

        let report_path = failure.artifact_dir.join("failure_report.json");
        let report: serde_json::Value =
            serde_json::from_str(&read_to_string(report_path).expect("report should exist"))
                .expect("report should be valid json");
        assert_eq!(report["schema_version"], 1);
        assert_eq!(
            report["scenario_name"],
            "unit_failure_report_screen_contains"
        );
        assert_eq!(report["terminal_size"]["width"], 100);
        assert_eq!(report["inputs"][0], "RefreshAll");
        assert_eq!(
            report["expectations"]["screen_contains"][0],
            "definitely not on screen"
        );
        assert_eq!(report["assertion_failures"][0]["kind"], "screen_contains");
        assert_eq!(
            report["assertion_failures"][0]["needle"],
            "definitely not on screen"
        );
        assert_eq!(report["git_operations"][0], "refresh-files");
        assert!(
            report["app_context_debug"]
                .as_str()
                .expect("app context should be a string")
                .contains("AppContext")
        );
        assert!(
            report["git_state_debug"]
                .as_str()
                .expect("git state should be a string")
                .contains("RepoSnapshot")
        );
        assert_eq!(report["artifacts"]["failure_report"], "failure_report.json");
    }

    #[test]
    fn scenario_manifest_parser_finds_harness_functions() {
        let scenarios = parse_harness_scenarios(include_str!("../tests/harness.rs"));

        assert!(scenarios.iter().any(|scenario| {
            scenario.name == "harness_files_stage_and_unstage"
                && scenario.area == ScenarioArea::Files
        }));
        assert!(scenarios.iter().any(|scenario| {
            scenario.name == "harness_branches_create_and_checkout"
                && scenario.area == ScenarioArea::Branches
        }));
        assert!(scenarios.iter().any(|scenario| {
            scenario.name == "harness_commits_create_and_refresh"
                && scenario.area == ScenarioArea::Commits
        }));
    }

    #[test]
    fn scenario_area_inference_handles_representative_prefixes() {
        assert_eq!(
            infer_scenario_area("harness_commit_files_search_selects_file_and_refreshes_diff"),
            ScenarioArea::CommitFiles
        );
        assert_eq!(
            infer_scenario_area(
                "harness_huge_repo_status_skips_file_scan_without_blocking_commits"
            ),
            ScenarioArea::LargeRepo
        );
        assert_eq!(
            infer_scenario_area("harness_global_pull_and_push_sync_repo"),
            ScenarioArea::Global
        );
        assert_eq!(
            infer_scenario_area("harness_error_visible_without_crash"),
            ScenarioArea::Error
        );
    }

    #[test]
    fn generated_scenario_manifest_matches_docs() {
        let generated = render_harness_scenario_manifest(include_str!("../tests/harness.rs"));
        let docs_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|libs_dir| libs_dir.parent())
            .expect("workspace root should resolve")
            .join("docs")
            .join("harness")
            .join("SCENARIOS.md");
        let current = read_to_string(&docs_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", docs_path.display()));

        assert_eq!(
            normalize_newlines(&current),
            normalize_newlines(&generated),
            "docs/harness/SCENARIOS.md is out of date; regenerate it after adding or renaming harness scenarios"
        );
    }

    fn normalize_newlines(value: &str) -> String {
        value.replace("\r\n", "\n")
    }
}
