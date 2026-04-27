use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ratagit_core::{BranchDeleteMode, CommitFileDiffPath, CommitFileDiffTarget, ResetMode};
use ratagit_git::{GitBackendHistoryRewrite, GitBackendRead, GitBackendWrite, HybridGitBackend};

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

fn seeded_repo_with_two_files(case_name: &str) -> TmpGitRepo {
    let repo = TmpGitRepo::new(case_name);
    write(repo.path().join("a.txt"), "a1\n").expect("a.txt should be writable");
    write(repo.path().join("b.txt"), "b1\n").expect("b.txt should be writable");
    repo.run_git(&["add", "--", "a.txt", "b.txt"]);
    repo.run_git(&["commit", "-m", "init"]);
    repo
}

#[test]
fn cli_create_commit_supports_multiline_message() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_create_commit_supports_multiline_message");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-create-commit");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let message = "feat: multiline subject\n\nline 1\nline 2";
    backend
        .create_commit(message)
        .expect("create_commit should succeed");

    let actual = repo.run_git_capture(&["log", "-1", "--pretty=%B"]);
    assert_eq!(actual.trim_end(), message);
}

#[test]
fn cli_branch_details_log_returns_colored_graph_limited_by_count() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_branch_details_log_returns_colored_graph_limited_by_count"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-branch-details-log");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "second"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let graph = backend
        .branch_details_log("HEAD", 1)
        .expect("branch details log should succeed");

    assert!(graph.contains("\u{1b}["));
    assert!(graph.contains("*"));
    assert!(graph.contains("second"));
    assert!(!graph.contains("init"));
}

#[test]
fn cli_stash_push_uses_title_for_all_changes() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_stash_push_uses_title_for_all_changes");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-stash-push");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .stash_push("all stash title")
        .expect("stash_push should succeed");

    let stash_list = repo.run_git_capture(&["stash", "list"]);
    assert!(stash_list.contains("all stash title"));
}

#[test]
fn cli_stash_push_includes_untracked_files() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_stash_push_includes_untracked_files");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-stash-push-untracked");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("new.txt"), "new\n").expect("new.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .stash_push("all stash with untracked")
        .expect("stash_push should succeed");

    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(!status.contains("a.txt"));
    assert!(!status.contains("new.txt"));

    let stash_files = repo.run_git_capture(&[
        "stash",
        "show",
        "--include-untracked",
        "--name-only",
        "stash@{0}",
    ]);
    assert!(stash_files.lines().any(|line| line == "new.txt"));
}

#[test]
fn cli_stash_push_with_blank_message_uses_git_default_message() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_stash_push_with_blank_message_uses_git_default_message"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-stash-push-blank");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .stash_push("   ")
        .expect("blank stash_push should succeed");

    let status = repo.run_git_capture(&["status", "--short"]);
    assert_eq!(status.trim(), "");
    let stash_list = repo.run_git_capture(&["stash", "list"]);
    assert!(stash_list.contains("WIP on"));
}

#[test]
fn cli_stash_files_limits_stash_to_selected_paths() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_stash_files_limits_stash_to_selected_paths");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-stash-files");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("b.txt"), "b2\n").expect("b.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .stash_files("selected stash", &["a.txt".to_string()])
        .expect("stash_files should succeed");

    let status = repo.run_git_capture(&["status", "--short"]);
    assert!(!status.contains("a.txt"));
    assert!(status.contains(" b.txt"));

    let stash_list = repo.run_git_capture(&["stash", "list"]);
    assert!(stash_list.contains("selected stash"));
}

#[test]
fn cli_reset_hard_clears_tracked_changes_but_keeps_untracked() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_reset_hard_clears_tracked_changes_but_keeps_untracked"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-reset-hard");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("new.txt"), "new\n").expect("new.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .reset(ResetMode::Hard)
        .expect("hard reset should succeed");

    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(!status.contains("a.txt"));
    assert!(status.contains("?? new.txt"));
}

