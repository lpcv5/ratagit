use super::*;

#[test]
fn test_staged_palette_affects_only_added_and_removed_rows_in_every_hunk_state() {
    let mut patch = patch(PatchSource::Index, 1);
    let hunk = &mut patch.files[0].hunks[0];
    hunk.lines = [
        (PatchLineKind::Context, "before", Some(1), Some(1)),
        (PatchLineKind::Removed, "old", Some(2), None),
        (PatchLineKind::Added, "new", None, Some(2)),
        (PatchLineKind::Context, "after", Some(3), Some(3)),
    ]
    .into_iter()
    .map(|(kind, content, old_line, new_line)| PatchLine {
        kind,
        content: content.as_bytes().to_vec(),
        old_line,
        new_line,
    })
    .collect();
    let lines = patch.display_lines();
    let id = patch.change_ids()[0];
    let theme = UiTheme::default();
    for state in 0..3 {
        let buffer = draw(
            DiffViewModel {
                lines: &lines,
                patch: Some(&patch),
                selection: DiffSelection {
                    focused: true,
                    hunk_cursor: (state == 1).then_some(id),
                    marked_hunks: if state == 2 {
                        std::slice::from_ref(&id)
                    } else {
                        &[]
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            &theme,
            &mut DiffViewState::default(),
        );
        let added = [
            theme.diff_staged_added_bg,
            theme.diff_staged_added_cursor_bg,
            theme.diff_staged_added_selected_bg,
        ][state];
        let removed = [
            theme.diff_staged_removed_bg,
            theme.diff_staged_removed_cursor_bg,
            theme.diff_staged_removed_selected_bg,
        ][state];
        for x in 4..79 {
            assert_eq!(buffer[(x, 3)].bg, theme.surface_raised);
            assert_eq!(buffer[(x, 4)].bg, theme.surface);
            assert_eq!(buffer[(x, 5)].bg, removed);
            assert_eq!(buffer[(x, 6)].bg, added);
            assert_eq!(buffer[(x, 7)].bg, theme.surface);
        }
        assert!(!row(&buffer, 3).contains("Staged"));
        assert!(!row(&buffer, 3).contains("Unstaged"));
    }
}

#[test]
fn test_context_separates_change_highlights_and_eof_markers_are_not_write_targets() {
    let mut patch = patch(PatchSource::Worktree, 1);
    let hunk = &mut patch.files[0].hunks[0];
    hunk.old_start = 1;
    hunk.old_lines = 4;
    hunk.new_lines = 6;
    hunk.header = b"@@ -1,4 +1,6 @@\n".to_vec();
    hunk.lines = [
        (PatchLineKind::Context, "before", Some(1), Some(1)),
        (PatchLineKind::Removed, "old call", Some(2), None),
        (PatchLineKind::Added, "new call", None, Some(2)),
        (PatchLineKind::Context, "gap one", Some(3), Some(3)),
        (PatchLineKind::Context, "gap two", Some(4), Some(4)),
        (PatchLineKind::Added, "new helper one", None, Some(5)),
        (PatchLineKind::Added, "new helper two", None, Some(6)),
        (
            PatchLineKind::NoNewline,
            "\\ No newline at end of file",
            None,
            None,
        ),
    ]
    .into_iter()
    .map(|(kind, content, old_line, new_line)| PatchLine {
        kind,
        content: content.as_bytes().to_vec(),
        old_line,
        new_line,
    })
    .collect();
    let blocks = patch.change_blocks();
    assert_eq!(blocks.len(), 2);
    assert_eq!(blocks[0].lines, 1..3);
    assert_eq!(blocks[1].lines, 5..8);
    let targets = patch.change_lines(&[blocks[1].id]).unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[1].line, 6);
    let lines = patch.display_lines();
    let theme = UiTheme::default();
    let mut state = DiffViewState::default();
    let area = Rect::new(0, 0, 80, 16);
    for selected in 0..2 {
        let mut buffer = Buffer::empty(area);
        StatefulWidget::render(
            DiffView::new(
                DiffViewModel {
                    lines: &lines,
                    patch: Some(&patch),
                    selection: DiffSelection {
                        focused: true,
                        hunk_cursor: Some(blocks[selected].id),
                        position: DiffPosition {
                            current: selected + 1,
                            total: 2,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                &theme,
            ),
            area,
            &mut buffer,
            &mut state,
        );
        for (index, block) in blocks.iter().enumerate() {
            let y = block.range.start as u16 + 1;
            assert_eq!(
                buffer[(1, y)].symbol(),
                if index == selected { ">" } else { " " }
            );
            let expected = if index == 0 {
                if index == selected {
                    theme.diff_removed_cursor_bg
                } else {
                    theme.diff_removed_bg
                }
            } else if index == selected {
                theme.diff_added_cursor_bg
            } else {
                theme.diff_added_bg
            };
            assert_eq!(buffer[(30, y)].bg, expected);
        }
        assert_eq!(buffer[(30, 7)].bg, theme.surface);
        assert_eq!(buffer[(30, 8)].bg, theme.surface);
        crate::ui::test_support::assert_golden(
            if selected == 0 {
                "diff_change_first"
            } else {
                "diff_change_second"
            },
            &crate::ui::test_support::RenderedUi {
                buffer,
                cursor: None,
            },
        );
    }
}
