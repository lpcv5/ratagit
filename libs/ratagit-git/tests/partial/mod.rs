use super::*;
use ratagit_core::patch::{PatchChange, PatchDirection, PatchLineKind, PatchRequest, PatchSource};

fn tracked_repo(name: &str, content: &[u8]) -> TmpGitRepo {
    let repo = TmpGitRepo::new(name);
    repo.run_git(&["config", "core.autocrlf", "false"]);
    write(repo.path().join("file.txt"), content).unwrap();
    repo.run_git(&["add", "--", "file.txt"]);
    repo.run_git(&["commit", "-m", "initial"]);
    repo
}

#[test]
fn test_stash_patch_includes_tracked_and_untracked_files_read_only() {
    let repo = tracked_repo("stash-patch-mixed", b"before\n");
    write(repo.path().join("file.txt"), "after\n").unwrap();
    write(repo.path().join("new.txt"), "untracked content\n").unwrap();
    repo.run_git(&["stash", "push", "--include-untracked", "-m", "mixed"]);
    let before = (
        repo.run_git_capture(&["status", "--porcelain=v1"]),
        repo.run_git_capture(&["ls-files", "--stage"]),
        repo.run_git_capture(&["rev-parse", "stash@{0}"]),
    );
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Stash {
            index: 0,
            path: None,
        })
        .unwrap();
    assert!(!patch.truncated);
    assert_eq!(patch.files.len(), 2);
    assert!(
        patch
            .files
            .iter()
            .all(|file| file.source == PatchSource::Stash(0))
    );
    assert_eq!(
        patch.files[0].new.path.as_deref(),
        Some(Path::new("file.txt"))
    );
    let added = &patch.files[1];
    assert_eq!(added.new.path.as_deref(), Some(Path::new("new.txt")));
    assert_eq!(added.change, PatchChange::Added);
    assert!(added.old.path.is_none());
    assert_eq!(added.new.mode, 0o100644);
    let header = String::from_utf8_lossy(&added.header);
    assert!(header.starts_with("diff --git a/new.txt b/new.txt\n"));
    assert!(header.contains("new file mode 100644\n"));
    assert!(header.contains("--- /dev/null\n+++ b/new.txt\n"));
    assert_eq!(added.hunks[0].lines[0].kind, PatchLineKind::Added);
    assert_eq!(added.hunks[0].lines[0].content, b"untracked content\n");
    let ids = patch.line_ids();
    for direction in [
        PatchDirection::Stage,
        PatchDirection::Unstage,
        PatchDirection::Discard,
    ] {
        assert!(backend.apply_patch_lines(&patch, &ids, direction).is_err());
    }
    assert_eq!(
        before,
        (
            repo.run_git_capture(&["status", "--porcelain=v1"]),
            repo.run_git_capture(&["ls-files", "--stage"]),
            repo.run_git_capture(&["rev-parse", "stash@{0}"]),
        )
    );
    assert!(!repo.path().join("new.txt").exists());
    assert_eq!(
        std::fs::read(repo.path().join("file.txt")).unwrap(),
        b"before\n"
    );
}

#[test]
fn test_stash_patch_includes_untracked_only_stash() {
    let repo = tracked_repo("stash-patch-untracked-only", b"base\n");
    write(repo.path().join("new.txt"), "untracked only\n").unwrap();
    repo.run_git(&["stash", "push", "--include-untracked"]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Stash {
            index: 0,
            path: None,
        })
        .unwrap();
    assert!(!patch.truncated);
    assert_eq!(patch.files.len(), 1);
    assert_eq!(
        patch.files[0].new.path.as_deref(),
        Some(Path::new("new.txt"))
    );
    assert_eq!(patch.files[0].source, PatchSource::Stash(0));
    assert_eq!(
        patch.files[0].hunks[0].lines[0].content,
        b"untracked only\n"
    );
}

#[test]
fn test_stash_patch_filters_tracked_and_untracked_paths_literally() {
    let repo = tracked_repo("stash-patch-paths", b"before\n");
    write(repo.path().join("file.txt"), "after\n").unwrap();
    write(repo.path().join("[new].txt"), "literal path\n").unwrap();
    write(repo.path().join("n.txt"), "glob lookalike\n").unwrap();
    create_dir_all(repo.path().join("nested")).unwrap();
    write(repo.path().join("nested/new.txt"), "nested content\n").unwrap();
    repo.run_git(&["stash", "push", "--include-untracked"]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    for (path, expected_path, expected_content) in [
        ("file.txt", "file.txt", b"after\n".as_slice()),
        ("[new].txt", "[new].txt", b"literal path\n".as_slice()),
        ("nested", "nested/new.txt", b"nested content\n".as_slice()),
    ] {
        let patch = backend
            .load_patch(PatchRequest::Stash {
                index: 0,
                path: Some(path.into()),
            })
            .unwrap();
        assert_eq!(patch.files.len(), 1, "filter {path}");
        let file = &patch.files[0];
        assert_eq!(file.new.path.as_deref(), Some(Path::new(expected_path)));
        assert_eq!(file.source, PatchSource::Stash(0));
        assert!(
            file.hunks.iter().flat_map(|hunk| &hunk.lines).any(|line| {
                line.kind == PatchLineKind::Added && line.content == expected_content
            })
        );
    }
    let patch = backend
        .load_patch(PatchRequest::Stash {
            index: 0,
            path: Some("absent.txt".into()),
        })
        .unwrap();
    assert!(patch.files.is_empty());
}

#[test]
fn test_stash_patch_preserves_two_parent_stash() {
    let repo = tracked_repo("stash-patch-two-parents", b"before\n");
    write(repo.path().join("file.txt"), "after\n").unwrap();
    write(repo.path().join("left-untracked.txt"), "outside stash\n").unwrap();
    repo.run_git(&["stash", "push"]);
    assert_eq!(
        repo.run_git_capture(&["rev-list", "--parents", "-n", "1", "stash@{0}"])
            .split_whitespace()
            .count(),
        3
    );
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Stash {
            index: 0,
            path: None,
        })
        .unwrap();
    assert!(!patch.truncated);
    assert_eq!(patch.files.len(), 1);
    let file = &patch.files[0];
    assert_eq!(file.source, PatchSource::Stash(0));
    assert_eq!(file.change, PatchChange::Modified);
    assert_eq!(file.new.path.as_deref(), Some(Path::new("file.txt")));
    assert_eq!(file.hunks[0].lines[0].content, b"before\n");
    assert_eq!(file.hunks[0].lines[1].content, b"after\n");
    assert!(repo.path().join("left-untracked.txt").exists());
}

