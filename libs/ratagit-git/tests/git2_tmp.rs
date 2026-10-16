use std::collections::BTreeMap;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use ratagit_core::{COMMITS_PAGE_SIZE, CommitFileStatus, CommitHashStatus, FileDiffTarget};
use ratagit_git::{
    GitBackendHistoryRewrite, GitBackendRead, GitBackendWrite, HybridGitBackend, is_git_repo,
};

mod commit_files;

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
        if std::thread::panicking() {
            eprintln!("retained failing Git fixture: {}", self.path.display());
            return;
        }
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

fn feature_repo_with_five_commits(case_name: &str) -> TmpGitRepo {
    let repo = seeded_repo_with_two_files(case_name);
    repo.run_git(&["checkout", "-b", "feature/rewrite"]);
    for message in ["second", "third", "fourth", "fifth"] {
        write(repo.path().join("a.txt"), format!("{message}\n")).expect("a.txt should be writable");
        repo.run_git(&["add", "--", "a.txt"]);
        repo.run_git(&["commit", "-m", message]);
    }
    repo
}

fn commit_id(repo: &TmpGitRepo, rev: &str) -> String {
    repo.run_git_capture(&["rev-parse", rev]).trim().to_string()
}

fn file_diff_target(path: &str, untracked: bool) -> FileDiffTarget {
    FileDiffTarget {
        path: path.to_string(),
        untracked,
        is_directory_marker: path.ends_with('/'),
    }
}

fn strip_ansi(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if next.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            output.push(ch);
        }
    }
    output
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
fn hybrid_backend_squashes_non_contiguous_commits_with_rebase() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping hybrid_backend_squashes_non_contiguous_commits_with_rebase"
        );
        return;
    }

    let repo = feature_repo_with_five_commits("squash-non-contiguous");
    let third = commit_id(&repo, "HEAD~2");
    let fifth = commit_id(&repo, "HEAD");
    HybridGitBackend::open(repo.path())
        .expect("hybrid backend should open")
        .squash_commits(&[fifth, third])
        .expect("non-contiguous squash should use rebase");

    assert_eq!(log_subjects(&repo), vec!["fourth", "second", "init"]);
    assert!(
        repo.run_git_capture(&["log", "-1", "--format=%B"])
            .contains("fifth")
    );
    assert!(
        repo.run_git_capture(&["log", "-1", "--format=%B", "HEAD~1"])
            .contains("third")
    );
}

#[test]
fn hybrid_backend_rewrites_public_history_and_rejects_root_parent_rewrites() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping hybrid_backend_rewrites_public_history_and_rejects_root_parent_rewrites"
        );
        return;
    }

    let public_repo = repo_with_three_commits("rewrite-public-history");
    let public_target = commit_id(&public_repo, "HEAD");
    HybridGitBackend::open(public_repo.path())
        .expect("hybrid backend should open")
        .delete_commits(&[public_target])
        .expect("main-reachable commits should be rewritten");
    assert_eq!(log_subjects(&public_repo), vec!["second", "init"]);

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
        .files_details_diff(&[
            file_diff_target("a.txt", false),
            file_diff_target("b.txt", false),
        ])
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
        .files_details_diff(&[file_diff_target("new.txt", true)])
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
        .files_details_diff(&[file_diff_target("literal[abc].txt", false)])
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
    let plain_diff = strip_ansi(&diff);

    assert!(plain_diff.contains(&format!("commit {commit}")));
    assert!(plain_diff.contains("Author:"));
    assert!(plain_diff.contains("diff --git a/b.txt b/b.txt"));
    assert!(plain_diff.contains("-b1"));
    assert!(plain_diff.contains("+third"));
    assert!(diff.contains('\u{1b}'));
}

#[test]
fn git2_commit_details_diff_truncates_large_patch_preview() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_commit_details_diff_truncates_large_patch_preview"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("commit-details-diff-large");
    write(repo.path().join("large.txt"), "x".repeat(2 * 1024 * 1024))
        .expect("large file should be writable");
    repo.run_git(&["add", "--", "large.txt"]);
    repo.run_git(&["commit", "-m", "large patch"]);

    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let diff = backend
        .commit_details_diff("HEAD")
        .expect("large commit details diff should render a bounded preview");

    assert!(diff.contains("commit "));
    assert!(diff.contains("diff --git a/large.txt b/large.txt"));
    assert!(diff.contains("### commit diff truncated at 1048576 bytes"));
    assert!(
        diff.len() < 1_100_000,
        "bounded preview should stay near the configured limit"
    );
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
    let plain_diff = strip_ansi(&diff);

    assert!(plain_diff.contains("diff --git a/b.txt b/b.txt"));
    assert!(plain_diff.contains("-b1"));
    assert!(plain_diff.contains("+third"));
    assert!(diff.contains('\u{1b}'));
}

