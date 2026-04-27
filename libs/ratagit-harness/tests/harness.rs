use std::collections::BTreeMap;
use std::io;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use ratagit_core::{
    Action, AppState, BranchDeleteMode, BranchEntry, CommitEntry, FileDiffTarget, FilesSnapshot,
    GitResult, RepoSnapshot, ResetMode, StashEntry, UiAction, update,
};
use ratagit_git::{GitBackend, GitError, MockGitBackend};
use ratagit_harness::{
    AsyncRuntime, MockScenario, Runtime, ScenarioExpectations, run_mock_scenario,
};
use ratagit_testkit::{
    fixture_commit, fixture_dirty_repo, fixture_empty_repo, fixture_file, fixture_many_files,
};
use ratagit_ui::{
    TerminalSize, details_content_lines_for_terminal_size, details_scroll_lines_for_terminal_size,
    render_terminal_buffer_with_cursor,
};
use tracing::Level;

fn assert_scenario(scenario: MockScenario<'_>) {
    let result = run_mock_scenario(scenario);
    assert!(result.is_ok(), "{result:?}");
}

fn clean_commit_fixture() -> RepoSnapshot {
    let mut fixture = fixture_dirty_repo();
    fixture.status_summary = "clean".to_string();
    fixture.files.clear();
    fixture
}

fn clean_three_commit_fixture() -> RepoSnapshot {
    let mut fixture = clean_commit_fixture();
    fixture
        .commits
        .push(fixture_commit("ghi9012", "older base"));
    fixture.commits.push(fixture_commit("jkl3456", "root base"));
    fixture
}

fn clean_many_commit_fixture(count: usize) -> RepoSnapshot {
    let mut fixture = clean_commit_fixture();
    fixture.commits = (0..count)
        .map(|index| fixture_commit(&format!("c{index:06}"), &format!("commit {index}")))
        .collect();
    fixture
}

fn large_repo_backend(fixture: RepoSnapshot) -> MockGitBackend {
    MockGitBackend::with_status_metadata(fixture, 100_000, true, false, true)
}

fn huge_repo_backend(mut fixture: RepoSnapshot) -> MockGitBackend {
    fixture.files.clear();
    fixture.status_summary = "status scan skipped: 1000000 indexed files".to_string();
    MockGitBackend::with_huge_repo_status_metadata(fixture, 1_000_000)
}

#[derive(Debug, Clone)]
struct BlockingBackend {
    inner: Arc<Mutex<MockGitBackend>>,
    refresh_started: Sender<()>,
    refresh_release: Arc<Mutex<Receiver<()>>>,
}

impl GitBackend for BlockingBackend {
    fn refresh_snapshot(&mut self) -> Result<RepoSnapshot, GitError> {
        let _ = self.refresh_started.send(());
        self.refresh_release
            .lock()
            .expect("release lock")
            .recv_timeout(Duration::from_secs(2))
            .expect("test should release refresh");
        self.inner.lock().expect("mock lock").refresh_snapshot()
    }

    fn load_more_commits(
        &mut self,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<ratagit_core::CommitEntry>, GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .load_more_commits(offset, limit)
    }

    fn refresh_files(&mut self) -> Result<FilesSnapshot, GitError> {
        let _ = self.refresh_started.send(());
        self.refresh_release
            .lock()
            .expect("release lock")
            .recv_timeout(Duration::from_secs(2))
            .expect("test should release files refresh");
        self.inner.lock().expect("mock lock").refresh_files()
    }

    fn refresh_branches(&mut self) -> Result<Vec<BranchEntry>, GitError> {
        self.inner.lock().expect("mock lock").refresh_branches()
    }

    fn refresh_commits(&mut self) -> Result<Vec<CommitEntry>, GitError> {
        self.inner.lock().expect("mock lock").refresh_commits()
    }

    fn refresh_stashes(&mut self) -> Result<Vec<StashEntry>, GitError> {
        self.inner.lock().expect("mock lock").refresh_stashes()
    }

    fn files_details_diff(&mut self, targets: &[FileDiffTarget]) -> Result<String, GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .files_details_diff(targets)
    }

    fn branch_details_log(&mut self, branch: &str, max_count: usize) -> Result<String, GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .branch_details_log(branch, max_count)
    }

    fn commit_details_diff(&mut self, commit_id: &str) -> Result<String, GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .commit_details_diff(commit_id)
    }

    fn commit_files(
        &mut self,
        commit_id: &str,
    ) -> Result<Vec<ratagit_core::CommitFileEntry>, GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .commit_files(commit_id)
    }

    fn commit_file_diff(
        &mut self,
        target: &ratagit_core::CommitFileDiffTarget,
    ) -> Result<String, GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .commit_file_diff(target)
    }

    fn stage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").stage_file(path)
    }

    fn unstage_file(&mut self, path: &str) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").unstage_file(path)
    }

    fn stage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").stage_files(paths)
    }

    fn unstage_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").unstage_files(paths)
    }

    fn create_commit(&mut self, message: &str) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").create_commit(message)
    }

    fn create_branch(&mut self, name: &str, start_point: &str) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .create_branch(name, start_point)
    }

    fn checkout_branch(&mut self, name: &str, auto_stash: bool) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .checkout_branch(name, auto_stash)
    }

    fn delete_branch(
        &mut self,
        name: &str,
        mode: BranchDeleteMode,
        force: bool,
    ) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .delete_branch(name, mode, force)
    }

    fn rebase_branch(
        &mut self,
        target: &str,
        interactive: bool,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .rebase_branch(target, interactive, auto_stash)
    }

    fn squash_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .squash_commits(commit_ids)
    }

    fn fixup_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .fixup_commits(commit_ids)
    }

    fn reword_commit(&mut self, commit_id: &str, message: &str) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .reword_commit(commit_id, message)
    }

    fn delete_commits(&mut self, commit_ids: &[String]) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .delete_commits(commit_ids)
    }

    fn checkout_commit_detached(
        &mut self,
        commit_id: &str,
        auto_stash: bool,
    ) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .checkout_commit_detached(commit_id, auto_stash)
    }

    fn stash_push(&mut self, message: &str) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").stash_push(message)
    }

    fn stash_files(&mut self, message: &str, paths: &[String]) -> Result<(), GitError> {
        self.inner
            .lock()
            .expect("mock lock")
            .stash_files(message, paths)
    }

    fn stash_pop(&mut self, stash_id: &str) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").stash_pop(stash_id)
    }

    fn reset(&mut self, mode: ResetMode) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").reset(mode)
    }

    fn nuke(&mut self) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").nuke()
    }

    fn discard_files(&mut self, paths: &[String]) -> Result<(), GitError> {
        self.inner.lock().expect("mock lock").discard_files(paths)
    }
}

