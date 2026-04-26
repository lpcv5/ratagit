use std::collections::BTreeMap;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ratagit_core::{COMMITS_PAGE_SIZE, CommitFileStatus, CommitHashStatus};
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

fn repo_with_three_commits(case_name: &str) -> TmpGitRepo {
    let repo = seeded_repo_with_two_files(case_name);
    for (file, message) in [("a.txt", "second"), ("b.txt", "third")] {
        write(repo.path().join(file), format!("{message}\n")).expect("file should be writable");
        repo.run_git(&["add", "--", file]);
        repo.run_git(&["commit", "-m", message]);
    }
    repo
}

fn feature_repo_with_three_commits(case_name: &str) -> TmpGitRepo {
    let repo = seeded_repo_with_two_files(case_name);
    repo.run_git(&["checkout", "-b", "feature/rewrite"]);
    for (file, message) in [("a.txt", "second"), ("b.txt", "third")] {
        write(repo.path().join(file), format!("{message}\n")).expect("file should be writable");
        repo.run_git(&["add", "--", file]);
        repo.run_git(&["commit", "-m", message]);
    }
    repo
}

fn commit_id(repo: &TmpGitRepo, rev: &str) -> String {
    repo.run_git_capture(&["rev-parse", rev]).trim().to_string()
}

fn log_subjects(repo: &TmpGitRepo) -> Vec<String> {
    repo.run_git_capture(&["log", "--format=%s"])
        .lines()
        .map(str::to_string)
        .collect()
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
fn git2_refresh_snapshot_reads_recent_commits_from_head_first() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_refresh_snapshot_reads_recent_commits_from_head_first"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("recent-commits");
    for index in 1..=12 {
        write(repo.path().join("a.txt"), format!("commit {index}\n"))
            .expect("a.txt should be writable");
        repo.run_git(&["add", "--", "a.txt"]);
        repo.run_git(&["commit", "-m", &format!("commit {index}")]);
    }

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let snapshot = backend
        .refresh_snapshot()
        .expect("snapshot should refresh with recent commits");

    assert_eq!(snapshot.commits.len(), 13);
    assert_eq!(snapshot.commits[0].summary, "commit 12");
    assert_eq!(snapshot.commits[12].summary, "init");
}

#[test]
fn git2_commit_snapshot_and_pages_load_one_hundred_at_a_time() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_commit_snapshot_and_pages_load_one_hundred_at_a_time"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("commit-pages");
    for index in 1..=125 {
        write(repo.path().join("a.txt"), format!("commit {index}\n"))
            .expect("a.txt should be writable");
        repo.run_git(&["add", "--", "a.txt"]);
        repo.run_git(&["commit", "-m", &format!("commit {index}")]);
    }

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let snapshot = backend
        .refresh_snapshot()
        .expect("snapshot should refresh with first commit page");
    assert_eq!(snapshot.commits.len(), COMMITS_PAGE_SIZE);
    assert_eq!(snapshot.commits[0].summary, "commit 125");
    assert_eq!(snapshot.commits[99].summary, "commit 26");

    let page = backend
        .load_more_commits(COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE)
        .expect("second commit page should load");
    assert_eq!(page.len(), 26);
    assert_eq!(page[0].summary, "commit 25");
    assert_eq!(page[25].summary, "init");
}

#[test]
fn git2_refresh_snapshot_reads_commit_metadata_and_hash_status() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_refresh_snapshot_reads_commit_metadata_and_hash_status"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("commit-metadata");
    repo.run_git(&["checkout", "-b", "feature/status"]);
    repo.run_git(&["config", "user.name", "Alice Baker"]);
    write(repo.path().join("a.txt"), "pushed\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "pushed commit", "-m", "pushed body"]);
    repo.run_git(&["branch", "feature/upstream"]);
    repo.run_git(&["branch", "--set-upstream-to=feature/upstream"]);
    write(repo.path().join("a.txt"), "unpushed\n").expect("a.txt should be writable");
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "unpushed commit"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let snapshot = backend
        .refresh_snapshot()
        .expect("snapshot should refresh commit metadata");

    assert_eq!(snapshot.commits[0].summary, "unpushed commit");
    assert_eq!(snapshot.commits[0].hash_status, CommitHashStatus::Unpushed);
    assert_eq!(snapshot.commits[1].summary, "pushed commit");
    assert_eq!(snapshot.commits[1].author_name, "Alice Baker");
    assert!(snapshot.commits[1].message.contains("pushed body"));
    assert_eq!(snapshot.commits[1].graph, "●");
    assert_eq!(snapshot.commits[1].hash_status, CommitHashStatus::Pushed);
    assert_eq!(
        snapshot.commits[2].hash_status,
        CommitHashStatus::MergedToMain
    );
}

#[test]
fn hybrid_backend_checks_out_commit_as_detached_head() {
    if !git_available() {
        eprintln!("git is unavailable, skipping hybrid_backend_checks_out_commit_as_detached_head");
        return;
    }

    let repo = repo_with_three_commits("detached-commit");
    let target = commit_id(&repo, "HEAD~1");
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    backend
        .checkout_commit_detached(&target, false)
        .expect("detached checkout should succeed");

    assert_eq!(
        repo.run_git_capture(&["rev-parse", "--abbrev-ref", "HEAD"])
            .trim(),
        "HEAD"
    );
    assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]).trim(), target);
}