#[test]
fn git2_commit_file_diff_truncates_large_patch_preview() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_commit_file_diff_truncates_large_patch_preview"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("commit-file-diff-large");
    write(repo.path().join("large.txt"), "x".repeat(2 * 1024 * 1024))
        .expect("large file should be writable");
    repo.run_git(&["add", "--", "large.txt"]);
    repo.run_git(&["commit", "-m", "large file patch"]);

    let target = ratagit_core::CommitFileDiffTarget {
        commit_id: "HEAD".to_string(),
        paths: vec![ratagit_core::CommitFileDiffPath {
            path: "large.txt".to_string(),
            old_path: None,
        }],
    };
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let diff = backend
        .commit_file_diff(&target)
        .expect("large commit file diff should render a bounded preview");

    assert!(diff.contains("diff --git a/large.txt b/large.txt"));
    assert!(diff.contains("### commit diff truncated at 1048576 bytes"));
    assert!(
        diff.len() < 1_100_000,
        "bounded preview should stay near the configured limit"
    );
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
fn git2_unstage_rename_destination_restores_both_index_paths() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_rename_destination_restores_both_index_paths"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("unstage-rename");
    repo.run_git(&["mv", "--", "a.txt", "renamed.txt"]);
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let files = backend.refresh_files().expect("files should refresh").files;
    assert!(files.iter().any(|file| {
        file.path == "renamed.txt" && file.staged && file.status == CommitFileStatus::Renamed
    }));

    backend
        .unstage_file("renamed.txt")
        .expect("unstaging the destination should unstage the whole rename");

    assert_eq!(
        repo.run_git_capture(&["diff", "--cached", "--name-only"]),
        ""
    );
    assert_eq!(repo.run_git_capture(&["show", ":a.txt"]), "a1\n");
    assert_eq!(
        repo.run_git_capture(&["status", "--short", "--untracked-files=all"]),
        " D a.txt\n?? renamed.txt\n"
    );
    assert!(!repo.path().join("a.txt").exists());
    assert_eq!(
        std::fs::read_to_string(repo.path().join("renamed.txt")).unwrap(),
        "a1\n"
    );
}

#[test]
fn git2_unstage_modified_rename_preserves_other_staged_and_worktree_changes() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_modified_rename_preserves_other_staged_and_worktree_changes"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("unstage-modified-rename");
    let original = (0..20)
        .map(|line| format!("original content line {line}\n"))
        .collect::<String>();
    for (path, content) in [
        ("source[abc].txt", original.as_str()),
        ("sourcea.txt", "unrelated source\n"),
        ("renameda.txt", "unrelated destination\n"),
    ] {
        write(repo.path().join(path), content).unwrap();
    }
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "seed literal paths"]);
    repo.run_git(&["mv", "--", "source[abc].txt", "renamed[abc].txt"]);
    let staged_content = format!("{original}staged edit\n");
    write(repo.path().join("renamed[abc].txt"), &staged_content).unwrap();
    write(repo.path().join("sourcea.txt"), "unrelated staged source\n").unwrap();
    write(
        repo.path().join("renameda.txt"),
        "unrelated staged destination\n",
    )
    .unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["mv", "--", "a.txt", "other.txt"]);
    let worktree_content = format!("{staged_content}unstaged edit\n");
    write(repo.path().join("renamed[abc].txt"), &worktree_content).unwrap();
    write(
        repo.path().join("sourcea.txt"),
        "unrelated unstaged source\n",
    )
    .unwrap();
    write(repo.path().join("b.txt"), "unrelated unstaged file\n").unwrap();
    let unrelated_diff = repo.run_git_capture(&[
        "diff",
        "--cached",
        "--binary",
        "--",
        "a.txt",
        "other.txt",
        "sourcea.txt",
        "renameda.txt",
    ]);
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    let files = backend.refresh_files().expect("files should refresh").files;
    assert!(files.iter().any(|file| {
        file.path == "renamed[abc].txt"
            && file.staged
            && file.unstaged
            && file.status == CommitFileStatus::Renamed
    }));

    backend
        .unstage_files(&[
            "renamed[abc].txt".to_string(),
            "renamed[abc].txt".to_string(),
        ])
        .expect("modified rename should unstage once even with duplicate selections");

    assert_eq!(
        repo.run_git_capture(&[
            "diff",
            "--cached",
            "--binary",
            "--",
            "a.txt",
            "other.txt",
            "sourcea.txt",
            "renameda.txt",
        ]),
        unrelated_diff
    );
    let index = git2::Repository::open(repo.path())
        .unwrap()
        .index()
        .unwrap();
    assert!(index.get_path(Path::new("renamed[abc].txt"), 0).is_none());
    let source = index.get_path(Path::new("source[abc].txt"), 0).unwrap();
    assert_eq!(
        git2::Repository::open(repo.path())
            .unwrap()
            .find_blob(source.id)
            .unwrap()
            .content(),
        original.as_bytes()
    );
    for (path, expected) in [
        ("renamed[abc].txt", worktree_content.as_str()),
        ("sourcea.txt", "unrelated unstaged source\n"),
        ("renameda.txt", "unrelated staged destination\n"),
        ("b.txt", "unrelated unstaged file\n"),
        ("other.txt", "a1\n"),
    ] {
        assert_eq!(
            std::fs::read_to_string(repo.path().join(path)).unwrap(),
            expected
        );
    }
    assert!(!repo.path().join("source[abc].txt").exists());
    assert!(!repo.path().join("a.txt").exists());
}