#[test]
fn harness_status_refresh() {
    let inputs = [UiAction::RefreshAll];
    assert_scenario(MockScenario::new(
        "mvp_status_refresh",
        fixture_empty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Files", "Details", "keys(files):"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_large_repo_fast_status_shows_notice_without_full_refresh() {
    let mut fixture = fixture_dirty_repo();
    fixture.files = vec![
        fixture_file("src/lib.rs", false, false),
        fixture_file("src/main.rs", true, false),
    ];
    let mut runtime = Runtime::new(
        AppState::default(),
        large_repo_backend(fixture),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    runtime.dispatch_ui(UiAction::RefreshAll);

    let screen = runtime.render_terminal_text();
    let operations = runtime.backend().operations().join("\n");
    assert!(screen.contains("status=large repo fast mode; untracked scan skipped"));
    assert!(screen.contains("tip=consider git untrackedCache/fsmonitor/splitIndex"));
    assert!(screen.contains("src/"));
    assert!(operations.contains("refresh-files"));
    assert!(!operations.lines().any(|operation| operation == "refresh"));
}

#[test]
fn harness_large_repo_fast_status_is_stable_with_tracing_enabled() {
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(io::sink)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let mut fixture = fixture_dirty_repo();
        fixture.files = vec![
            fixture_file("src/lib.rs", false, false),
            fixture_file("src/main.rs", true, false),
        ];
        let mut runtime = Runtime::new(
            AppState::default(),
            large_repo_backend(fixture),
            TerminalSize {
                width: 100,
                height: 30,
            },
        );

        runtime.dispatch_ui(UiAction::RefreshAll);

        let screen = runtime.render_terminal_text();
        let operations = runtime.backend().operations().join("\n");
        let git_state = format!("{:#?}", runtime.backend().snapshot());
        assert!(screen.contains("status=large repo fast mode; untracked scan skipped"));
        assert!(operations.contains("refresh-files"));
        assert!(git_state.contains("path: \"src/lib.rs\""));
    });
}

#[test]
fn harness_huge_repo_status_skips_file_scan_without_blocking_commits() {
    let mut runtime = Runtime::new(
        AppState::default(),
        huge_repo_backend(fixture_dirty_repo()),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    runtime.dispatch_ui(UiAction::RefreshAll);
    runtime.dispatch_ui(UiAction::FocusNext);
    runtime.dispatch_ui(UiAction::FocusNext);

    let screen = runtime.render_terminal_text();
    let operations = runtime.backend().operations().join("\n");
    let git_state = format!("{:#?}", runtime.backend().snapshot());
    assert!(screen.contains("status=huge repo metadata-only; file scan skipped"));
    assert!(screen.contains("tip=focus Commits/Branches or narrow Git outside ratagit"));
    assert!(screen.contains("init project"));
    assert!(screen.contains("diff --git a/commit.txt b/commit.txt"));
    assert!(!screen.contains("README.md"));
    assert!(operations.contains("refresh-files"));
    assert!(operations.contains("refresh-commits"));
    assert!(operations.contains("commit-diff:abc1234"));
    assert!(
        !operations
            .lines()
            .any(|operation| operation == "details-diff:README.md")
    );
    assert!(git_state.contains("current_branch: \"main\""));
}

#[test]
fn harness_large_directory_details_limits_diff_targets() {
    let mut fixture = fixture_dirty_repo();
    fixture.files = (0..101)
        .map(|index| fixture_file(&format!("src/file-{index:03}.txt"), false, false))
        .collect();
    let mut runtime = Runtime::new(
        AppState::default(),
        large_repo_backend(fixture),
        TerminalSize {
            width: 120,
            height: 34,
        },
    );

    runtime.dispatch_ui(UiAction::RefreshAll);

    let screen = runtime.render_terminal_text();
    let operations = runtime.backend().operations().join("\n");
    assert!(screen.contains("details(files): showing first 100 of 101 files"));
    assert!(screen.contains("details=diff limited to first 100 of 101 files"));
    assert!(operations.contains("details-diff:src/file-000.txt"));
    assert!(operations.contains("src/file-099.txt"));
    assert!(!operations.contains("src/file-100.txt"));
}

#[test]
fn async_runtime_renders_loading_before_refresh_finishes() {
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    let inner = Arc::new(Mutex::new(MockGitBackend::new(fixture_dirty_repo())));
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let backend = BlockingBackend {
        inner: Arc::clone(&inner),
        refresh_started: started_tx,
        refresh_release: Arc::new(Mutex::new(release_rx)),
    };
    let mut runtime = AsyncRuntime::new(AppState::default(), move || backend.clone(), size);

    runtime.dispatch_ui(UiAction::RefreshAll);
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("refresh should start on worker thread");

    for _ in 0..100 {
        runtime.tick();
        if !runtime.state().branches.items.is_empty()
            && !runtime.state().commits.items.is_empty()
            && !runtime.state().stash.items.is_empty()
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let loading_screen = runtime.render_terminal_text();
    assert!(loading_screen.contains("work=refreshing repository"));
    assert!(loading_screen.contains("feature/mvp"));
    assert!(loading_screen.contains("init project"));
    assert!(loading_screen.contains("stash@{0}"));
    assert!(!loading_screen.contains("README.md"));
    assert_eq!(runtime.state().status.refresh_count, 0);

    release_tx.send(()).expect("refresh should be releasable");
    for _ in 0..100 {
        runtime.tick();
        if runtime.state().status.refresh_count == 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(runtime.state().status.refresh_count, 1);
    assert!(runtime.render_terminal_text().contains("README.md"));
    assert!(
        inner
            .lock()
            .expect("mock lock")
            .operations()
            .contains(&"refresh-files".to_string())
    );
}

#[test]
fn async_runtime_drops_blocked_read_result_after_queued_mutation() {
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    let inner = Arc::new(Mutex::new(MockGitBackend::new(fixture_dirty_repo())));
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let backend = BlockingBackend {
        inner,
        refresh_started: started_tx,
        refresh_release: Arc::new(Mutex::new(release_rx)),
    };
    let mut state = AppState::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    state.work.details_pending = false;
    let baseline_refresh_count = state.status.refresh_count;
    let mut runtime = AsyncRuntime::new(state, move || backend.clone(), size);

    runtime.dispatch_ui(UiAction::RefreshAll);
    started_rx
        .recv_timeout(Duration::from_secs(2))
        .expect("refresh should start on read worker");

    runtime.dispatch_ui(UiAction::CreateCommit {
        message: String::new(),
    });
    for _ in 0..100 {
        runtime.tick();
        if runtime
            .state()
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("Failed to create commit"))
        {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        runtime
            .state()
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("Failed to create commit"))
    );

    release_tx.send(()).expect("refresh should be releasable");
    for _ in 0..10 {
        runtime.tick();
        std::thread::sleep(Duration::from_millis(10));
    }

    assert_eq!(runtime.state().status.refresh_count, baseline_refresh_count);
    assert!(
        runtime
            .state()
            .status
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("Failed to create commit"))
    );
}

#[test]
fn harness_files_stage_and_unstage() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::StageSelectedFile,
        UiAction::UnstageSelectedFile,
    ];
    assert_scenario(MockScenario::new(
        "mvp_files_stage_unstage",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Files", "src/lib.rs", "### unstaged"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["stage-files:src/lib.rs", "unstage-files:src/lib.rs"],
            git_state_contains: &["path: \"src/lib.rs\"", "staged: false"],
        },
    ));
}

