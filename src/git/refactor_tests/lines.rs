use super::*;

fn select(patch: &Patch, kind: PatchLineKind, content: &[u8]) -> LineId {
    patch
        .line_ids()
        .into_iter()
        .find(|id| {
            let line = &patch.files[id.file].hunks[id.hunk].lines[id.line];
            line.kind == kind && line.content == content
        })
        .unwrap_or_else(|| {
            panic!(
                "missing line {:?} in {patch:?}",
                String::from_utf8_lossy(content)
            )
        })
}

fn prepare(old: &[u8], new: &[u8]) -> Fixture {
    let fixture = Fixture::new();
    fixture.write("file.txt", old);
    fixture.git_ok(&["add", "--all"]);
    if !fixture
        .git(&["diff", "--cached", "--quiet"])
        .status
        .success()
    {
        fixture.git_ok(&["commit", "-m", "line base"]);
    }
    fixture.write("file.txt", new);
    fixture
}

#[test]
fn test_line_stage_and_unstage_independent_changes_in_one_hunk() {
    let fixture = prepare(b"old1\nold2\n", b"new1\nnew2\n");
    let patch = fixture.worktree();
    let ids = [
        select(&patch, PatchLineKind::Removed, b"old2\n"),
        select(&patch, PatchLineKind::Added, b"new1\n"),
    ];
    fixture
        .repo
        .apply_lines(&patch, &ids, PatchDirection::Stage)
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), "old1\nnew1\n");
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        b"new1\nnew2\n"
    );
    let patch = fixture.patch(PatchRequest::Index("file.txt".into()));
    fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Unstage)
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), "old1\nold2\n");
}

#[test]
fn test_line_unstage_and_discard_keep_unselected_changes() {
    for direction in [PatchDirection::Unstage, PatchDirection::Discard] {
        let fixture = prepare(b"old1\nold2\n", b"new1\nnew2\n");
        let request = if direction == PatchDirection::Unstage {
            fixture.repo.stage(Path::new("file.txt")).unwrap();
            PatchRequest::Index("file.txt".into())
        } else {
            PatchRequest::Worktree("file.txt".into())
        };
        let patch = fixture.patch(request);
        let ids = [
            select(&patch, PatchLineKind::Removed, b"old2\n"),
            select(&patch, PatchLineKind::Added, b"new1\n"),
        ];
        fixture.repo.apply_lines(&patch, &ids, direction).unwrap();
        let index = fixture.git_ok(&["show", ":file.txt"]);
        let work = std::fs::read(fixture.dir.path().join("file.txt")).unwrap();
        if direction == PatchDirection::Unstage {
            assert_eq!(index, "old2\nnew2\n");
            assert_eq!(work, b"new1\nnew2\n");
        } else {
            assert_eq!(index, "old1\nold2\n");
            assert_eq!(work, b"old2\nnew2\n");
        }
    }
}

#[test]
fn test_partial_added_and_deleted_files_keep_correct_file_metadata() {
    let fixture = Fixture::new();
    fixture.write("new 文件.txt", "one\ntwo\nthree\n");
    let patch = fixture.patch(PatchRequest::Untracked("new 文件.txt".into()));
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Added, b"two\n")],
            PatchDirection::Stage,
        )
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":new 文件.txt"]), "two\n");
    fixture.repo.stage(Path::new("new 文件.txt")).unwrap();
    let patch = fixture.patch(PatchRequest::Index("new 文件.txt".into()));
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Added, b"two\n")],
            PatchDirection::Unstage,
        )
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":new 文件.txt"]), "one\nthree\n");

    let fixture = prepare(b"one\ntwo\nthree\n", b"");
    std::fs::remove_file(fixture.dir.path().join("file.txt")).unwrap();
    let patch = fixture.worktree();
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Removed, b"two\n")],
            PatchDirection::Stage,
        )
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), "one\nthree\n");
    assert!(!fixture.dir.path().join("file.txt").exists());
}

