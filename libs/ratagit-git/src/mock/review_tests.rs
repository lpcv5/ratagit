use super::*;

#[test]
fn committing_a_partially_staged_mock_file_retains_unstaged_changes() {
    let mut backend = MockGitBackend::new(RepoSnapshot {
        status_summary: "staged: 1, unstaged: 1".into(),
        current_branch: "main".into(),
        detached_head: false,
        commits: Vec::new(),
        branches: Vec::new(),
        stashes: Vec::new(),
        files: vec![ratagit_core::FileEntry {
            path: "mixed.txt".into(),
            staged: true,
            unstaged: true,
            untracked: false,
            status: CommitFileStatus::Modified,
            conflicted: false,
        }],
    });
    let targets = [FileDiffTarget {
        path: "mixed.txt".into(),
        untracked: false,
        is_directory_marker: false,
    }];
    let diff = backend.files_details_diff(&targets).unwrap();
    assert!(diff.contains("### staged"));
    assert!(diff.contains("### unstaged"));
    backend.create_commit("consume only staged").unwrap();
    assert_eq!(backend.snapshot.files.len(), 1);
    let file = &backend.snapshot.files[0];
    assert!(!file.staged && file.unstaged);
    assert_eq!(backend.snapshot.status_summary, "staged: 0, unstaged: 1");
    let diff = backend.files_details_diff(&targets).unwrap();
    assert!(diff.contains("### unstaged"));
    assert!(!diff.contains("### staged"));
}