#[test]
fn harness_files_details_follow_cursor_with_combined_diff_sections() {
    let inputs = [UiAction::RefreshAll, UiAction::MoveDown];
    assert_scenario(MockScenario::new(
        "files_details_follow_cursor_combined_diff",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "### unstaged",
                "diff --git a/src/lib.rs b/src/lib.rs",
                "@@ -1 +1 @@",
                "+new src/lib.rs",
                "### staged",
                "diff --git a/src/main.rs b/src/main.rs",
            ],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "details-diff:README.md",
                "details-diff:src/lib.rs,src/main.rs",
            ],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_files_details_reuses_cached_diff_when_selection_repeats() {
    let mut runtime = Runtime::new(
        AppState::default(),
        MockGitBackend::new(fixture_dirty_repo()),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    runtime.dispatch_ui(UiAction::RefreshAll);
    runtime.dispatch_ui(UiAction::MoveDown);
    runtime.dispatch_ui(UiAction::MoveUp);

    let details_ops = runtime
        .backend()
        .operations()
        .iter()
        .filter(|operation| operation.starts_with("details-diff:"))
        .cloned()
        .collect::<Vec<_>>();

    assert_eq!(
        details_ops,
        vec![
            "details-diff:README.md".to_string(),
            "details-diff:src/lib.rs,src/main.rs".to_string(),
        ]
    );
    assert!(
        runtime
            .render_terminal_text()
            .contains("diff --git a/README.md")
    );
    assert_eq!(
        runtime.state().details.files_targets,
        vec!["README.md".to_string()]
    );
}

#[test]
fn harness_files_details_show_untracked_file_diff() {
    let inputs = [UiAction::RefreshAll];
    assert_scenario(MockScenario::new(
        "files_details_untracked_file_diff",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "### unstaged",
                "diff --git a/README.md b/README.md",
                "+new file README.md",
            ],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["details-diff:README.md"],
            git_state_contains: &["path: \"README.md\"", "untracked: true"],
        },
    ));
}

#[test]
fn harness_files_details_ctrl_scroll_moves_details_content_without_git_change() {
    let size = TerminalSize {
        width: 80,
        height: 14,
    };
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::DetailsScrollDown {
            lines: details_scroll_lines_for_terminal_size(size),
            visible_lines: details_content_lines_for_terminal_size(size),
        },
    ];
    let mut scenario = MockScenario::new(
        "files_details_global_scroll",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["@@ -1 +1 @@", "-old src/lib.rs", "+new src/lib.rs"],
            screen_not_contains: &["### unstaged", "diff --git a/src/lib.rs b/src/lib.rs"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "refresh",
                "details-diff:README.md",
                "details-diff:src/lib.rs,src/main.rs",
            ],
            git_state_contains: &["path: \"src/lib.rs\"", "current_branch: \"main\""],
        },
    );
    scenario.terminal_size = size;
    assert_scenario(scenario);
}