#[test]
fn test_discard_untracked_lines_and_restore_part_of_deleted_file() {
    let fixture = Fixture::new();
    fixture.write("new.txt", "one\ntwo\nthree\n");
    let patch = fixture.patch(PatchRequest::Untracked("new.txt".into()));
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Added, b"two\n")],
            PatchDirection::Discard,
        )
        .unwrap();
    assert_eq!(
        std::fs::read(fixture.dir.path().join("new.txt")).unwrap(),
        b"one\nthree\n"
    );
    let patch = fixture.patch(PatchRequest::Untracked("new.txt".into()));
    fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Discard)
        .unwrap();
    assert!(!fixture.dir.path().join("new.txt").exists());
    let fixture = prepare(b"one\ntwo\nthree\n", b"");
    std::fs::remove_file(fixture.dir.path().join("file.txt")).unwrap();
    let patch = fixture.worktree();
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Removed, b"two\n")],
            PatchDirection::Discard,
        )
        .unwrap();
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        b"two\n"
    );
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), "one\ntwo\nthree\n");
}

#[test]
fn test_line_patch_preserves_crlf_invalid_utf8_and_missing_final_newline() {
    let fixture = prepare(b"first\r\nold\r\nlast", b"first\r\nnew\xff\r\nlast");
    let patch = fixture.worktree();
    fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Stage)
        .unwrap();
    let raw = git2::Repository::open(fixture.dir.path()).unwrap();
    raw.index().unwrap().read(true).unwrap();
    let blob = raw
        .find_blob(
            raw.index()
                .unwrap()
                .get_path(Path::new("file.txt"), 0)
                .unwrap()
                .id,
        )
        .unwrap();
    assert_eq!(blob.content(), b"first\r\nnew\xff\r\nlast");
    let patch = fixture.patch(PatchRequest::Index("file.txt".into()));
    fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Unstage)
        .unwrap();
    raw.index().unwrap().read(true).unwrap();
    let blob = raw
        .find_blob(
            raw.index()
                .unwrap()
                .get_path(Path::new("file.txt"), 0)
                .unwrap()
                .id,
        )
        .unwrap();
    assert_eq!(blob.content(), b"first\r\nold\r\nlast");
    let patch = fixture.worktree();
    fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Discard)
        .unwrap();
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        b"first\r\nold\r\nlast"
    );
}

#[test]
fn test_selected_lines_in_later_hunks_recalculate_offsets() {
    let base = Fixture::base();
    let changed = base
        .replace("line 3\n", "inserted1\ninserted2\n")
        .replace("line 30\n", "last1\nlast2\n");
    for direction in [
        PatchDirection::Stage,
        PatchDirection::Unstage,
        PatchDirection::Discard,
    ] {
        let fixture = prepare(base.as_bytes(), changed.as_bytes());
        if direction == PatchDirection::Unstage {
            fixture.repo.stage(Path::new("file.txt")).unwrap();
        }
        let patch = fixture.patch(if direction == PatchDirection::Unstage {
            PatchRequest::Index("file.txt".into())
        } else {
            PatchRequest::Worktree("file.txt".into())
        });
        let ids = [
            select(&patch, PatchLineKind::Added, b"inserted2\n"),
            select(&patch, PatchLineKind::Added, b"last2\n"),
        ];
        fixture.repo.apply_lines(&patch, &ids, direction).unwrap();
        let actual = if direction == PatchDirection::Discard {
            std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap()
        } else {
            fixture.git_ok(&["show", ":file.txt"])
        };
        let expected = if direction == PatchDirection::Stage {
            base.replace("line 3\n", "line 3\ninserted2\n")
                .replace("line 30\n", "line 30\nlast2\n")
        } else {
            changed.replace("inserted2\n", "").replace("last2\n", "")
        };
        assert_eq!(actual, expected, "{direction:?}");
    }
}

#[test]
fn test_lines_reject_stale_context_invalid_selection_and_wrong_direction_without_writing() {
    let fixture = prepare(b"one\ntwo\n", b"one\nTWO\n");
    let mut patch = fixture.worktree();
    let id = patch.line_ids()[0];
    assert!(fixture
        .repo
        .apply_lines(&patch, &[], PatchDirection::Stage)
        .is_err());
    assert!(fixture
        .repo
        .apply_lines(&patch, &[id, id], PatchDirection::Stage)
        .is_err());
    assert!(fixture
        .repo
        .apply_lines(
            &patch,
            &[LineId {
                line: usize::MAX,
                ..id
            }],
            PatchDirection::Stage
        )
        .is_err());
    assert!(fixture
        .repo
        .apply_lines(&patch, &[id], PatchDirection::Unstage)
        .is_err());
    patch.truncated = true;
    assert!(fixture
        .repo
        .apply_lines(&patch, &[id], PatchDirection::Stage)
        .is_err());
    patch.truncated = false;
    fixture.write("file.txt", "changed externally\n");
    for direction in [PatchDirection::Stage, PatchDirection::Discard] {
        assert_eq!(
            fixture
                .repo
                .apply_lines(&patch, &[id], direction)
                .unwrap_err()
                .category(),
            GitErrorCategory::StalePatch
        );
    }
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), "one\ntwo\n");
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        b"changed externally\n"
    );
}

