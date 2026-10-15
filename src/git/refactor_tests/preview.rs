use super::*;

fn presentation(patch: &Patch) -> Vec<(DiffLine, Option<u32>, Option<u32>)> {
    patch
        .display_rows()
        .into_iter()
        .map(|row| (row.line, row.metadata.old_line, row.metadata.new_line))
        .collect()
}

#[test]
fn test_partial_staging_preserves_native_hunks_context_and_line_numbers() {
    let base = Fixture::base();
    let changes = [
        base.replace("line 10\n", "replacement\n")
            .replace("line 15\n", "helper\nline 15\n"),
        base.replace("line 3\n", "insert one\ninsert two\nline 3\n")
            .replace("line 10\n", ""),
        base.replace("line 3\nline 4\n", "")
            .replace("line 30\n", "replacement\n"),
        base.replace("line 1\n", "first\nline 1\n")
            .replace("line 5\n", "replacement\n"),
        base.replace("line 1\n", "")
            .replace("line 5\n", "replacement\n"),
        base.replace("line 10\n", "same\n")
            .replace("line 15\n", "same\n"),
    ];
    for content in changes {
        for selected in 0..2 {
            let fixture = Fixture::new();
            fixture.write("file.txt", &content);
            let request = PatchRequest::File("file.txt".into());
            let before = fixture.patch(request.clone());
            let expected = presentation(&before);
            let blocks = before.change_ids();
            assert_eq!(blocks.len(), 2);
            fixture
                .repo
                .apply_lines(
                    &before,
                    &before.change_lines(&[blocks[selected]]).unwrap(),
                    PatchDirection::Stage,
                )
                .unwrap();
            let after = fixture.patch(request.clone());
            assert!(
                after.preview.is_some(),
                "missing native preview for {content:?}, edit {selected}"
            );
            assert_eq!(presentation(&after), expected);
            assert_eq!(after.change_ids().len(), 2);
            let block = after.change_ids()[selected];
            assert_eq!(after.files[block.file].source, PatchSource::Index);
            // The displayed native header must not replace the raw source IDs
            // required for the inverse operation.
            fixture
                .repo
                .apply_lines(
                    &after,
                    &after.change_lines(&[block]).unwrap(),
                    PatchDirection::Unstage,
                )
                .unwrap();
            assert_eq!(presentation(&fixture.patch(request)), expected);
            assert_eq!(fixture.git_ok(&["show", ":file.txt"]), base);
            assert_eq!(
                std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
                content
            );
        }
    }
}

#[test]
fn test_mixed_native_preview_matches_git_diff_head_and_directory_layout() {
    let fixture = Fixture::new();
    fixture.write(
        "file.txt",
        Fixture::base()
            .replace("line 10\n", "replacement\n")
            .replace("line 15\n", "helper\nline 15\n"),
    );
    let before = fixture.worktree();
    let block = before.change_ids()[0];
    fixture
        .repo
        .apply_lines(
            &before,
            &before.change_lines(&[block]).unwrap(),
            PatchDirection::Stage,
        )
        .unwrap();
    for request in [
        PatchRequest::File("file.txt".into()),
        PatchRequest::Directory("".into()),
    ] {
        let patch = fixture.patch(request);
        let rows = patch.display_rows();
        assert!(patch.preview.is_some());
        assert_eq!(
            rows.iter()
                .filter(|row| row.line.content.starts_with("@@"))
                .count(),
            1
        );
        let body = rows
            .iter()
            .skip_while(|row| !row.line.content.starts_with("@@"))
            .map(|row| {
                format!(
                    "{}{}",
                    match row.line.kind {
                        DiffLineKind::Added => "+",
                        DiffLineKind::Removed => "-",
                        DiffLineKind::Context => " ",
                        DiffLineKind::Header => "",
                    },
                    row.line.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let native = fixture.git_ok(&[
            "diff",
            "--no-color",
            "--no-ext-diff",
            "--no-textconv",
            "HEAD",
            "--",
            "file.txt",
        ]);
        assert_eq!(
            body,
            native[native.find("@@").unwrap()..].trim_end_matches('\n')
        );
        for id in patch.line_ids() {
            let row = patch.line_row(id).unwrap();
            assert_eq!(rows[row].metadata.id, Some(id));
        }
    }
}

#[test]
fn test_overlapping_sources_keep_both_reviewed_versions_operable() {
    let fixture = Fixture::new();
    fixture.write(
        "file.txt",
        Fixture::base().replace("line 10\n", "intermediate\n"),
    );
    fixture.repo.stage(Path::new("file.txt")).unwrap();
    fixture.write(
        "file.txt",
        Fixture::base().replace("line 10\n", "final version\n"),
    );
    let patch = fixture.patch(PatchRequest::File("file.txt".into()));
    assert!(patch.preview.is_none());
    assert_eq!(patch.change_ids().len(), 2);
    assert!(patch
        .display_lines()
        .iter()
        .any(|line| line.content == "intermediate"));
    for source in [PatchSource::Index, PatchSource::Worktree] {
        let id = patch
            .change_ids()
            .into_iter()
            .find(|id| patch.files[id.file].source == source)
            .unwrap();
        let direction = source.direction().unwrap();
        patch
            .validate_lines(&patch.change_lines(&[id]).unwrap(), direction)
            .unwrap();
    }
}