#[test]
fn harness_files_tree_expand_collapse() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::ToggleSelectedDirectory,
    ];
    assert_scenario(MockScenario::new(
        "files_tree_expand_collapse",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["", "src/"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"src/main.rs\""],
        },
    ));
}

#[test]
fn harness_files_space_toggles_directory_stage() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::ToggleSelectedFileStage,
    ];
    assert_scenario(MockScenario::new(
        "files_space_toggles_directory_stage",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Staged src/lib.rs", "### staged"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["stage-files:src/lib.rs"],
            git_state_contains: &["path: \"src/lib.rs\"", "staged: true"],
        },
    ));
}

#[test]
fn harness_files_multi_select_stashes_selected_targets() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::ToggleFilesMultiSelect,
        UiAction::MoveDown,
        UiAction::StashSelectedFiles,
    ];
    assert_scenario(MockScenario::new(
        "files_multi_select_stash",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Stashed 3 files", "savepoint"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["stash-files:savepoint:README.md,src/lib.rs,src/main.rs"],
            git_state_contains: &["summary: \"savepoint\""],
        },
    ));
}

#[test]
fn harness_files_commit_editor_multiline_confirm() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenCommitEditor,
        UiAction::EditorInputChar('f'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar('a'),
        UiAction::EditorInputChar('t'),
        UiAction::EditorInputChar(':'),
        UiAction::EditorInputChar(' '),
        UiAction::EditorInputChar('f'),
        UiAction::EditorInputChar('i'),
        UiAction::EditorInputChar('l'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar('s'),
        UiAction::EditorNextField,
        UiAction::EditorInputChar('l'),
        UiAction::EditorInputChar('i'),
        UiAction::EditorInputChar('n'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar(' '),
        UiAction::EditorInputChar('1'),
        UiAction::EditorInsertNewline,
        UiAction::EditorInputChar('l'),
        UiAction::EditorInputChar('i'),
        UiAction::EditorInputChar('n'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar(' '),
        UiAction::EditorInputChar('2'),
        UiAction::EditorConfirm,
    ];
    assert_scenario(MockScenario::new(
        "files_commit_editor_multiline_confirm",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Commit created:", "feat: files", " Commits"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["commit:feat: files", "refresh"],
            git_state_contains: &["summary: \"feat: files\""],
        },
    ));
}

#[test]
fn harness_files_commit_editor_reports_terminal_cursor() {
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    let mut runtime = Runtime::new(
        AppState::default(),
        MockGitBackend::new(fixture_dirty_repo()),
        size,
    );
    runtime.dispatch_ui(UiAction::RefreshAll);
    runtime.dispatch_ui(UiAction::OpenCommitEditor);
    for ch in "feat".chars() {
        runtime.dispatch_ui(UiAction::EditorInputChar(ch));
    }
    runtime.dispatch_ui(UiAction::EditorNextField);
    for ch in "line 1".chars() {
        runtime.dispatch_ui(UiAction::EditorInputChar(ch));
    }
    runtime.dispatch_ui(UiAction::EditorInsertNewline);
    for ch in "line 2".chars() {
        runtime.dispatch_ui(UiAction::EditorInputChar(ch));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(runtime.state(), size);
    assert_eq!(cursor.expect("editor cursor should render").y, 14);
    assert!(
        runtime
            .backend()
            .operations()
            .contains(&"refresh-files".to_string())
    );
}

#[test]
fn harness_files_stash_editor_all_mode() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenStashEditor,
        UiAction::EditorInputChar('a'),
        UiAction::EditorInputChar('l'),
        UiAction::EditorInputChar('l'),
        UiAction::EditorInputChar(' '),
        UiAction::EditorInputChar('s'),
        UiAction::EditorInputChar('a'),
        UiAction::EditorInputChar('v'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorConfirm,
    ];
    assert_scenario(MockScenario::new(
        "files_stash_editor_all_mode",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Stash pushed: all save"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["stash-push:all save", "refresh"],
            git_state_contains: &["summary: \"all save\""],
        },
    ));
}

#[test]
fn harness_files_stash_editor_multiselect_mode() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::ToggleFilesMultiSelect,
        UiAction::MoveDown,
        UiAction::OpenStashEditor,
        UiAction::EditorInputChar('p'),
        UiAction::EditorInputChar('i'),
        UiAction::EditorInputChar('c'),
        UiAction::EditorInputChar('k'),
        UiAction::EditorInputChar(' '),
        UiAction::EditorInputChar('s'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar('l'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar('c'),
        UiAction::EditorInputChar('t'),
        UiAction::EditorInputChar('i'),
        UiAction::EditorInputChar('o'),
        UiAction::EditorInputChar('n'),
        UiAction::EditorConfirm,
    ];
    assert_scenario(MockScenario::new(
        "files_stash_editor_multiselect_mode",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Stashed 3 files: pick selection"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["stash-files:pick selection:README.md,src/lib.rs,src/main.rs"],
            git_state_contains: &["summary: \"pick selection\""],
        },
    ));
}

