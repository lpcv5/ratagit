use super::*;
use ratagit_core::patch::{PatchDirection, PatchLineKind, PatchRequest};

fn tracked_repo(name: &str, content: &[u8]) -> TmpGitRepo {
    let repo = TmpGitRepo::new(name);
    repo.run_git(&["config", "core.autocrlf", "false"]);
    write(repo.path().join("file.txt"), content).unwrap();
    repo.run_git(&["add", "--", "file.txt"]);
    repo.run_git(&["commit", "-m", "initial"]);
    repo
}

#[test]
fn test_partial_stage_unstage_discard_roundtrip() {
    let repo = tracked_repo("partial-roundtrip", b"one\ntwo\nthree\nfour\nfive\n");
    write(
        repo.path().join("file.txt"),
        "ONE\ntwo\nthree\nfour\nFIVE\n",
    )
    .unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let request = PatchRequest::File("file.txt".into());
    let patch = backend.load_patch(request.clone()).unwrap();
    let ids = patch
        .line_ids()
        .into_iter()
        .filter(|id| {
            let line = &patch.files[id.file].hunks[id.hunk].lines[id.line];
            line.old_line == Some(1) || line.new_line == Some(1)
        })
        .collect::<Vec<_>>();
    backend
        .apply_patch_lines(&patch, &ids, PatchDirection::Stage)
        .unwrap();
    assert_eq!(
        repo.run_git_capture(&["show", ":file.txt"]),
        "ONE\ntwo\nthree\nfour\nfive\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
        "ONE\ntwo\nthree\nfour\nFIVE\n"
    );
    let patch = backend
        .load_patch(PatchRequest::Index("file.txt".into()))
        .unwrap();
    backend
        .apply_patch_lines(&patch, &patch.line_ids(), PatchDirection::Unstage)
        .unwrap();
    assert_eq!(
        repo.run_git_capture(&["show", ":file.txt"]),
        "one\ntwo\nthree\nfour\nfive\n"
    );
    let patch = backend.load_patch(request).unwrap();
    backend
        .apply_patch_lines(&patch, &ids, PatchDirection::Discard)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
        "one\ntwo\nthree\nfour\nFIVE\n"
    );
}

#[test]
fn test_stale_patch_and_wrong_direction_do_not_mutate() {
    let repo = tracked_repo("partial-stale", b"old\n");
    write(repo.path().join("file.txt"), "reviewed\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    assert!(
        backend
            .apply_patch_lines(&patch, &patch.line_ids(), PatchDirection::Unstage)
            .is_err()
    );
    write(repo.path().join("file.txt"), "newer\n").unwrap();
    assert!(
        backend
            .apply_patch_lines(&patch, &patch.line_ids(), PatchDirection::Stage)
            .is_err()
    );
    assert_eq!(repo.run_git_capture(&["show", ":file.txt"]), "old\n");
}

