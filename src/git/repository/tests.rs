use super::references::parse_remote_from_upstream;
use super::*;
use crate::git::{capabilities::*, models::*};
use pretty_assertions::assert_eq;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

mod file_tools;
mod scheduling;

fn write_file(path: &Path, content: &str) {
    fs::write(path, content).expect("write file");
}

fn init_repo_with_commit() -> (TempDir, Git2Repository) {
    let dir = TempDir::new().expect("tempdir");
    let repo = git2::Repository::init(dir.path()).expect("init repo");

    let file = dir.path().join("tracked.txt");
    write_file(&file, "v1\n");

    let mut index = repo.index().expect("index");
    index
        .add_path(Path::new("tracked.txt"))
        .expect("add tracked.txt");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("find tree");

    let sig = git2::Signature::now("tester", "tester@example.com").expect("signature");
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
        .expect("initial commit");

    let repo = Git2Repository::open(dir.path()).expect("open git2 repo");
    (dir, repo)
}

#[test]
fn test_discover_repo() {
    // Comment in English.
    let result = Git2Repository::discover();
    assert!(result.is_ok());
}

#[test]
fn test_stage_unstage_roundtrip() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "v2\n");

    let status = repo.status().expect("status before stage");
    assert!(status
        .unstaged
        .iter()
        .any(|f| f.path == std::path::Path::new("tracked.txt")));

    repo.stage(&PathBuf::from("tracked.txt")).expect("stage");
    let status = repo.status().expect("status after stage");
    assert!(status
        .staged
        .iter()
        .any(|f| f.path == std::path::Path::new("tracked.txt")));

    repo.unstage(&PathBuf::from("tracked.txt"))
        .expect("unstage");
    let status = repo.status().expect("status after unstage");
    assert!(status
        .unstaged
        .iter()
        .any(|f| f.path == std::path::Path::new("tracked.txt")));
}

#[test]
fn test_commit_happy_path() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("new.txt"), "hello\n");

    repo.stage(&PathBuf::from("new.txt")).expect("stage new");
    let oid = repo.commit("add new").expect("commit");
    assert!(!oid.is_empty());

    let commits = repo.commits(1).expect("commits");
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].message, "add new");
    assert_eq!(commits[0].oid, oid);
    assert!(!commits[0].graph.is_empty());
    assert_eq!(commits[0].sync_state, CommitSyncState::DefaultBranch);

    let patch = repo.commit_diff_scoped(&oid, None).expect("commit diff");
    assert!(!patch.is_empty());

    let files = repo.commit_files(&oid).expect("commit files");
    assert!(files
        .iter()
        .any(|f| f.path == std::path::Path::new("new.txt")));

    let scoped = repo
        .commit_diff_scoped(&oid, Some(std::path::Path::new("new.txt")))
        .expect("commit scoped diff");
    assert!(!scoped.is_empty());
}

#[test]
fn test_branch_create_checkout_delete() {
    let (_dir, repo) = init_repo_with_commit();
    repo.create_branch("feature/a").expect("create branch");
    assert!(repo
        .branches()
        .expect("branches")
        .iter()
        .any(|b| b.name == "feature/a"));

    repo.checkout_branch("feature/a").expect("checkout branch");
    assert!(repo
        .branches()
        .expect("branches after checkout")
        .iter()
        .any(|b| b.name == "feature/a" && b.is_current));

    repo.checkout_branch("main")
        .or_else(|_| repo.checkout_branch("master"))
        .expect("checkout default branch");
    repo.delete_branch("feature/a").expect("delete branch");
    assert!(!repo
        .branches()
        .expect("branches after delete")
        .iter()
        .any(|b| b.name == "feature/a"));
}

#[test]
fn test_stash_push_apply_drop() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "v2\n");

    let created = repo
        .stash_push_paths(&[PathBuf::from("tracked.txt")], "wip")
        .expect("stash push");
    assert_eq!(created, 0);
    assert_eq!(repo.stashes().expect("stashes after push").len(), 1);

    repo.stash_apply(0).expect("stash apply");
    let status_after_apply = repo.status().expect("status after stash apply");
    assert!(status_after_apply
        .unstaged
        .iter()
        .any(|f| f.path == std::path::Path::new("tracked.txt")));

    repo.stash_drop(0).expect("stash drop");
    assert!(repo.stashes().expect("stashes after drop").is_empty());
}