#[test]
fn git2_unstage_case_only_renames_restores_original_index_spelling() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_case_only_renames_restores_original_index_spelling"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("unstage-case-only-renames");
    repo.run_git(&["config", "core.ignorecase", "true"]);
    // Multiple pairs exercise both HashSet orders without depending on its seed.
    for number in 0..16 {
        write(
            repo.path().join(format!("lower{number}.txt")),
            format!("content {number}\n"),
        )
        .unwrap();
    }
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "seed case-only renames"]);
    let mut selected = Vec::new();
    for number in 0..16 {
        let source = format!("lower{number}.txt");
        let destination = format!("LOWER{number}.txt");
        repo.run_git(&["mv", "--", &source, &destination]);
        selected.push(destination);
    }
    write(repo.path().join("b.txt"), "unrelated staged\n").unwrap();
    repo.run_git(&["add", "--", "b.txt"]);
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    backend
        .unstage_files(&selected)
        .expect("case-only renames should unstage regardless of removal order");

    assert_eq!(
        repo.run_git_capture(&["diff", "--cached", "--name-only"]),
        "b.txt\n"
    );
    let index = git2::Repository::open(repo.path())
        .unwrap()
        .index()
        .unwrap();
    for number in 0..16 {
        let source = format!("lower{number}.txt");
        let destination = format!("LOWER{number}.txt");
        assert!(index.iter().any(|entry| entry.path == source.as_bytes()));
        assert!(
            !index
                .iter()
                .any(|entry| entry.path == destination.as_bytes())
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join(destination)).unwrap(),
            format!("content {number}\n")
        );
    }
    assert_eq!(
        repo.run_git_capture(&["show", ":b.txt"]),
        "unrelated staged\n"
    );
}

#[test]
fn git2_unstage_symlink_rename_restores_link_mode_and_preserves_worktree() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_symlink_rename_restores_link_mode_and_preserves_worktree"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("unstage-symlink-rename");
    // Git's symlink emulation exercises real link-mode entries without requiring
    // Windows symlink privileges; the worktree contains the literal link target.
    repo.run_git(&["config", "core.symlinks", "false"]);
    write(repo.path().join("source-link"), "a.txt").unwrap();
    repo.run_git(&["add", "--", "source-link"]);
    let oid = repo.run_git_capture(&["rev-parse", ":source-link"]);
    repo.run_git(&[
        "update-index",
        "--cacheinfo",
        "120000",
        oid.trim(),
        "source-link",
    ]);
    repo.run_git(&["commit", "-m", "seed symlink"]);
    repo.run_git(&["mv", "--", "source-link", "renamed-link"]);
    write(repo.path().join("renamed-link"), "b.txt").unwrap();
    write(repo.path().join("b.txt"), "unrelated staged\n").unwrap();
    repo.run_git(&["add", "--", "b.txt"]);
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");
    assert!(backend.refresh_files().unwrap().files.iter().any(|file| {
        file.path == "renamed-link" && file.staged && file.status == CommitFileStatus::Renamed
    }));

    backend
        .unstage_file("renamed-link")
        .expect("symlink rename should restore both paths");

    assert_eq!(
        repo.run_git_capture(&["diff", "--cached", "--name-only"]),
        "b.txt\n"
    );
    let index = git2::Repository::open(repo.path())
        .unwrap()
        .index()
        .unwrap();
    let source = index.get_path(Path::new("source-link"), 0).unwrap();
    assert_eq!(source.mode, 0o120000);
    assert_eq!(source.id.to_string(), oid.trim());
    assert!(index.get_path(Path::new("renamed-link"), 0).is_none());
    assert!(!repo.path().join("source-link").exists());
    assert_eq!(
        std::fs::read_to_string(repo.path().join("renamed-link")).unwrap(),
        "b.txt"
    );
    assert_eq!(
        repo.run_git_capture(&["show", ":b.txt"]),
        "unrelated staged\n"
    );
}