#[test]
fn harness_files_v_marks_individual_rows() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::ToggleFilesMultiSelect,
        UiAction::MoveDown,
    ];
    assert_scenario(MockScenario::new(
        "files_v_marks_individual_rows",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["✓   README.md", "✓   src/"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[" src/"],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_files_search_jumps_and_clears() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::StartSearch,
        UiAction::InputSearchChar('l'),
        UiAction::InputSearchChar('i'),
        UiAction::ConfirmSearch,
        UiAction::NextSearchMatch,
        UiAction::PrevSearchMatch,
        UiAction::CancelSearch,
    ];
    assert_scenario(MockScenario::new(
        "files_search_jumps_and_clears",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["src/lib.rs", "keys(files):"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_left_panel_search_selects_branch_and_commit_matches() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::StartSearch,
        UiAction::InputSearchChar('f'),
        UiAction::InputSearchChar('e'),
        UiAction::InputSearchChar('a'),
        UiAction::InputSearchChar('t'),
        UiAction::ConfirmSearch,
        UiAction::FocusNext,
        UiAction::StartSearch,
        UiAction::InputSearchChar('w'),
        UiAction::InputSearchChar('i'),
        UiAction::InputSearchChar('r'),
        UiAction::InputSearchChar('e'),
        UiAction::ConfirmSearch,
    ];
    assert_scenario(MockScenario::new(
        "left_panel_search_selects_branch_and_commit_matches",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["feature/mvp", "def5678", "commit def5678"],
            screen_not_contains: &["/ search"],
            selected_screen_rows: &["def5678"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["branch-log:feature/mvp:50", "commit-diff:def5678"],
            git_state_contains: &["current_branch: \"main\"", "name: \"feature/mvp\""],
        },
    ));
}

#[test]
fn harness_stash_search_selects_match_without_git_operation() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusPanel {
            panel: ratagit_core::PanelFocus::Stash,
        },
        UiAction::StartSearch,
        UiAction::InputSearchChar('w'),
        UiAction::InputSearchChar('i'),
        UiAction::InputSearchChar('p'),
        UiAction::ConfirmSearch,
    ];
    assert_scenario(MockScenario::new(
        "stash_search_selects_match_without_git_operation",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["stash@{0} WIP on main", "keys(stash):"],
            screen_not_contains: &["/ search"],
            selected_screen_rows: &["stash@{0}"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\"", "id: \"stash@{0}\""],
        },
    ));
}

#[test]
fn harness_files_reset_mixed_menu() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenResetMenu,
        UiAction::ConfirmResetMenu,
    ];
    assert_scenario(MockScenario::new(
        "files_reset_mixed_menu",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Reset mixed to HEAD", "keys(files):", "D reset"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["reset:mixed", "refresh"],
            git_state_contains: &[
                "path: \"src/main.rs\"",
                "path: \"README.md\"",
                "staged: false",
            ],
        },
    ));
}

#[test]
fn harness_files_reset_hard_menu() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenResetMenu,
        UiAction::MoveResetMenuDown,
        UiAction::MoveResetMenuDown,
        UiAction::ConfirmResetMenu,
    ];
    assert_scenario(MockScenario::new(
        "files_reset_hard_menu",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Reset hard to HEAD", "README.md"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["reset:hard", "refresh"],
            git_state_contains: &[
                "status_summary: \"staged: 0, unstaged: 1\"",
                "path: \"README.md\"",
                "untracked: true",
            ],
        },
    ));
}

#[test]
fn harness_files_reset_nuke_menu() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenResetMenu,
        UiAction::MoveResetMenuDown,
        UiAction::MoveResetMenuDown,
        UiAction::MoveResetMenuDown,
        UiAction::ConfirmResetMenu,
    ];
    assert_scenario(MockScenario::new(
        "files_reset_nuke_menu",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Nuked working tree", "details(files): no diff"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["nuke", "refresh"],
            git_state_contains: &["status_summary: \"staged: 0, unstaged: 0\"", "files: []"],
        },
    ));
}

#[test]
fn harness_files_discard_current_target_with_confirmation() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenDiscardConfirm,
        UiAction::ConfirmDiscard,
    ];
    assert_scenario(MockScenario::new(
        "files_discard_current_target_confirm",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Discarded README.md", "keys(files):"],
            screen_not_contains: &[" README.md"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["discard-files:README.md", "refresh"],
            git_state_contains: &["path: \"src/lib.rs\"", "path: \"src/main.rs\""],
        },
    ));
}

#[test]
fn harness_files_discard_visual_targets_with_confirmation() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::ToggleFilesMultiSelect,
        UiAction::MoveDown,
        UiAction::OpenDiscardConfirm,
        UiAction::ConfirmDiscard,
    ];
    assert_scenario(MockScenario::new(
        "files_discard_visual_targets_confirm",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Discarded 3 files", "details(files): no diff"],
            screen_not_contains: &[" README.md", " lib.rs", " main.rs"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["discard-files:README.md,src/lib.rs,src/main.rs", "refresh"],
            git_state_contains: &["files: []"],
        },
    ));
}

#[test]
fn harness_files_discard_confirmation_can_cancel() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::OpenDiscardConfirm,
        UiAction::CancelDiscard,
    ];
    assert_scenario(MockScenario::new(
        "files_discard_confirm_cancel",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["README.md", "keys(files):"],
            screen_not_contains: &["Discarded README.md"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"README.md\""],
        },
    ));
}

#[test]
fn harness_files_scroll_keeps_selection_visible() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 20))
        .collect::<Vec<_>>();
    assert_scenario(MockScenario::new(
        "files_scroll_keeps_selection_visible",
        fixture_many_files(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "    file-18.txt",
                "    file-19.txt",
                "    file-20.txt",
                "    file-23.txt",
            ],
            screen_not_contains: &[],
            selected_screen_rows: &[" file-20.txt"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"file-20.txt\""],
        },
    ));
}

#[test]
fn harness_files_reversing_up_does_not_jump_to_top_reserve() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 25))
        .chain(std::iter::repeat_n(UiAction::MoveUp, 5))
        .collect::<Vec<_>>();
    assert_scenario(MockScenario::new(
        "files_reversing_up_does_not_jump_to_top_reserve",
        fixture_many_files(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["    file-17.txt", "    file-20.txt", "    file-22.txt"],
            screen_not_contains: &[],
            selected_screen_rows: &[" file-20.txt"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"file-24.txt\""],
        },
    ));
}