#[test]
fn test_stash_untracked_patch_shares_line_budget_and_binary_limits() {
    let repo = tracked_repo("stash-patch-limits", b"before\n");
    write(repo.path().join("file.txt"), "tracked\n".repeat(1000)).unwrap();
    write(repo.path().join("new-binary.bin"), b"binary\0content").unwrap();
    write(
        repo.path().join("new-large.txt"),
        "untracked\n".repeat(6000),
    )
    .unwrap();
    write(
        repo.path().join("new-oversized.txt"),
        vec![b'x'; 1024 * 1024 + 1],
    )
    .unwrap();
    repo.run_git(&["stash", "push", "--include-untracked"]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Stash {
            index: 0,
            path: None,
        })
        .unwrap();
    assert!(patch.truncated);
    assert_eq!(patch.files.len(), 3);
    assert_eq!(
        patch.files[0].new.path.as_deref(),
        Some(Path::new("file.txt"))
    );
    let binary = &patch.files[1];
    assert_eq!(
        binary.new.path.as_deref(),
        Some(Path::new("new-binary.bin"))
    );
    assert!(binary.binary);
    assert!(binary.hunks.is_empty());
    assert!(binary.encoded.is_none());
    assert!(String::from_utf8_lossy(&binary.header).contains("Binary files"));
    let large = &patch.files[2];
    assert_eq!(large.new.path.as_deref(), Some(Path::new("new-large.txt")));
    assert!(large.encoded.is_none());
    let line_count = patch
        .files
        .iter()
        .map(|file| {
            file.header
                .split_inclusive(|byte| *byte == b'\n')
                .count()
                .max(1)
                + file
                    .hunks
                    .iter()
                    .map(|hunk| hunk.lines.len())
                    .sum::<usize>()
        })
        .sum::<usize>();
    assert_eq!(line_count, 5000);
    let oversized = backend
        .load_patch(PatchRequest::Stash {
            index: 0,
            path: Some("new-oversized.txt".into()),
        })
        .unwrap();
    assert_eq!(oversized.files.len(), 1);
    assert!(oversized.files[0].binary);
    assert!(oversized.files[0].hunks.is_empty());
    assert!(oversized.files[0].encoded.is_none());
    assert_eq!(oversized.files[0].source, PatchSource::Stash(0));
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
    let header = String::from_utf8_lossy(&patch.files[0].header);
    assert!(header.starts_with("diff --git a/file.txt b/file.txt\n"));
    assert!(header.contains("Binary files a/file.txt and b/file.txt differ"));
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
        String::from_utf8_lossy(&patch.files[0].header)
            .contains("--- a/file.txt\n+++ b/file.txt\n")
    );
    assert!(
        backend
            .apply_patch_lines(&patch, &patch.line_ids()[..1], PatchDirection::Stage)
            .is_err()
    );
}

#[test]
fn test_reviewed_rows_reproduce_native_git_diff_text() {
    let before = (1..=40)
        .map(|n| format!("    line {n}\n"))
        .collect::<String>();
    let repo = tracked_repo("native-diff-rows", before.as_bytes());
    let after = before
        .replace("    line 3\n", "    CHANGED 3  \n")
        .replace("    line 30\n", "    CHANGED 30\n");
    write(repo.path().join("file.txt"), after).unwrap();
    let expected = repo.run_git_capture(&[
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--abbrev=7",
        "--",
        "file.txt",
    ]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::File("file.txt".into()))
        .unwrap();
    let content = ratagit_core::partial::patch_rows(&patch)
        .into_iter()
        .map(|row| {
            let prefix = match row.kind {
                PatchLineKind::Added => "+",
                PatchLineKind::Removed => "-",
                PatchLineKind::Context if row.id.is_some() => " ",
                _ => "",
            };
            format!("{prefix}{}\n", row.text)
        })
        .collect::<String>();
    assert_eq!(content, expected);
    assert_eq!(repo.run_git_capture(&["show", ":file.txt"]), before);
}

#[test]
fn test_mode_only_patch_retains_native_metadata_without_selectable_lines() {
    let repo = tracked_repo("native-mode-header", b"unchanged\n");
    repo.run_git(&["update-index", "--chmod=+x", "file.txt"]);
    let expected = repo.run_git_capture(&[
        "diff",
        "--cached",
        "--no-color",
        "--no-ext-diff",
        "--",
        "file.txt",
    ]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let patch = backend
        .load_patch(PatchRequest::Index("file.txt".into()))
        .unwrap();
    let rows = ratagit_core::partial::patch_rows(&patch);
    let content = rows
        .iter()
        .map(|row| format!("{}\n", row.text))
        .collect::<String>();
    assert_eq!(content, expected);
    assert!(content.contains("old mode 100644\nnew mode 100755\n"));
    assert!(rows.iter().all(|row| row.id.is_none()));
    assert!(patch.line_ids().is_empty());
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
