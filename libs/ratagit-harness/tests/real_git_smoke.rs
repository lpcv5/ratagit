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