#[test]
fn harness_files_reversing_down_does_not_jump_to_bottom_reserve() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 25))
        .chain(std::iter::repeat_n(UiAction::MoveUp, 5))
        .chain(std::iter::once(UiAction::MoveDown))
        .collect::<Vec<_>>();
    assert_scenario(MockScenario::new(
        "files_reversing_down_does_not_jump_to_bottom_reserve",
        fixture_many_files(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["    file-17.txt", "    file-21.txt", "    file-22.txt"],
            screen_not_contains: &[],
            selected_screen_rows: &[" file-21.txt"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"file-24.txt\""],
        },
    ));
}

#[test]
fn harness_untracked_directory_marker_displays_as_tree_directory() {
    let mut fixture = fixture_empty_repo();
    fixture.files = vec![fixture_file("libs/ratagit-git/tests/", false, true)];
    fixture.status_summary = "staged: 0, unstaged: 1".to_string();

    let inputs = [UiAction::RefreshAll];
    assert_scenario(MockScenario::new(
        "files_untracked_directory_marker_tree_node",
        fixture,
        &inputs,
        ScenarioExpectations {
            screen_contains: &[" tests/"],
            screen_not_contains: &[" libs/ratagit-git/tests/"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"libs/ratagit-git/tests/\""],
        },
    ));
}

#[test]
fn harness_commits_create_and_refresh() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::CreateCommit {
            message: "mvp commit".to_string(),
        },
    ];
    assert_scenario(MockScenario::new(
        "mvp_commits_create_refresh",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["mvp commit", " Commits"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["commit:mvp commit", "refresh"],
            git_state_contains: &["summary: \"mvp commit\""],
        },
    ));
}

#[test]
fn harness_commits_visual_multiselect_marks_rows() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::ToggleCommitsMultiSelect,
        UiAction::MoveDown,
    ];
    assert_scenario(MockScenario::new(
        "commits_visual_multiselect_marks_rows",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["init project", "wire commands"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &["abc1234", "def5678"],
            git_ops_contains: &["refresh"],
            git_state_contains: &["summary: \"init project\""],
        },
    ));
}

#[test]
fn harness_commits_details_follow_cursor_with_commit_diff() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::MoveDown,
    ];
    assert_scenario(MockScenario::new(
        "commits_details_follow_cursor_with_commit_diff",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "commit def5678",
                "Author: ratagit-tests",
                "diff --git a/commit.txt b/commit.txt",
            ],
            screen_not_contains: &["details(commits): pending details implementation"],
            selected_screen_rows: &["def5678"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["commit-diff:abc1234", "commit-diff:def5678"],
            git_state_contains: &["summary: \"wire commands\""],
        },
    ));
}

#[test]
fn harness_commits_details_renders_truncated_commit_diff_notice() {
    let fixture = clean_three_commit_fixture();
    let mut overrides = BTreeMap::new();
    overrides.insert(
        "abc1234".to_string(),
        "commit abc1234\nAuthor: ratagit-tests <ratagit-tests@example.com>\n\n    large patch\n\ndiff --git a/large.txt b/large.txt\n@@ -0,0 +1 @@\n+partial\n\n### commit diff truncated at 1048576 bytes\n"
            .to_string(),
    );
    let mut runtime = Runtime::new(
        AppState::default(),
        MockGitBackend::with_commit_diff_overrides(fixture, overrides),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    runtime.dispatch_ui(UiAction::RefreshAll);
    runtime.dispatch_ui(UiAction::FocusNext);
    runtime.dispatch_ui(UiAction::FocusNext);

    let screen = runtime.render_terminal_text();
    let operations = runtime.backend().operations().join("\n");
    let git_state = format!("{:#?}", runtime.backend().snapshot());
    assert!(screen.contains("### commit diff truncated at 1048576 bytes"));
    assert!(screen.contains("diff --git a/large.txt b/large.txt"));
    assert!(operations.contains("commit-diff:abc1234"));
    assert!(git_state.contains("summary: \"init project\""));
}

#[test]
fn harness_commits_enter_files_subpanel_and_follow_file_cursor() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::OpenCommitFilesPanel,
        UiAction::MoveDown,
        UiAction::MoveDown,
    ];
    assert_scenario(MockScenario::new(
        "commits_files_subpanel_follow_file_cursor",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "Commit Files",
                "A lib.rs",
                "diff --git a/src/lib.rs b/src/lib.rs",
            ],
            screen_not_contains: &["details(commits): pending details implementation"],
            selected_screen_rows: &["A lib.rs"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "commit-files:abc1234",
                "commit-file-diff:abc1234:README.md",
                "commit-file-diff:abc1234:src/lib.rs",
            ],
            git_state_contains: &["summary: \"init project\""],
        },
    ));
}

#[test]
fn harness_commit_files_search_selects_file_and_refreshes_diff() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::OpenCommitFilesPanel,
        UiAction::StartSearch,
        UiAction::InputSearchChar('l'),
        UiAction::InputSearchChar('i'),
        UiAction::InputSearchChar('b'),
        UiAction::ConfirmSearch,
    ];
    assert_scenario(MockScenario::new(
        "commit_files_search_selects_file_and_refreshes_diff",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "Commit Files",
                "A lib.rs",
                "diff --git a/src/lib.rs b/src/lib.rs",
            ],
            screen_not_contains: &["/ search"],
            selected_screen_rows: &["A lib.rs"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "commit-files:abc1234",
                "commit-file-diff:abc1234:README.md",
                "commit-file-diff:abc1234:src/lib.rs",
            ],
            git_state_contains: &["summary: \"init project\""],
        },
    ));
}

