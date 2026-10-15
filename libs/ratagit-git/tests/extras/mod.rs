use super::*;
use ratagit_core::extras::RepositoryOperation;

#[test]
fn test_ignore_paths_are_literal_idempotent_and_keep_unrelated_files() {
    let repo = TmpGitRepo::new("ignore-literal");
    write(repo.path().join("[new] file.txt"), "ignored\n").unwrap();
    write(repo.path().join("n file.txt"), "visible\n").unwrap();
    write(repo.path().join(".gitignore"), "# existing, no newline").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let op = RepositoryOperation::Ignore("[new] file.txt".into());
    backend.repository_operation(&op).unwrap();
    backend.repository_operation(&op).unwrap();
    let text = std::fs::read_to_string(repo.path().join(".gitignore")).unwrap();
    assert_eq!(text, "# existing, no newline\n/\\[new\\]\\ file.txt\n");
    let status = repo.run_git_capture(&["status", "--short", "--untracked-files=all"]);
    assert!(!status.contains("[new]"));
    assert!(status.contains("n file.txt"));
    assert!(
        backend
            .repository_operation(&RepositoryOperation::Ignore("../escape".into()))
            .is_err()
    );
}

#[test]
fn test_stash_apply_retains_stash_and_drop_removes_it() {
    let repo = TmpGitRepo::new("stash-apply-drop");
    write(repo.path().join("file.txt"), "base\n").unwrap();
    repo.run_git(&["add", "file.txt"]);
    repo.run_git(&["commit", "-m", "initial"]);
    write(repo.path().join("file.txt"), "changed\n").unwrap();
    repo.run_git(&["stash", "push", "-m", "saved"]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    backend
        .repository_operation(&RepositoryOperation::StashApply("stash@{0}".into()))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
        "changed\n"
    );
    assert_eq!(backend.refresh_stashes().unwrap().len(), 1);
    backend
        .repository_operation(&RepositoryOperation::StashDrop("stash@{0}".into()))
        .unwrap();
    assert!(backend.refresh_stashes().unwrap().is_empty());
}
