use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ratagit_core::{AppContext, PanelFocus, UiAction};
use ratagit_git::HybridGitBackend;
use ratagit_harness::Runtime;
use ratagit_ui::TerminalSize;

struct TmpGitRepo {
    path: PathBuf,
}

impl TmpGitRepo {
    fn new(case_name: &str) -> Self {
        let root = workspace_tmp_root().join("git-tests");
        create_dir_all(&root).expect("tmp git-tests root should be creatable");
        let unique = format!(
            "{case_name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        );
        let path = root.join(unique);
        create_dir_all(&path).expect("test repo directory should be creatable");
        let repo = Self { path };
        repo.run_git(&["init"]);
        repo.run_git(&["config", "user.name", "ratagit-tests"]);
        repo.run_git(&["config", "user.email", "ratagit-tests@example.com"]);
        repo.run_git(&["config", "core.autocrlf", "false"]);
        repo.run_git(&["config", "commit.gpgsign", "false"]);
        repo
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn run_git(&self, args: &[&str]) {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("git command should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn run_git_capture(&self, args: &[&str]) -> String {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .expect("git command should run");
        assert!(
            output.status.success(),
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).to_string()
    }
}

impl Drop for TmpGitRepo {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.path);
    }
}

fn workspace_tmp_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(|libs| libs.parent())
        .map(|workspace| workspace.join("tmp"))
        .expect("workspace root should be discoverable from manifest dir")
}

fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn seeded_repo(case_name: &str) -> TmpGitRepo {
    let repo = TmpGitRepo::new(case_name);
    write(repo.path().join("a.txt"), "a1\n").expect("a.txt should be writable");
    write(repo.path().join("b.txt"), "b1\n").expect("b.txt should be writable");
    repo.run_git(&["add", "--", "a.txt", "b.txt"]);
    repo.run_git(&["commit", "-m", "init"]);
    repo.run_git(&["branch", "-M", "main"]);
    repo
}

fn runtime_for(repo: &TmpGitRepo) -> Runtime<HybridGitBackend> {
    Runtime::new(
        AppContext::default(),
        HybridGitBackend::open(repo.path()).expect("hybrid backend should open"),
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
}

fn dispatch_all(runtime: &mut Runtime<HybridGitBackend>, actions: &[UiAction]) {
    for action in actions {
        runtime.dispatch_ui(action.clone());
    }
}

struct TracedCase<'a> {
    runtime: &'a mut Runtime<HybridGitBackend>,
    actions: Vec<UiAction>,
}

impl std::ops::Deref for TracedCase<'_> {
    type Target = Runtime<HybridGitBackend>;
    fn deref(&self) -> &Self::Target {
        self.runtime
    }
}

impl TracedCase<'_> {
    fn dispatch_ui(&mut self, action: UiAction) {
        self.actions.push(action.clone());
        self.runtime.dispatch_ui(action);
    }
    fn dispatch_all(&mut self, actions: &[UiAction]) {
        for action in actions {
            self.dispatch_ui(action.clone());
        }
    }
}

fn with_case_artifacts(
    repo: &TmpGitRepo,
    runtime: &mut Runtime<HybridGitBackend>,
    check: impl FnOnce(&mut TracedCase<'_>),
) {
    let mut case = TracedCase {
        runtime,
        actions: Vec::new(),
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check(&mut case)));
    if let Err(payload) = outcome {
        let name = std::thread::current()
            .name()
            .unwrap_or("real-git-case")
            .replace(':', "_");
        let base = workspace_tmp_root().join("harness-artifacts").join(name);
        let _ = create_dir_all(&base);
        let _ = write(base.join("screen.txt"), case.render_terminal_text());
        let _ = write(base.join("app_context.txt"), format!("{:#?}", case.state()));
        let _ = write(
            base.join("input_sequence.txt"),
            format!("{:#?}", case.actions),
        );
        let git_state = Command::new("git")
            .args(["status", "--porcelain=v1"])
            .current_dir(repo.path())
            .output()
            .map(|output| output.stdout)
            .unwrap_or_default();
        let _ = write(base.join("git_state.txt"), git_state);
        eprintln!("real Git scenario artifacts: {}", base.display());
        std::panic::resume_unwind(payload);
    }
}

