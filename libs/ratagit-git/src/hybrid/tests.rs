use super::*;

#[test]
fn file_entry_from_status_maps_index_worktree_and_untracked_states() {
    let staged = file_entry_from_status("staged.txt".to_string(), Status::INDEX_MODIFIED);
    assert!(staged.staged);
    assert!(!staged.untracked);

    let unstaged = file_entry_from_status("unstaged.txt".to_string(), Status::WT_MODIFIED);
    assert!(!unstaged.staged);
    assert!(!unstaged.untracked);

    let untracked = file_entry_from_status("new.txt".to_string(), Status::WT_NEW);
    assert!(!untracked.staged);
    assert!(untracked.untracked);

    let both = file_entry_from_status(
        "both.txt".to_string(),
        Status::INDEX_MODIFIED | Status::WT_MODIFIED,
    );
    assert!(both.staged);
    assert!(!both.untracked);
}

#[test]
fn sort_files_is_deterministic_by_path() {
    let mut files = vec![
        FileEntry {
            path: "z.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "a.txt".to_string(),
            staged: true,
            untracked: false,
        },
    ];
    sort_files(&mut files);
    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "z.txt");
}

#[test]
fn branch_helpers_handle_normal_empty_and_detached_states() {
    assert_eq!(
        branch_name_from_reference_name("refs/heads/main"),
        Some("main")
    );
    assert_eq!(branch_name_from_reference_name("HEAD"), None);

    let current = branch_entry("main", "main", false);
    assert!(current.is_current);

    let detached = branch_entry("main", "main", true);
    assert!(!detached.is_current);
}

#[test]
fn summarize_files_matches_app_status_summary_shape() {
    let files = vec![
        FileEntry {
            path: "staged.txt".to_string(),
            staged: true,
            untracked: false,
        },
        FileEntry {
            path: "new.txt".to_string(),
            staged: false,
            untracked: true,
        },
    ];
    assert_eq!(summarize_files(&files), "staged: 1, unstaged: 1");
}