#[test]
fn cli_reset_soft_preserves_index_and_mixed_unstages_changes() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_reset_soft_preserves_index_and_mixed_unstages_changes"
        );
        return;
    }

    let soft_repo = seeded_repo_with_two_files("cli-reset-soft");
    write(soft_repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    soft_repo.run_git(&["add", "--", "a.txt"]);
    let mut soft_backend =
        HybridGitBackend::open(soft_repo.path()).expect("hybrid backend should open");
    soft_backend
        .reset(ResetMode::Soft)
        .expect("soft reset should succeed");
    let soft_status = soft_repo.run_git_capture(&["status", "--short"]);
    assert!(soft_status.lines().any(|line| line == "M  a.txt"));

    let mixed_repo = seeded_repo_with_two_files("cli-reset-mixed");
    write(mixed_repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    mixed_repo.run_git(&["add", "--", "a.txt"]);
    let mut mixed_backend =
        HybridGitBackend::open(mixed_repo.path()).expect("hybrid backend should open");
    mixed_backend
        .reset(ResetMode::Mixed)
        .expect("mixed reset should succeed");
    let mixed_status = mixed_repo.run_git_capture(&["status", "--short"]);
    assert!(mixed_status.lines().any(|line| line == " M a.txt"));
}

#[test]
fn cli_nuke_clears_tracked_and_untracked_changes() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_nuke_clears_tracked_and_untracked_changes");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-nuke");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("new.txt"), "new\n").expect("new.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend.nuke().expect("nuke should succeed");

    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert_eq!(status.trim(), "");
}

#[test]
fn cli_discard_files_restores_tracked_and_removes_untracked_targets() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_discard_files_restores_tracked_and_removes_untracked_targets"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-discard-files");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("new.txt"), "new\n").expect("new.txt should be writable");
    write(repo.path().join("keep.txt"), "keep\n").expect("keep.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .discard_files(&["a.txt".to_string(), "new.txt".to_string()])
        .expect("discard_files should succeed");

    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(!status.contains("a.txt"));
    assert!(!status.contains("new.txt"));
    assert!(status.contains("?? keep.txt"));
}

#[test]
fn cli_checkout_branch_auto_stash_restores_dirty_worktree_on_target_branch() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_checkout_branch_auto_stash_restores_dirty_worktree_on_target_branch"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-checkout-auto-stash");
    repo.run_git(&["branch", "feature/target"]);
    write(repo.path().join("a.txt"), "dirty\n").expect("a.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .checkout_branch("feature/target", true)
        .expect("auto-stash checkout should succeed");

    let current_branch = repo.run_git_capture(&["branch", "--show-current"]);
    assert_eq!(current_branch.trim(), "feature/target");
    let status = repo.run_git_capture(&["status", "--short"]);
    assert!(status.lines().any(|line| line == " M a.txt"));
    assert!(repo.run_git_capture(&["stash", "list"]).trim().is_empty());
}