#[test]
fn test_stash_pop_restores_and_removes_entry() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "v3\n");

    repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "wip pop")
        .expect("stash push");
    assert_eq!(repo.stashes().expect("stashes after push").len(), 1);

    repo.stash_pop(0).expect("stash pop");
    let status = repo.status().expect("status after stash pop");
    assert!(status
        .unstaged
        .iter()
        .any(|f| f.path == std::path::Path::new("tracked.txt")));
    assert!(repo.stashes().expect("stashes after pop").is_empty());
}

#[test]
fn test_stash_pop_with_unrelated_local_change_keeps_both_changes() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "v3\n");

    repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "wip pop")
        .expect("stash push");

    write_file(&dir.path().join("local-only.txt"), "local\n");

    repo.stash_pop(0).expect("stash pop");

    let status = repo.status().expect("status after stash pop");
    assert!(status
        .unstaged
        .iter()
        .any(|f| f.path == std::path::Path::new("tracked.txt")));
    assert!(status
        .untracked
        .iter()
        .any(|f| f.path == std::path::Path::new("local-only.txt")));
    assert!(repo.stashes().expect("stashes after pop").is_empty());
}

#[test]
fn test_stash_push_only_selected_paths() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "tracked changed\n");
    write_file(&dir.path().join("other.txt"), "other changed\n");

    repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "partial")
        .expect("stash push selected path");

    let status = repo.status().expect("status after partial stash");
    assert!(!repo
        .stashes()
        .expect("stashes after partial push")
        .is_empty());
    assert!(status
        .unstaged
        .iter()
        .chain(status.untracked.iter())
        .any(|f| f.path == std::path::Path::new("other.txt")));
}

#[test]
fn test_stash_diff_for_selected_path() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "v2\n");

    repo.stash_push_paths(&[PathBuf::from("tracked.txt")], "diff path")
        .expect("stash push for diff");

    let diff = repo
        .stash_diff(0, Some(Path::new("tracked.txt")))
        .expect("stash diff for path");
    assert!(!diff.is_empty());
    assert!(diff.iter().any(|l| matches!(l.kind, DiffLineKind::Header)));
}

#[test]
fn test_diff_directory_includes_staged_unstaged_and_untracked_changes() {
    let (dir, repo) = init_repo_with_commit();
    fs::create_dir_all(dir.path().join("nested")).expect("create nested dir");

    write_file(
        &dir.path().join("nested").join("tracked.txt"),
        "tracked change\n",
    );
    repo.stage(&PathBuf::from("nested/tracked.txt"))
        .expect("stage tracked path");
    repo.commit("add nested tracked")
        .expect("commit nested tracked");

    write_file(&dir.path().join("nested").join("staged.txt"), "staged\n");
    repo.stage(&PathBuf::from("nested/staged.txt"))
        .expect("stage nested file");
    write_file(
        &dir.path().join("nested").join("tracked.txt"),
        "tracked changed again\n",
    );
    write_file(&dir.path().join("nested").join("new.txt"), "new file\n");

    let diff = repo
        .diff_directory(Path::new("nested"))
        .expect("directory diff");
    assert!(!diff.is_empty());
    assert!(diff
        .iter()
        .any(|line| line.content.contains("nested/staged.txt")));
    assert!(diff
        .iter()
        .any(|line| line.content.contains("nested/tracked.txt")));
    assert!(diff
        .iter()
        .any(|line| line.content.contains("nested/new.txt")));
}

#[test]
fn test_parse_remote_from_upstream() {
    assert_eq!(
        parse_remote_from_upstream("origin/main").as_deref(),
        Some("origin")
    );
    assert_eq!(
        parse_remote_from_upstream("upstream/feature/x").as_deref(),
        Some("upstream")
    );
    assert!(parse_remote_from_upstream("").is_none());
}

#[test]
fn test_async_git_repository_status() {
    let (_dir, repo) = init_repo_with_commit();
    let status = WorktreeRepository::status_async(&repo)
        .unwrap()
        .recv()
        .unwrap()
        .expect("status");
    assert!(status.unstaged.is_empty());
}

#[test]
fn test_async_git_repository_commits() {
    let (_dir, repo) = init_repo_with_commit();
    let commits = HistoryRepository::commits_async(&repo, 1)
        .unwrap()
        .recv()
        .unwrap()
        .expect("commits");
    assert_eq!(commits.len(), 1);
}
