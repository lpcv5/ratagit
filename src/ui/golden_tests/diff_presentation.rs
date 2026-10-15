use super::*;
use crate::ui::theme::UiTheme;

fn row_text(ui: &RenderedUi, y: u16) -> String {
    (0..ui.buffer.area.width)
        .map(|x| ui.buffer[(x, y)].symbol())
        .collect()
}

#[test]
fn test_diff_switch_keeps_numbers_and_scrolled_preview_until_replacement_is_ready() {
    let mut fixture = Fixture::populated();
    let mut patch = patch();
    let hunk = &mut patch.files[0].hunks[0];
    hunk.new_start = 100;
    hunk.new_lines = 40;
    hunk.lines = (100..140)
        .map(|number| PatchLine {
            kind: PatchLineKind::Added,
            old_line: None,
            new_line: Some(number),
            content: format!("old preview line {number}\n").into_bytes(),
        })
        .collect();
    fixture
        .app
        .diff
        .set_test_document(crate::git::patch::DiffDocument {
            lines: patch.display_lines().into(),
            patch: Some(Arc::new(patch)),
        });
    for _ in 0..2 {
        dispatch_test_action(&mut fixture.app, DomainAction::DiffScrollDown);
    }
    let mut viewport = Default::default();
    let before = diff_with_viewport(&fixture.snapshot(), 80, 10, &mut viewport);
    assert!(row_text(&before, 1).contains("old preview line 117"));

    std::fs::write(fixture._dir.path().join("notes.txt"), "replacement text\n").unwrap();
    for _ in 0..3 {
        dispatch_test_action(&mut fixture.app, DomainAction::ListDown);
    }
    assert_eq!(
        fixture.app.files.view().tree_nodes[fixture.app.files.view().panel.selected().unwrap()]
            .path,
        std::path::PathBuf::from("notes.txt")
    );
    for phase in 0..2 {
        if phase == 1 {
            fixture.app.flush_pending_diff_reload();
        }
        let snapshot = AppStateSnapshotOwned::from_app(&fixture.app);
        assert!(!snapshot.diff_ready);
        assert!(!snapshot.diff_loading);
        assert!(snapshot.current_patch.is_none());
        assert!(snapshot.retained_diff_patch.is_some());
        let loading = diff_with_viewport(&snapshot, 80, 10, &mut viewport);
        // Every content cell, including its style and gutter, must remain stable
        // through both the debounce interval and the in-flight Git read.
        for y in 1..9 {
            for x in 0..80 {
                assert_eq!(
                    before.buffer[(x, y)],
                    loading.buffer[(x, y)],
                    "phase={phase}, x={x}, y={y}"
                );
            }
        }
        assert!(row_text(&loading, 9).contains("Loading diff..."));
    }
    crate::flux::test_runtime::finish_background(&mut fixture.app);
    let snapshot = AppStateSnapshotOwned::from_app(&fixture.app);
    assert!(snapshot.diff_ready);
    assert!(snapshot.retained_diff_patch.is_none());
    let after = diff_with_viewport(&snapshot, 80, 10, &mut viewport);
    assert!(row_text(&after, 0).contains("notes.txt"));
    assert!((1..9).any(|y| row_text(&after, y).contains("1 │ + replacement text")));
    assert!((1..9).all(|y| !row_text(&after, y).contains("old preview line")));
}

#[test]
fn test_diff_numbers_and_content_stay_aligned_across_focus_and_modes() {
    let mut fixture = Fixture::populated();
    let preview = diff(&fixture.snapshot());
    assert!(row_text(&preview, 4).contains("1 1 │   # Ratagit"));
    assert!(row_text(&preview, 5).contains("2   │ - Old description"));
    assert!(row_text(&preview, 6).contains("  2 │ + Tested terminal UI"));
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffFocus);
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffLineMode);
    let lines = diff(&fixture.snapshot());
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffLineMode);
    let hunks = diff(&fixture.snapshot());
    for y in 4..=6 {
        for x in 4..79 {
            assert_eq!(
                preview.buffer[(x, y)].symbol(),
                lines.buffer[(x, y)].symbol()
            );
            assert_eq!(lines.buffer[(x, y)].symbol(), hunks.buffer[(x, y)].symbol());
        }
    }
    assert!(row_text(&hunks, 0).contains("Diff · README.md"));
    assert!(row_text(&hunks, 0).contains("+1 -1"));
    assert!(row_text(&hunks, 7).contains("Hunk 1/1"));
}

#[test]
fn test_diff_range_and_cursor_preserve_change_colors() {
    let mut fixture = Fixture::populated();
    for action in [
        DomainAction::ToggleDiffFocus,
        DomainAction::ToggleDiffLineMode,
        DomainAction::ToggleDiffLineRange,
        DomainAction::DiffNextLine,
    ] {
        dispatch_test_action(&mut fixture.app, action);
    }
    let ui = diff(&fixture.snapshot());
    let theme = UiTheme::default();
    assert_eq!(ui.buffer[(20, 5)].bg, theme.diff_removed_selected_bg);
    assert_eq!(ui.buffer[(20, 5)].fg, theme.error);
    assert_eq!(ui.buffer[(20, 6)].bg, theme.diff_added_cursor_bg);
    assert_eq!(ui.buffer[(20, 6)].fg, theme.success);
    assert_eq!(ui.buffer[(2, 5)].symbol(), "┃");
    assert_eq!(ui.buffer[(2, 6)].symbol(), "┃");
    assert_eq!(ui.buffer[(1, 5)].symbol(), " ");
    assert_eq!(ui.buffer[(1, 6)].symbol(), ">");
    assert_eq!(ui.buffer[(78, 5)].bg, theme.diff_removed_selected_bg);
    assert!(row_text(&ui, 7).contains("2 selected | Range"));
    assert_golden("diff_line_range_compact", &ui);
}