#[test]
fn cli_rebase_branch_auto_stash_restores_dirty_worktree_after_rebase() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_rebase_branch_auto_stash_restores_dirty_worktree_after_rebase"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-rebase-auto-stash");
    let default_branch = repo.run_git_capture(&["branch", "--show-current"]);
    let default_branch = default_branch.trim();
    repo.run_git(&["checkout", "-b", "feature/rebase"]);
    write(repo.path().join("a.txt"), "feature\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "feature change"]);
    repo.run_git(&["checkout", default_branch]);
    write(repo.path().join("b.txt"), "main\n").expect("b.txt should be writable");
    repo.run_git(&["add", "--", "b.txt"]);
    repo.run_git(&["commit", "-m", "main change"]);
    repo.run_git(&["checkout", "feature/rebase"]);
    write(repo.path().join("dirty.txt"), "dirty\n").expect("dirty.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .rebase_branch(default_branch, false, true)
        .expect("auto-stash rebase should succeed");

    let log = repo.run_git_capture(&["log", "--oneline", "-2"]);
    assert!(log.contains("feature change"));
    assert!(log.contains("main change"));
    let status = repo.run_git_capture(&["status", "--short"]);
    assert!(status.lines().any(|line| line == "?? dirty.txt"));
    assert!(repo.run_git_capture(&["stash", "list"]).trim().is_empty());
}

#[test]
fn cli_commit_file_diff_for_rename_includes_old_and_new_paths() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_commit_file_diff_for_rename_includes_old_and_new_paths"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-commit-file-diff-rename");
    repo.run_git(&["mv", "a.txt", "renamed.txt"]);
    write(repo.path().join("renamed.txt"), "a1\na2\n").expect("renamed.txt should be writable");
    repo.run_git(&["add", "--", "renamed.txt"]);
    repo.run_git(&["commit", "-m", "rename a"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let files = backend
        .commit_files("HEAD")
        .expect("commit files should include rename");
    let renamed = files
        .iter()
        .find(|entry| entry.path == "renamed.txt")
        .expect("renamed file should be listed")
        .clone();
    assert_eq!(renamed.old_path.as_deref(), Some("a.txt"));

    let diff = backend
        .commit_file_diff(&CommitFileDiffTarget {
            commit_id: "HEAD".to_string(),
            paths: vec![CommitFileDiffPath {
                path: renamed.path,
                old_path: renamed.old_path,
            }],
        })
        .expect("commit file diff should succeed");

    assert!(diff.contains("diff --git a/a.txt b/renamed.txt"));
    assert!(diff.contains("--- a/a.txt"));
    assert!(diff.contains("+++ b/renamed.txt"));
}

#[test]
fn cli_commit_file_diff_accepts_directory_pathspec() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_commit_file_diff_accepts_directory_pathspec");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-commit-file-diff-directory");
    create_dir_all(repo.path().join("src")).expect("src dir should be creatable");
    write(repo.path().join("src").join("a.rs"), "fn a() {}\n").expect("a.rs should be writable");
    write(repo.path().join("src").join("b.rs"), "fn b() {}\n").expect("b.rs should be writable");
    repo.run_git(&["add", "--", "src"]);
    repo.run_git(&["commit", "-m", "add src files"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let diff = backend
        .commit_file_diff(&CommitFileDiffTarget {
            commit_id: "HEAD".to_string(),
            paths: vec![CommitFileDiffPath {
                path: "src".to_string(),
                old_path: None,
            }],
        })
        .expect("directory commit file diff should succeed");

    assert!(diff.contains("diff --git a/src/a.rs b/src/a.rs"));
    assert!(diff.contains("diff --git a/src/b.rs b/src/b.rs"));
}

#[test]
fn cli_delete_branch_reports_worktree_occupancy() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_delete_branch_reports_worktree_occupancy");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-delete-worktree-branch");
    repo.run_git(&["branch", "feature/worktree"]);
    let worktree_path = repo.path().with_file_name(format!(
        "{}-linked",
        repo.path()
            .file_name()
            .expect("repo path should have a file name")
            .to_string_lossy()
    ));
    let worktree_arg = worktree_path.to_string_lossy().to_string();
    repo.run_git(&["worktree", "add", &worktree_arg, "feature/worktree"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let error = backend
        .delete_branch("feature/worktree", BranchDeleteMode::Local, false)
        .expect_err("worktree-occupied branch delete should fail");

    assert!(error.message.contains("branch is checked out in worktree"));
    assert!(error.message.contains("-linked"));
    let _ = remove_dir_all(worktree_path);
}

#[test]
fn cli_delete_branch_allows_tip_contained_by_another_local_branch() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_delete_branch_allows_tip_contained_by_another_local_branch"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-delete-contained-branch");
    let default_branch = repo.run_git_capture(&["branch", "--show-current"]);
    let default_branch = default_branch.trim();
    repo.run_git(&["checkout", "-b", "feature/base"]);
    write(repo.path().join("a.txt"), "feature\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "feature base"]);
    repo.run_git(&["checkout", default_branch]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .create_branch("feature/temp", "feature/base")
        .expect("branch from non-current base should be created");
    backend
        .delete_branch("feature/temp", BranchDeleteMode::Local, false)
        .expect_err("safe delete should ask for confirmation when not merged into current branch");
    backend
        .delete_branch("feature/temp", BranchDeleteMode::Local, true)
        .expect("force delete should delete after confirmation");

    let branches = repo.run_git_capture(&["branch", "--format=%(refname:short)"]);
    assert!(branches.lines().any(|line| line == "feature/base"));
    assert!(!branches.lines().any(|line| line == "feature/temp"));
}

#[test]
fn cli_delete_branch_still_rejects_unique_unmerged_tip() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping cli_delete_branch_still_rejects_unique_unmerged_tip"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("cli-delete-unique-branch");
    let default_branch = repo.run_git_capture(&["branch", "--show-current"]);
    let default_branch = default_branch.trim();
    repo.run_git(&["checkout", "-b", "feature/unique"]);
    write(repo.path().join("a.txt"), "unique\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "unique branch"]);
    repo.run_git(&["checkout", default_branch]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let error = backend
        .delete_branch("feature/unique", BranchDeleteMode::Local, false)
        .expect_err("unique unmerged branch should not be deleted");

    assert!(error.message.contains("not fully merged"));
    let branches = repo.run_git_capture(&["branch", "--format=%(refname:short)"]);
    assert!(branches.lines().any(|line| line == "feature/unique"));
}