#[test]
fn real_git_smoke_stage_and_unstage_selected_file() {
    if !git_available() {
        eprintln!("git is unavailable, skipping real_git_smoke_stage_and_unstage_selected_file");
        return;
    }

    let repo = seeded_repo("harness-real-stage-unstage");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    let mut runtime = runtime_for(&repo);

    dispatch_all(
        &mut runtime,
        &[UiAction::RefreshAll, UiAction::StageSelectedFile],
    );

    let staged_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(staged_status.lines().any(|line| line == "M  a.txt"));
    assert_eq!(runtime.state().last_operation.as_deref(), Some("stage"));

    runtime.dispatch_ui(UiAction::UnstageSelectedFile);

    let unstaged_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(unstaged_status.lines().any(|line| line == " M a.txt"));
    assert_eq!(runtime.state().last_operation.as_deref(), Some("unstage"));
}

#[test]
fn real_git_smoke_stash_push_and_pop_including_untracked() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping real_git_smoke_stash_push_and_pop_including_untracked"
        );
        return;
    }

    let repo = seeded_repo("harness-real-stash");
    write(repo.path().join("a.txt"), "dirty\n").expect("a.txt should be writable");
    write(repo.path().join("new.txt"), "new\n").expect("new.txt should be writable");
    let mut runtime = runtime_for(&repo);

    dispatch_all(
        &mut runtime,
        &[
            UiAction::RefreshAll,
            UiAction::StashPush {
                message: "real smoke stash".to_string(),
            },
        ],
    );

    let clean_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert_eq!(clean_status.trim(), "");
    assert!(
        runtime
            .state()
            .repo
            .stash
            .items
            .iter()
            .any(|stash| stash.summary.contains("real smoke stash"))
    );

    runtime.dispatch_ui(UiAction::StashPopSelected);

    let dirty_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(dirty_status.lines().any(|line| line == " M a.txt"));
    assert!(dirty_status.lines().any(|line| line == "?? new.txt"));
    assert!(repo.run_git_capture(&["stash", "list"]).trim().is_empty());
    assert_eq!(runtime.state().last_operation.as_deref(), Some("stash_pop"));
}

#[test]
fn real_git_smoke_create_multiline_commit_from_selected_file() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping real_git_smoke_create_multiline_commit_from_selected_file"
        );
        return;
    }

    let repo = seeded_repo("harness-real-commit");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    let mut runtime = runtime_for(&repo);
    let message = "feat: real smoke commit\n\nbody line 1\nbody line 2";

    dispatch_all(
        &mut runtime,
        &[
            UiAction::RefreshAll,
            UiAction::StageSelectedFile,
            UiAction::CreateCommit {
                message: message.to_string(),
            },
        ],
    );

    let latest_message = repo.run_git_capture(&["log", "-1", "--pretty=%B"]);
    assert_eq!(latest_message.trim_end(), message);
    assert_eq!(runtime.state().last_operation.as_deref(), Some("commit"));
    assert_eq!(
        repo.run_git_capture(&["status", "--short", "--untracked-files=all"])
            .trim(),
        ""
    );
}

#[test]
fn real_git_smoke_amend_staged_changes_into_selected_commit() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping real_git_smoke_amend_staged_changes_into_selected_commit"
        );
        return;
    }

    let repo = seeded_repo("harness-real-amend-selected");
    repo.run_git(&["checkout", "-b", "feature/amend"]);
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "middle"]);
    write(repo.path().join("b.txt"), "b2\n").expect("b.txt should be writable");
    repo.run_git(&["add", "--", "b.txt"]);
    repo.run_git(&["commit", "-m", "head"]);
    write(repo.path().join("a.txt"), "a3\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    let mut runtime = runtime_for(&repo);

    dispatch_all(
        &mut runtime,
        &[
            UiAction::RefreshAll,
            UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            },
            UiAction::MoveDown,
            UiAction::AmendStagedChanges,
        ],
    );

    let subjects = repo.run_git_capture(&["log", "--format=%s"]);
    assert_eq!(
        subjects.lines().collect::<Vec<_>>(),
        vec!["head", "middle", "init"]
    );
    assert_eq!(repo.run_git_capture(&["show", "HEAD~1:a.txt"]), "a3\n");
    assert_eq!(
        repo.run_git_capture(&["status", "--short", "--untracked-files=all"])
            .trim(),
        ""
    );
    assert_eq!(runtime.state().last_operation.as_deref(), Some("amend"));
}