#[test]
fn test_diff_hunk_selection_has_one_cursor_and_highlights_only_changed_rows() {
    let mut fixture = Fixture::populated();
    let preview = diff(&fixture.snapshot());
    for action in [
        DomainAction::ToggleDiffFocus,
        DomainAction::ToggleHunkSelection,
    ] {
        dispatch_test_action(&mut fixture.app, action);
    }
    let ui = diff(&fixture.snapshot());
    assert_eq!(ui.buffer[(1, 5)].symbol(), ">");
    for y in 3..=6 {
        assert_eq!(ui.buffer[(2, y)].symbol(), if y >= 5 { "┃" } else { " " });
        if y != 5 {
            assert_eq!(ui.buffer[(1, y)].symbol(), " ");
        }
    }
    let theme = UiTheme::default();
    // Hunk focus and range tint only changed rows. Headers and context keep
    // their preview backgrounds, while the header arrow identifies the hunk.
    for y in [3, 4] {
        assert_eq!(ui.buffer[(20, y)].bg, preview.buffer[(20, y)].bg);
        assert_eq!(ui.buffer[(78, y)].bg, preview.buffer[(78, y)].bg);
    }
    assert_eq!(ui.buffer[(20, 5)].bg, theme.diff_removed_cursor_bg);
    assert_eq!(ui.buffer[(20, 6)].bg, theme.diff_added_cursor_bg);
}

#[test]
fn test_diff_large_line_numbers_multiple_files_and_no_newline_golden() {
    let mut fixture = Fixture::populated();
    let mut patch = patch();
    let hunk = &mut patch.files[0].hunks[0];
    hunk.old_start = 999;
    hunk.new_start = 999;
    hunk.header = b"@@ -999,2 +999,2 @@\n".to_vec();
    hunk.lines[0].old_line = Some(999);
    hunk.lines[0].new_line = Some(999);
    hunk.lines[1].old_line = Some(1000);
    hunk.lines[2].new_line = Some(1000);
    hunk.lines[2].content = b"\tTested terminal UI\r\n".to_vec();
    hunk.lines.push(PatchLine {
        kind: PatchLineKind::NoNewline,
        old_line: None,
        new_line: None,
        content: b"\\ No newline at end of file\n".to_vec(),
    });
    let mut second = super::patch().files.remove(0);
    second.source = PatchSource::Index;
    second.old.path = Some("src/main.rs".into());
    second.new.path = Some("src/main.rs".into());
    patch.files.push(second);
    fixture
        .app
        .diff
        .set_test_document(crate::git::patch::DiffDocument {
            lines: patch.display_lines().into(),
            patch: Some(Arc::new(patch)),
        });
    let snapshot = fixture.snapshot();
    let ui = diff_sized(&snapshot, 80, 16);
    assert!(row_text(&ui, 4).contains(" 999  999 │   # Ratagit"));
    assert!(row_text(&ui, 5).contains("1000      │ - Old description"));
    assert!(row_text(&ui, 6).contains("     1000 │ +     Tested terminal UI"));
    assert!(row_text(&ui, 7).contains("          │   \\ No newline"));
    assert!(row_text(&ui, 11).contains("   1    1 │   # Ratagit"));
    assert!(row_text(&ui, 10).contains("@@ -1,2 +1,2 @@"));
    assert_golden("diff_numbered_files", &ui);
    let narrow = diff_sized(&snapshot, 25, 16);
    assert!(row_text(&narrow, 4).starts_with("│     # Ratagit"));
    assert_golden("diff_numbered_narrow", &narrow);
    for width in [0, 1, 2, 8, 24, 32, 80] {
        for height in [0, 1, 2, 3] {
            diff_sized(&snapshot, width, height);
        }
    }
}

#[test]
fn test_combined_staged_and_unstaged_hunks_share_one_file_header_golden() {
    let mut fixture = Fixture::populated();
    let mut patch = patch();
    patch.request = PatchRequest::File("README.md".into());
    let mut staged = patch.files[0].clone();
    staged.source = PatchSource::Index;
    staged.hunks[0].new_start = 20;
    staged.hunks[0].old_start = 20;
    staged.hunks[0].header = b"@@ -20,2 +20,2 @@\n".to_vec();
    for line in &mut staged.hunks[0].lines {
        line.old_line = line.old_line.map(|number| number + 19);
        line.new_line = line.new_line.map(|number| number + 19);
    }
    staged.hunks[0].lines[2].content = b"Already staged change\n".to_vec();
    patch.files.insert(0, staged);
    fixture
        .app
        .diff
        .set_test_document(crate::git::patch::DiffDocument {
            lines: patch.display_lines().into(),
            patch: Some(Arc::new(patch)),
        });
    let ui = diff_sized(&fixture.snapshot(), 80, 14);
    let text: String = (0..14).map(|y| row_text(&ui, y)).collect();
    assert_eq!(text.matches("--- a/README.md").count(), 1);
    assert_eq!(text.matches("+++ b/README.md").count(), 1);
    assert!(!text.contains("Unstaged") && !text.contains("Staged"));
    assert!(!text.contains("2 files"));
    let theme = UiTheme::default();
    assert_eq!(ui.buffer[(40, 10)].bg, theme.diff_staged_added_bg);
    assert_eq!(ui.buffer[(40, 9)].bg, theme.diff_staged_removed_bg);
    assert_eq!(ui.buffer[(40, 8)].bg, theme.surface);
    assert_eq!(ui.buffer[(40, 7)].bg, theme.surface_raised);
    assert_golden("diff_combined_staged", &ui);
}