#[test]
fn test_partial_untracked_and_literal_glob_paths() {
    let repo = tracked_repo("partial-untracked", b"base\n");
    write(repo.path().join("[new].txt"), "first\nsecond\n").unwrap();
    write(repo.path().join("n.txt"), "unrelated\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Untracked("[new].txt".into()))
        .unwrap();
    assert_eq!(patch.files.len(), 1);
    backend
        .apply_patch_lines(&patch, &patch.line_ids()[..1], PatchDirection::Stage)
        .unwrap();
    assert_eq!(repo.run_git_capture(&["show", ":[new].txt"]), "first\n");
    assert!(repo.run_git_capture(&["ls-files", "n.txt"]).is_empty());
}

#[test]
fn test_partial_preserves_crlf_and_missing_final_newline() {
    for (name, before, after) in [
        (
            "partial-crlf",
            b"old\r\nkeep\r\n".as_slice(),
            b"new\r\nkeep\r\n".as_slice(),
        ),
        ("partial-eof", b"old".as_slice(), b"new".as_slice()),
    ] {
        let repo = tracked_repo(name, before);
        write(repo.path().join("file.txt"), after).unwrap();
        let mut backend = HybridGitBackend::open(repo.path()).unwrap();
        let patch = backend
            .load_patch(PatchRequest::Worktree("file.txt".into()))
            .unwrap();
        backend
            .apply_patch_lines(&patch, &patch.line_ids(), PatchDirection::Stage)
            .unwrap();
        let git_repo = git2::Repository::open(repo.path()).unwrap();
        let oid = git_repo
            .index()
            .unwrap()
            .get_path(Path::new("file.txt"), 0)
            .unwrap()
            .id;
        assert_eq!(git_repo.find_blob(oid).unwrap().content(), after);
    }
}

#[test]
fn test_binary_and_truncated_patches_are_read_only() {
    let repo = tracked_repo("partial-limits", b"old\n");
    write(repo.path().join("file.txt"), b"binary\0data").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    assert!(patch.files.iter().any(|file| file.binary));
    assert!(
        backend
            .apply_patch_lines(&patch, &[], PatchDirection::Stage)
            .is_err()
    );
    write(repo.path().join("file.txt"), "line\n".repeat(6000)).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    assert!(patch.truncated);
    assert!(
        backend
            .apply_patch_lines(&patch, &patch.line_ids()[..1], PatchDirection::Stage)
            .is_err()
    );
}

#[test]
fn test_context_and_duplicate_lines_are_rejected() {
    let repo = tracked_repo("partial-invalid", b"old\nkeep\n");
    write(repo.path().join("file.txt"), "new\nkeep\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Worktree("file.txt".into()))
        .unwrap();
    let id = patch.line_ids()[0];
    assert!(
        backend
            .apply_patch_lines(&patch, &[id, id], PatchDirection::Stage)
            .is_err()
    );
    let context = ratagit_core::patch::LineId {
        file: 0,
        hunk: 0,
        line: patch.files[0].hunks[0]
            .lines
            .iter()
            .position(|line| line.kind == PatchLineKind::Context)
            .unwrap(),
    };
    assert!(
        backend
            .apply_patch_lines(&patch, &[context], PatchDirection::Stage)
            .is_err()
    );
}
#[test]
fn test_combined_preview_maps_to_independent_source_bytes() {
    let repo = tracked_repo("partial-combined", b"one\ntwo\nthree\nfour\nfive\n");
    write(
        repo.path().join("file.txt"),
        "ONE\ntwo\nthree\nfour\nfive\n",
    )
    .unwrap();
    repo.run_git(&["add", "file.txt"]);
    write(
        repo.path().join("file.txt"),
        "ONE\ntwo\nthree\nfour\nFIVE\n",
    )
    .unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    let rows = patch
        .preview
        .as_ref()
        .expect("disjoint staged and unstaged edits should combine");
    assert_eq!(
        rows.iter().filter(|row| row.text.starts_with("@@")).count(),
        1
    );
    assert!(rows.iter().any(|row| {
        row.id.is_some_and(|id| {
            patch.files[id.file].source == ratagit_core::patch::PatchSource::Index
        })
    }));
    let ids = patch
        .line_ids()
        .into_iter()
        .filter(|id| patch.files[id.file].source == ratagit_core::patch::PatchSource::Worktree)
        .collect::<Vec<_>>();
    backend
        .apply_patch_lines(&patch, &ids, PatchDirection::Stage)
        .unwrap();
    assert_eq!(
        repo.run_git_capture(&["show", ":file.txt"]),
        "ONE\ntwo\nthree\nfour\nFIVE\n"
    );
}

#[test]
fn test_overlapping_changes_keep_reviewed_source_previews() {
    let repo = tracked_repo("partial-overlap", b"base\n");
    write(repo.path().join("file.txt"), "staged\n").unwrap();
    repo.run_git(&["add", "file.txt"]);
    write(repo.path().join("file.txt"), "worktree\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    assert!(patch.preview.is_none());
    assert_eq!(patch.files.len(), 2);
    let ids = patch
        .line_ids()
        .into_iter()
        .filter(|id| patch.files[id.file].source == ratagit_core::patch::PatchSource::Index)
        .collect::<Vec<_>>();
    backend
        .apply_patch_lines(&patch, &ids, PatchDirection::Unstage)
        .unwrap();
    assert_eq!(repo.run_git_capture(&["show", ":file.txt"]), "base\n");
    assert_eq!(
        std::fs::read_to_string(repo.path().join("file.txt")).unwrap(),
        "worktree\n"
    );
}

#[test]
fn test_partial_deleted_file_does_not_delete_unselected_lines() {
    let repo = tracked_repo("partial-delete", b"first\nsecond\n");
    std::fs::remove_file(repo.path().join("file.txt")).unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    backend
        .apply_patch_lines(&patch, &patch.line_ids()[..1], PatchDirection::Stage)
        .unwrap();
    assert_eq!(repo.run_git_capture(&["show", ":file.txt"]), "second\n");
    assert!(!repo.path().join("file.txt").exists());
}
#[test]
fn test_renamed_file_is_not_partially_applied() {
    let repo = tracked_repo("partial-rename", b"one\ntwo\nthree\n");
    repo.run_git(&["mv", "file.txt", "renamed.txt"]);
    write(repo.path().join("renamed.txt"), "one\ntwo\nTHREE\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("renamed.txt".into()))
        .unwrap();
    let staged = patch
        .line_ids()
        .into_iter()
        .filter(|id| patch.files[id.file].source == ratagit_core::patch::PatchSource::Index)
        .collect::<Vec<_>>();
    assert!(
        backend
            .apply_patch_lines(&patch, &staged, PatchDirection::Unstage)
            .is_err()
    );
    assert!(
        repo.run_git_capture(&["ls-files", "renamed.txt"])
            .contains("renamed.txt")
    );
}