#[test]
fn real_git_smoke_dirty_branch_checkout_uses_auto_stash() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping real_git_smoke_dirty_branch_checkout_uses_auto_stash"
        );
        return;
    }

    let repo = seeded_repo("harness-real-checkout-auto-stash");
    repo.run_git(&["branch", "feature/target"]);
    write(repo.path().join("a.txt"), "dirty\n").expect("a.txt should be writable");
    let mut runtime = runtime_for(&repo);

    dispatch_all(
        &mut runtime,
        &[
            UiAction::RefreshAll,
            UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            },
            UiAction::CheckoutSelectedBranch,
        ],
    );
    assert!(runtime.state().ui.branches.auto_stash_confirm.active);

    runtime.dispatch_ui(UiAction::ConfirmAutoStash);

    assert_eq!(
        repo.run_git_capture(&["branch", "--show-current"]).trim(),
        "feature/target"
    );
    let dirty_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(dirty_status.lines().any(|line| line == " M a.txt"));
    assert!(repo.run_git_capture(&["stash", "list"]).trim().is_empty());
    assert_eq!(
        runtime.state().last_operation.as_deref(),
        Some("checkout_branch")
    );
}
#[path = "real_git/native_diff.rs"]
mod native_diff;
#[test]
fn test_real_git_stage_all_ignore_and_stash_confirmation() {
    let repo = seeded_repo("harness-file-extras");
    write(repo.path().join("a.txt"), "changed\n").unwrap();
    write(repo.path().join("z.log"), "noise\n").unwrap();
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_all(&[UiAction::RefreshAll, UiAction::ToggleStageAll]);
        assert!(
            repo.run_git_capture(&["status", "--short"])
                .contains("M  a.txt")
        );
        assert!(runtime.render_terminal_text().contains("Staged 2 files"));
        runtime.dispatch_ui(UiAction::ToggleStageAll);
        assert!(repo.run_git_capture(&["diff", "--cached"]).is_empty());
        runtime.dispatch_ui(UiAction::MoveDown);
        runtime.dispatch_ui(UiAction::IgnoreSelectedFile);
        assert!(repo.path().join(".gitignore").exists());
        assert!(
            !repo
                .run_git_capture(&["status", "--short", "--untracked-files=all"])
                .contains("z.log")
        );
        assert!(runtime.render_terminal_text().contains("Ignore"));
        runtime.dispatch_ui(UiAction::StashPush {
            message: "saved".into(),
        });
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Stash,
        });
        assert!(
            runtime
                .render_terminal_text()
                .contains("diff --git a/a.txt b/a.txt")
        );
        assert!(runtime.render_terminal_text().contains("changed"));
        runtime.dispatch_ui(UiAction::StashApplySelected);
        assert!(repo.run_git_capture(&["stash", "list"]).contains("saved"));
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "changed\n"
        );
        runtime.dispatch_ui(UiAction::StashDropSelected);
        assert!(
            runtime
                .render_terminal_text()
                .contains("Drop selected stash?")
        );
        assert!(!repo.run_git_capture(&["stash", "list"]).is_empty());
        runtime.dispatch_ui(UiAction::ConfirmRepositoryOperation);
        assert!(repo.run_git_capture(&["stash", "list"]).is_empty());
        assert!(runtime.render_terminal_text().contains("StashDrop"));
    });
}
#[test]
fn test_real_git_case_failure_produces_action_and_state_artifacts() {
    let repo = seeded_repo("harness-artifact-check");
    let mut runtime = runtime_for(&repo);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        with_case_artifacts(&repo, &mut runtime, |case| {
            case.dispatch_ui(UiAction::RefreshAll);
            panic!("intentional artifact verification");
        });
    }));
    assert!(outcome.is_err());
    let base = workspace_tmp_root()
        .join("harness-artifacts")
        .join("test_real_git_case_failure_produces_action_and_state_artifacts");
    assert!(
        std::fs::read_to_string(base.join("screen.txt"))
            .unwrap()
            .contains("Files")
    );
    assert!(
        std::fs::read_to_string(base.join("input_sequence.txt"))
            .unwrap()
            .contains("RefreshAll")
    );
    assert!(base.join("app_context.txt").exists());
    assert!(base.join("git_state.txt").exists());
}