#[test]
fn test_partial_selection_of_changed_eof_without_newline() {
    let fixture = prepare(b"head\nold", b"head\nnew");
    let patch = fixture.worktree();
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Removed, b"old")],
            PatchDirection::Stage,
        )
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), "head\n");
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        b"head\nnew"
    );

    let fixture = Fixture::new();
    fixture.write("eof.txt", "first\nlast");
    let patch = fixture.patch(PatchRequest::Untracked("eof.txt".into()));
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Added, b"last")],
            PatchDirection::Stage,
        )
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":eof.txt"]), "last");
    fixture.repo.stage(Path::new("eof.txt")).unwrap();
    let patch = fixture.patch(PatchRequest::Index("eof.txt".into()));
    fixture
        .repo
        .apply_lines(
            &patch,
            &[select(&patch, PatchLineKind::Added, b"first\n")],
            PatchDirection::Unstage,
        )
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":eof.txt"]), "last");
    assert_eq!(
        std::fs::read(fixture.dir.path().join("eof.txt")).unwrap(),
        b"first\nlast"
    );
}

#[test]
fn test_local_hunk_discard_and_mixed_file_selections_are_validated_atomically() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    fixture
        .repo
        .apply_hunks(
            &patch,
            &[HunkId { file: 0, hunk: 0 }],
            PatchDirection::Discard,
        )
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::base().replace("line 30\n", "second change\n")
    );
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::base());
    fixture.repo.stage(Path::new("file.txt")).unwrap();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.patch(PatchRequest::Directory("".into()));
    assert!(patch
        .files
        .iter()
        .any(|file| file.source == PatchSource::Index));
    assert!(patch
        .files
        .iter()
        .any(|file| file.source == PatchSource::Worktree));
    let before_index = fixture.git_ok(&["show", ":file.txt"]);
    assert!(fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Stage)
        .is_err());
    assert!(fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Discard)
        .is_err());
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), before_index);
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
}

#[test]
fn test_multi_file_line_selection_rejects_any_stale_file_before_writing() {
    let fixture = Fixture::new();
    fixture.write("other.txt", "other old\n");
    fixture.git_ok(&["add", "--all"]);
    fixture.git_ok(&["commit", "-m", "other base"]);
    fixture.write("file.txt", Fixture::changed());
    fixture.write("other.txt", "other new\n");
    let patch = fixture.patch(PatchRequest::Directory("".into()));
    fixture.write("other.txt", "external\n");
    assert!(fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Stage)
        .is_err());
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::base());
    assert_eq!(fixture.git_ok(&["show", ":other.txt"]), "other old\n");
    let patch = fixture.patch(PatchRequest::Directory("".into()));
    fixture
        .repo
        .apply_lines(&patch, &patch.line_ids(), PatchDirection::Stage)
        .unwrap();
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::changed());
    assert_eq!(fixture.git_ok(&["show", ":other.txt"]), "external\n");
}

#[test]
fn test_line_operations_with_git_crlf_conversion() {
    let fixture = Fixture::new();
    let raw = git2::Repository::open(fixture.dir.path()).unwrap();
    raw.config()
        .unwrap()
        .set_bool("core.autocrlf", true)
        .unwrap();
    let changed = Fixture::changed().replace('\n', "\r\n");
    fixture.write("file.txt", &changed);
    let patch = fixture.worktree();
    let ids: Vec<_> = patch
        .line_ids()
        .into_iter()
        .filter(|id| id.hunk == 0)
        .collect();
    fixture
        .repo
        .apply_lines(&patch, &ids, PatchDirection::Discard)
        .unwrap();
    let remaining = std::fs::read(fixture.dir.path().join("file.txt")).unwrap();
    assert_eq!(
        remaining,
        Fixture::base()
            .replace("line 30\n", "second change\n")
            .replace('\n', "\r\n")
            .as_bytes()
    );
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::base());
}
