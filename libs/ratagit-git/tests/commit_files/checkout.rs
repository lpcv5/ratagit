use super::*;

#[test]
fn commit_files_checkout_selection_larger_than_windows_command_line_preserves_index() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-many-paths");
    let paths = (0..250)
        .map(|n| format!("file-{n:03}-{}[x].txt", "a".repeat(128)))
        .collect::<Vec<_>>();
    assert!(paths.iter().map(String::len).sum::<usize>() > 32767);
    for path in &paths {
        write(repo.path().join(path), "old\n").unwrap();
    }
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "many old files"]);
    let source = id(&repo);
    for path in &paths {
        write(repo.path().join(path), "new\n").unwrap();
    }
    repo.run_git(&["commit", "-am", "many new files"]);
    let original = id(&repo);
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    run(
        &repo,
        Operation::CommitFileCheckout {
            oid: source,
            paths: paths.clone(),
        },
    )
    .unwrap();
    assert_eq!(id(&repo), original);
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
    for path in paths {
        assert_eq!(
            std::fs::read_to_string(repo.path().join(path)).unwrap(),
            "old\n"
        );
    }
}