#[test]
fn hybrid_backend_replays_linear_commit_rewrites() {
    if !git_available() {
        eprintln!("git is unavailable, skipping hybrid_backend_replays_linear_commit_rewrites");
        return;
    }

    let delete_repo = feature_repo_with_three_commits("delete-commits");
    let delete_target = commit_id(&delete_repo, "HEAD~1");
    HybridGitBackend::open(delete_repo.path())
        .expect("hybrid backend should open")
        .delete_commits(&[delete_target])
        .expect("delete should replay history");
    assert_eq!(log_subjects(&delete_repo), vec!["third", "init"]);

    let fixup_repo = feature_repo_with_three_commits("fixup-commits");
    let fixup_target = commit_id(&fixup_repo, "HEAD");
    HybridGitBackend::open(fixup_repo.path())
        .expect("hybrid backend should open")
        .fixup_commits(&[fixup_target])
        .expect("fixup should replay history");
    assert_eq!(log_subjects(&fixup_repo), vec!["second", "init"]);

    let squash_repo = feature_repo_with_three_commits("squash-commits");
    let squash_target = commit_id(&squash_repo, "HEAD");
    HybridGitBackend::open(squash_repo.path())
        .expect("hybrid backend should open")
        .squash_commits(&[squash_target])
        .expect("squash should replay history");
    assert_eq!(log_subjects(&squash_repo), vec!["second", "init"]);
    assert!(
        squash_repo
            .run_git_capture(&["log", "-1", "--format=%B"])
            .contains("third")
    );

    let reword_repo = feature_repo_with_three_commits("reword-commits");
    let reword_target = commit_id(&reword_repo, "HEAD");
    HybridGitBackend::open(reword_repo.path())
        .expect("hybrid backend should open")
        .reword_commit(&reword_target, "third reworded\n\nnew body")
        .expect("reword should replay history");
    assert_eq!(log_subjects(&reword_repo)[0], "third reworded");
    assert!(
        reword_repo
            .run_git_capture(&["log", "-1", "--format=%B"])
            .contains("new body")
    );
}

#[test]
fn hybrid_backend_rejects_public_and_root_parent_rewrites() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping hybrid_backend_rejects_public_and_root_parent_rewrites"
        );
        return;
    }

    let public_repo = repo_with_three_commits("rewrite-public-history");
    let public_target = commit_id(&public_repo, "HEAD");
    let error = HybridGitBackend::open(public_repo.path())
        .expect("hybrid backend should open")
        .delete_commits(&[public_target])
        .expect_err("main-reachable commits should not be rewritten");
    assert!(error.message.contains("merged to main"));

    let root_parent_repo = seeded_repo_with_two_files("rewrite-root-parent");
    root_parent_repo.run_git(&["checkout", "-b", "feature/root-parent"]);
    write(root_parent_repo.path().join("a.txt"), "feature\n").expect("a.txt should be writable");
    root_parent_repo.run_git(&["add", "--", "a.txt"]);
    root_parent_repo.run_git(&["commit", "-m", "feature change"]);
    let root_parent_target = commit_id(&root_parent_repo, "HEAD");
    let error = HybridGitBackend::open(root_parent_repo.path())
        .expect("hybrid backend should open")
        .fixup_commits(&[root_parent_target])
        .expect_err("squash/fixup into root should be rejected");
    assert!(error.message.contains("cannot squash or fixup into root"));
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
fn git2_files_details_diff_treats_selected_paths_as_literals() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_files_details_diff_treats_selected_paths_as_literals"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("literal-pathspec-details");
    write(repo.path().join("literal[abc].txt"), "v1\n")
        .expect("literal path file should be writable");
    repo.run_git(&["add", "--", "literal[abc].txt"]);
    repo.run_git(&["commit", "-m", "add literal path"]);
    write(repo.path().join("literal[abc].txt"), "v2\n")
        .expect("literal path file should be modified");

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let diff = backend
        .files_details_diff(&["literal[abc].txt".to_string()])
        .expect("literal path diff should render");

    assert!(diff.contains("diff --git a/literal[abc].txt b/literal[abc].txt"));
    assert!(diff.contains("v2"), "{diff}");
}

#[test]
fn git2_commit_details_diff_emits_header_and_patch() {
    if !git_available() {
        eprintln!("git is unavailable, skipping git2_commit_details_diff_emits_header_and_patch");
        return;
    }

    let repo = repo_with_three_commits("commit-details-diff");
    let commit = commit_id(&repo, "HEAD");
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    let diff = backend
        .commit_details_diff(&commit)
        .expect("commit details diff should render");

    assert!(diff.contains(&format!("commit {commit}")));
    assert!(diff.contains("Author:"));
    assert!(diff.contains("diff --git a/b.txt b/b.txt"));
    assert!(diff.contains("-b1"));
    assert!(diff.contains("+third"));
}

#[test]
fn git2_commit_files_and_file_diff_follow_selected_path() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_commit_files_and_file_diff_follow_selected_path"
        );
        return;
    }

    let repo = repo_with_three_commits("commit-files");
    let commit = commit_id(&repo, "HEAD");
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    let files = backend
        .commit_files(&commit)
        .expect("commit files should render");

    assert!(files.iter().any(|file| {
        file.path == "b.txt" && file.old_path.is_none() && file.status == CommitFileStatus::Modified
    }));
    let target = ratagit_core::CommitFileDiffTarget {
        commit_id: commit,
        paths: vec![ratagit_core::CommitFileDiffPath {
            path: "b.txt".to_string(),
            old_path: None,
        }],
    };
    let diff = backend
        .commit_file_diff(&target)
        .expect("commit file diff should render");

    assert!(diff.contains("diff --git a/b.txt b/b.txt"));
    assert!(diff.contains("-b1"));
    assert!(diff.contains("+third"));
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