#[test]
fn harness_commits_files_directory_uses_directory_pathspec() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::OpenCommitFilesPanel,
        UiAction::MoveDown,
        UiAction::ToggleCommitFilesDirectory,
    ];
    assert_scenario(MockScenario::new(
        "commits_files_directory_diff",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Commit Files", " src/", "diff --git a/src b/src"],
            screen_not_contains: &["A lib.rs"],
            selected_screen_rows: &[" src/"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "commit-files:abc1234",
                "commit-file-diff:abc1234:README.md",
                "commit-file-diff:abc1234:src",
            ],
            git_state_contains: &["summary: \"init project\""],
        },
    ));
}

#[test]
fn harness_commits_lazy_loads_next_page_when_scrolling_past_first_hundred() {
    let mut inputs = vec![
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
    ];
    inputs.extend((0..100).map(|_| UiAction::MoveDown));

    assert_scenario(MockScenario::new(
        "commits_lazy_load_next_page",
        clean_many_commit_fixture(125),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["commit 100"],
            screen_not_contains: &[],
            selected_screen_rows: &["commit 100"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh", "commits-page:100:100"],
            git_state_contains: &["summary: \"commit 124\""],
        },
    ));
}

#[test]
fn harness_commits_prefetches_next_page_before_tail() {
    let mut inputs = vec![
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
    ];
    inputs.extend((0..79).map(|_| UiAction::MoveDown));

    assert_scenario(MockScenario::new(
        "commits_prefetch_next_page_before_tail",
        clean_many_commit_fixture(125),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["commit 79"],
            screen_not_contains: &[],
            selected_screen_rows: &["commit 79"],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh", "commits-page:100:100"],
            git_state_contains: &["summary: \"commit 124\""],
        },
    ));
}

#[test]
fn harness_commits_squash_multiselect() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::ToggleCommitsMultiSelect,
        UiAction::MoveDown,
        UiAction::SquashSelectedCommits,
    ];
    assert_scenario(MockScenario::new(
        "commits_squash_multiselect",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Squashed 2 commits", "older base"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["squash:abc1234,def5678", "refresh"],
            git_state_contains: &["summary: \"older base + wire commands + init project\""],
        },
    ));
}

#[test]
fn harness_commits_fixup_selected() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::FixupSelectedCommits,
    ];
    assert_scenario(MockScenario::new(
        "commits_fixup_selected",
        clean_three_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Fixed up 1 commit", "wire commands"],
            screen_not_contains: &["init project"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["fixup:abc1234", "refresh"],
            git_state_contains: &["summary: \"wire commands\""],
        },
    ));
}

#[test]
fn harness_commits_reword_selected() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::OpenCommitRewordEditor,
        UiAction::EditorMoveCursorEnd,
        UiAction::EditorInputChar(' '),
        UiAction::EditorInputChar('u'),
        UiAction::EditorInputChar('p'),
        UiAction::EditorInputChar('d'),
        UiAction::EditorInputChar('a'),
        UiAction::EditorInputChar('t'),
        UiAction::EditorInputChar('e'),
        UiAction::EditorInputChar('d'),
        UiAction::EditorConfirm,
    ];
    assert_scenario(MockScenario::new(
        "commits_reword_selected",
        clean_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Reworded abc1234", "init project updated"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["reword:abc1234:init project updated", "refresh"],
            git_state_contains: &["summary: \"init project updated\""],
        },
    ));
}

#[test]
fn harness_commits_delete_selected() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::DeleteSelectedCommits,
    ];
    assert_scenario(MockScenario::new(
        "commits_delete_selected",
        clean_commit_fixture(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Deleted 1 commit", "wire commands"],
            screen_not_contains: &["init project"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["delete-commits:abc1234", "refresh"],
            git_state_contains: &["summary: \"wire commands\""],
        },
    ));
}

#[test]
fn harness_commits_detached_checkout_uses_auto_stash() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::CheckoutSelectedCommitDetached,
        UiAction::ConfirmAutoStash,
    ];
    assert_scenario(MockScenario::new(
        "commits_detached_checkout_auto_stash",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Checked out detached with auto-stash", "abc1234"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "auto-stash-push",
                "checkout-detached:abc1234",
                "auto-stash-pop",
                "refresh",
            ],
            git_state_contains: &["detached_head: true", "current_branch: \"abc1234\""],
        },
    ));
}

#[test]
fn harness_branches_create_and_checkout() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::CreateBranch {
            name: "feature/new".to_string(),
            start_point: "main".to_string(),
        },
        UiAction::MoveDown,
        UiAction::MoveDown,
        UiAction::CheckoutSelectedBranch,
        UiAction::ConfirmAutoStash,
    ];
    assert_scenario(MockScenario::new(
        "mvp_branches_create_checkout",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["feature/new", "commit abc1234"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "branch-log:main:50",
                "create-branch:feature/new:main",
                "branch-log:feature/new:50",
                "auto-stash-push",
                "checkout-branch:feature/new",
                "auto-stash-pop",
            ],
            git_state_contains: &["current_branch: \"feature/new\""],
        },
    ));
}

#[test]
fn harness_branch_details_follow_cursor_with_log_graph() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::MoveDown,
    ];
    assert_scenario(MockScenario::new(
        "branches_details_follow_cursor_log_graph",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "* commit abc1234",
                "init project on feature/mvp",
                "keys(branches):",
            ],
            screen_not_contains: &["details(branches): pending"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["branch-log:main:50", "branch-log:feature/mvp:50"],
            git_state_contains: &["current_branch: \"main\"", "name: \"feature/mvp\""],
        },
    ));
}

