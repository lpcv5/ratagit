use std::collections::BTreeMap;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ratagit_git::{GitBackend, HybridGitBackend, is_git_repo};

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
    repo.run_git(&["branch", "-M", "main"]);
    repo
}

#[test]
fn git2_refresh_snapshot_reads_status_refs_commits_and_stashes() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_refresh_snapshot_reads_status_refs_commits_and_stashes"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("refresh-snapshot");
    write(repo.path().join("a.txt"), "stash change\n").expect("a.txt should be writable");
    repo.run_git(&["stash", "push", "-m", "saved stash", "--", "a.txt"]);
    repo.run_git(&["branch", "feature/test"]);

    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("b.txt"), "b2\n").expect("b.txt should be writable");
    repo.run_git(&["add", "--", "b.txt"]);
    create_dir_all(repo.path().join("nested")).expect("nested dir should be creatable");
    write(repo.path().join("nested").join("new.txt"), "new\n")
        .expect("nested new file should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let snapshot = backend
        .refresh_snapshot()
        .expect("snapshot should refresh with git2");

    assert_eq!(snapshot.current_branch, "main");
    assert!(!snapshot.detached_head);
    assert_eq!(snapshot.status_summary, "staged: 1, unstaged: 2");
    assert_eq!(snapshot.commits[0].summary, "init");
    assert!(
        snapshot
            .stashes
            .iter()
            .any(|stash| { stash.id == "stash@{0}" && stash.summary.contains("saved stash") })
    );

    let branches = snapshot
        .branches
        .iter()
        .map(|branch| (branch.name.as_str(), branch.is_current))
        .collect::<Vec<_>>();
    assert_eq!(branches, vec![("feature/test", false), ("main", true)]);

    let files = snapshot
        .files
        .iter()
        .map(|entry| (entry.path.as_str(), (entry.staged, entry.untracked)))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(files.get("a.txt"), Some(&(false, false)));
    assert_eq!(files.get("b.txt"), Some(&(true, false)));
    assert_eq!(files.get("nested/new.txt"), Some(&(false, true)));
}

#[test]
fn git2_files_details_diff_emits_unstaged_and_staged_sections() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_files_details_diff_emits_unstaged_and_staged_sections"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("details-diff");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("b.txt"), "b2\n").expect("b.txt should be writable");
    repo.run_git(&["add", "--", "b.txt"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let diff = backend
        .files_details_diff(&["a.txt".to_string(), "b.txt".to_string()])
        .expect("diff should render");

    assert!(diff.contains("### unstaged"));
    assert!(diff.contains("diff --git a/a.txt b/a.txt"));
    assert!(diff.contains("### staged"));
    assert!(diff.contains("diff --git a/b.txt b/b.txt"));
}

#[test]
fn git2_files_details_diff_emits_untracked_file_patch() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_files_details_diff_emits_untracked_file_patch"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("untracked-details-diff");
    write(repo.path().join("new.txt"), "hello\nworld\n").expect("new.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let diff = backend
        .files_details_diff(&["new.txt".to_string()])
        .expect("untracked diff should render");

    assert!(diff.contains("### unstaged"));
    assert!(diff.contains("diff --git a/new.txt b/new.txt"));
    assert!(diff.contains("new file mode 100644"));
    assert!(diff.contains("--- /dev/null"));
    assert!(diff.contains("+++ b/new.txt"));
    assert!(diff.contains("+hello"));
    assert!(diff.contains("+world"));
}

#[test]
fn git2_stage_and_unstage_files_preserves_worktree_changes() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_stage_and_unstage_files_preserves_worktree_changes"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("stage-unstage");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");
    write(repo.path().join("new.txt"), "new\n").expect("new.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .stage_files(&["a.txt".to_string(), "new.txt".to_string()])
        .expect("stage_files should stage modified and untracked files");

    let staged_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(staged_status.lines().any(|line| line == "M  a.txt"));
    assert!(staged_status.lines().any(|line| line == "A  new.txt"));

    backend
        .unstage_files(&["a.txt".to_string(), "new.txt".to_string()])
        .expect("unstage_files should restore index to HEAD");

    let unstaged_status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(unstaged_status.lines().any(|line| line == " M a.txt"));
    assert!(unstaged_status.lines().any(|line| line == "?? new.txt"));
}

#[test]
fn hybrid_backend_uses_cli_executor_for_commit_creation() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping hybrid_backend_uses_cli_executor_for_commit_creation"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("hybrid-commit");
    write(repo.path().join("a.txt"), "a2\n").expect("a.txt should be writable");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    backend
        .stage_files(&["a.txt".to_string()])
        .expect("git2 stage should succeed");
    backend
        .create_commit("feat: hybrid commit")
        .expect("cli-executed commit should succeed");

    let message = repo.run_git_capture(&["log", "-1", "--pretty=%s"]);
    assert_eq!(message.trim(), "feat: hybrid commit");
}

#[test]
fn hybrid_backend_discovers_repo_from_subdirectory() {
    if !git_available() {
        eprintln!("git is unavailable, skipping hybrid_backend_discovers_repo_from_subdirectory");
        return;
    }

    let repo = seeded_repo_with_two_files("discover-subdir");
    let subdir = repo.path().join("nested").join("child");
    create_dir_all(&subdir).expect("subdir should be creatable");

    assert!(is_git_repo(&subdir));
    let mut backend =
        HybridGitBackend::open(&subdir).expect("hybrid backend should open from subdir");
    let snapshot = backend
        .refresh_snapshot()
        .expect("snapshot should refresh from discovered repo");
    assert_eq!(snapshot.current_branch, "main");
}

#[test]
fn git2_refresh_snapshot_handles_unborn_branch() {
    if !git_available() {
        eprintln!("git is unavailable, skipping git2_refresh_snapshot_handles_unborn_branch");
        return;
    }

    let repo = TmpGitRepo::new("unborn-branch");
    repo.run_git(&["symbolic-ref", "HEAD", "refs/heads/main"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let snapshot = backend
        .refresh_snapshot()
        .expect("unborn branch snapshot should refresh");

    assert_eq!(snapshot.current_branch, "main");
    assert!(!snapshot.detached_head);
    assert_eq!(snapshot.status_summary, "staged: 0, unstaged: 0");
    assert!(snapshot.commits.is_empty());
    assert!(snapshot.branches.is_empty());
}