#[test]
fn git2_unstage_symlink_rename_pairs_identical_targets_like_git() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_symlink_rename_pairs_identical_targets_like_git"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("unstage-identical-symlink-renames");
    repo.run_git(&["config", "core.symlinks", "false"]);
    for directory in ["alpha", "beta", "new"] {
        create_dir_all(repo.path().join(directory)).unwrap();
    }
    for path in ["alpha/source", "beta/other"] {
        write(repo.path().join(path), "../a.txt").unwrap();
        repo.run_git(&["add", "--", path]);
        let oid = repo.run_git_capture(&["rev-parse", &format!(":{path}")]);
        repo.run_git(&["update-index", "--cacheinfo", "120000", oid.trim(), path]);
    }
    repo.run_git(&["commit", "-m", "seed identical symlink targets"]);
    // Git pairs identical blobs by basename, regardless of how they were moved.
    repo.run_git(&["mv", "--", "alpha/source", "new/other"]);
    repo.run_git(&["mv", "--", "beta/other", "new/source"]);
    assert_eq!(
        repo.run_git_capture(&["diff", "--cached", "--name-status", "--find-renames"]),
        "R100\tbeta/other\tnew/other\nR100\talpha/source\tnew/source\n"
    );
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    backend
        .unstage_file("new/other")
        .expect("only the displayed rename pair should unstage");

    assert_eq!(
        repo.run_git_capture(&["diff", "--cached", "--name-status", "--find-renames"]),
        "R100\talpha/source\tnew/source\n"
    );
    let index = git2::Repository::open(repo.path())
        .unwrap()
        .index()
        .unwrap();
    assert!(index.get_path(Path::new("beta/other"), 0).is_some());
    assert!(index.get_path(Path::new("alpha/source"), 0).is_none());
    for path in ["new/other", "new/source"] {
        assert_eq!(
            std::fs::read_to_string(repo.path().join(path)).unwrap(),
            "../a.txt"
        );
    }
}

#[test]
fn git2_unstage_files_treats_selected_paths_as_literals() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_files_treats_selected_paths_as_literals"
        );
        return;
    }

    let repo = seeded_repo_with_two_files("unstage-literal-paths");
    for path in ["literal[abc].txt", "literala.txt"] {
        write(repo.path().join(path), "original\n").unwrap();
    }
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "seed literal paths"]);
    for path in [
        "literal[abc].txt",
        "literala.txt",
        "new[abc].txt",
        "newa.txt",
    ] {
        write(repo.path().join(path), "staged\n").unwrap();
    }
    repo.run_git(&["add", "."]);
    write(repo.path().join("literal[abc].txt"), "later edit\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    backend
        .unstage_files(&["literal[abc].txt".to_string(), "new[abc].txt".to_string()])
        .expect("literal paths should unstage without matching other files");

    assert_eq!(
        repo.run_git_capture(&["diff", "--cached", "--name-only"]),
        "literala.txt\nnewa.txt\n"
    );
    assert_eq!(
        repo.run_git_capture(&["show", ":literal[abc].txt"]),
        "original\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("literal[abc].txt")).unwrap(),
        "later edit\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("new[abc].txt")).unwrap(),
        "staged\n"
    );
}

#[test]
fn git2_unstage_files_before_first_commit_preserves_unselected_entries() {
    if !git_available() {
        eprintln!(
            "git is unavailable, skipping git2_unstage_files_before_first_commit_preserves_unselected_entries"
        );
        return;
    }

    let repo = TmpGitRepo::new("unstage-unborn");
    for path in ["new[abc].txt", "newa.txt"] {
        write(repo.path().join(path), "staged\n").unwrap();
    }
    repo.run_git(&["add", "."]);
    let mut backend = HybridGitBackend::open(repo.path()).expect("hybrid backend should open");

    backend
        .unstage_files(&["new[abc].txt".to_string(), "missing.txt".to_string()])
        .expect("unstaging before the first commit should remove only selected entries");
    backend
        .unstage_file("new[abc].txt")
        .expect("unstaging again should be harmless");

    assert_eq!(repo.run_git_capture(&["ls-files"]), "newa.txt\n");
    assert_eq!(
        std::fs::read_to_string(repo.path().join("new[abc].txt")).unwrap(),
        "staged\n"
    );
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
mod extras;
mod partial;
