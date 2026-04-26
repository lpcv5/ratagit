use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ratagit_core::ResetMode;
use ratagit_git::{CliGitBackend, GitBackend};

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

    let mut backend = CliGitBackend::new(repo.path().to_path_buf());
    let message = "feat: multiline subject\n\nline 1\nline 2";
    backend
        .create_commit(message)
        .expect("create_commit should succeed");

    let actual = repo.run_git_capture(&["log", "-1", "--pretty=%B"]);
    assert_eq!(actual.trim_end(), message);
}

#[test]
fn cli_stash_push_uses_title_for_all_changes() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_stash_push_uses_title_for_all_changes");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-stash-push");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");

    let mut backend = CliGitBackend::new(repo.path().to_path_buf());
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

    let mut backend = CliGitBackend::new(repo.path().to_path_buf());
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
fn cli_stash_files_limits_stash_to_selected_paths() {
    if !git_available() {
        eprintln!("git is unavailable, skipping cli_stash_files_limits_stash_to_selected_paths");
        return;
    }

    let repo = seeded_repo_with_two_files("cli-stash-files");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("b.txt"), "b2\n").expect("b.txt should be writable");

    let mut backend = CliGitBackend::new(repo.path().to_path_buf());
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

    let mut backend = CliGitBackend::new(repo.path().to_path_buf());
    backend
        .reset(ResetMode::Hard)
        .expect("hard reset should succeed");

    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(!status.contains("a.txt"));
    assert!(status.contains("?? new.txt"));
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

    let mut backend = CliGitBackend::new(repo.path().to_path_buf());
    backend.nuke().expect("nuke should succeed");

    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert_eq!(status.trim(), "");
}