#[test]
fn harness_branches_create_from_selected_branch() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::MoveDown,
        UiAction::OpenBranchCreateInput,
        UiAction::BranchCreateInputChar('f'),
        UiAction::BranchCreateInputChar('e'),
        UiAction::BranchCreateInputChar('a'),
        UiAction::BranchCreateInputChar('t'),
        UiAction::BranchCreateInputChar('u'),
        UiAction::BranchCreateInputChar('r'),
        UiAction::BranchCreateInputChar('e'),
        UiAction::BranchCreateInputChar('/'),
        UiAction::BranchCreateInputChar('f'),
        UiAction::BranchCreateInputChar('r'),
        UiAction::BranchCreateInputChar('o'),
        UiAction::BranchCreateInputChar('m'),
        UiAction::BranchCreateInputChar('-'),
        UiAction::BranchCreateInputChar('m'),
        UiAction::BranchCreateInputChar('v'),
        UiAction::BranchCreateInputChar('p'),
        UiAction::ConfirmBranchCreate,
    ];
    assert_scenario(MockScenario::new(
        "branches_create_from_selected_branch",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["feature/from-mvp", "Branch created"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["create-branch:feature/from-mvp:feature/mvp"],
            git_state_contains: &["name: \"feature/from-mvp\""],
        },
    ));
}

#[test]
fn harness_branches_dirty_checkout_with_auto_stash_confirm() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::MoveDown,
        UiAction::CheckoutSelectedBranch,
        UiAction::ConfirmAutoStash,
    ];
    assert_scenario(MockScenario::new(
        "branches_dirty_checkout_auto_stash",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Checked out with auto-stash: feature/mvp"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &[
                "auto-stash-push",
                "checkout-branch:feature/mvp",
                "auto-stash-pop",
            ],
            git_state_contains: &["current_branch: \"feature/mvp\""],
        },
    ));
}

#[test]
fn harness_branches_delete_local_branch() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::MoveDown,
        UiAction::OpenBranchDeleteMenu,
        UiAction::ConfirmBranchDeleteMenu,
    ];
    assert_scenario(MockScenario::new(
        "branches_delete_local",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Deleted local branch: feature/mvp"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["delete-local:feature/mvp"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_branches_delete_current_branch_is_protected() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::OpenBranchDeleteMenu,
        UiAction::ConfirmBranchDeleteMenu,
    ];
    assert_scenario(MockScenario::new(
        "branches_delete_current_protected",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Cannot delete current branch"],
            screen_not_contains: &["Deleted local branch: main"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\"", "name: \"main\""],
        },
    ));
}

#[test]
fn harness_branches_rebase_simple_and_origin_main() {
    let mut fixture = fixture_dirty_repo();
    fixture.files.clear();
    fixture.status_summary = "staged: 0, unstaged: 0".to_string();

    let simple_inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::MoveDown,
        UiAction::OpenBranchRebaseMenu,
        UiAction::ConfirmBranchRebaseMenu,
    ];
    assert_scenario(MockScenario::new(
        "branches_rebase_simple",
        fixture.clone(),
        &simple_inputs,
        ScenarioExpectations {
            screen_contains: &["Rebased (simple) onto feature/mvp"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["rebase:simple:feature/mvp"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));

    let origin_main_inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::MoveDown,
        UiAction::OpenBranchRebaseMenu,
        UiAction::MoveBranchRebaseMenuDown,
        UiAction::MoveBranchRebaseMenuDown,
        UiAction::ConfirmBranchRebaseMenu,
    ];
    assert_scenario(MockScenario::new(
        "branches_rebase_origin_main",
        fixture,
        &origin_main_inputs,
        ScenarioExpectations {
            screen_contains: &["Rebased (simple) onto origin/main"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["rebase:simple:origin/main"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_stash_push_and_pop() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusPrev,
        UiAction::StashPush {
            message: "savepoint".to_string(),
        },
        UiAction::StashPopSelected,
    ];
    assert_scenario(MockScenario::new(
        "mvp_stash_push_pop",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Stash", "stash@{0} WIP on main"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["stash-push:savepoint", "stash-pop:stash@{0}"],
            git_state_contains: &["summary: \"WIP on main: local test\""],
        },
    ));
}

#[test]
fn harness_error_visible_without_crash() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::CreateCommit {
            message: String::new(),
        },
    ];
    assert_scenario(MockScenario::new(
        "mvp_error_visible_non_crash",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["error=Failed to create commit"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["commit:"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_focus_panel_shortcuts_follow_focus() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusPanel {
            panel: ratagit_core::PanelFocus::Details,
        },
        UiAction::FocusPanel {
            panel: ratagit_core::PanelFocus::Log,
        },
        UiAction::FocusPanel {
            panel: ratagit_core::PanelFocus::Branches,
        },
    ];
    assert_scenario(MockScenario::new(
        "mvp_focus_panel_shortcuts_follow_focus",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Details", "Log", "keys(branches):", "space checkout"],
            screen_not_contains: &[],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_panel_titles_are_numbered_and_empty_placeholders_hidden() {
    let inputs = [UiAction::RefreshAll];
    assert_scenario(MockScenario::new(
        "ui_numbered_titles_no_empty_placeholders",
        fixture_empty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "[1] 󰈙 Files",
                "[2]  Branches",
                "[3]  Commits",
                "[4]  Stash",
                "[5]  Details",
                "[6] 󰌱 Log",
            ],
            screen_not_contains: &["<empty>", "<none>", "error=<none>"],
            selected_screen_rows: &[],
            batch_selected_screen_rows: &[],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}
