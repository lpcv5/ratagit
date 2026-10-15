use super::*;

fn changed() -> String {
    Fixture::base()
        .replace("line 10\n", "replacement\n")
        .replace("line 15\n", "new helper\nline 15\n")
}

fn fixture() -> Fixture {
    let mut fixture = Fixture::new();
    std::fs::write(fixture.dir.path().join("file.txt"), changed()).unwrap();
    load_file(&mut fixture, "file.txt");
    let patch = fixture.app.diff.document().patch.as_ref().unwrap();
    assert_eq!(
        patch.hunk_ids().len(),
        1,
        "both edits must share one native hunk"
    );
    assert_eq!(patch.change_ids().len(), 2);
    fixture.focus();
    fixture
}

#[test]
fn test_jk_selects_separate_changes_inside_one_git_hunk() {
    let mut fixture = fixture();
    let ids = fixture
        .app
        .diff
        .document()
        .patch
        .as_ref()
        .unwrap()
        .change_ids();
    assert_eq!(fixture.app.diff.interaction().cursor, Some(ids[0]));
    assert_eq!(fixture.app.hunk_view().cursor_position(), (1, 2));
    press(&mut fixture.app, KeyCode::Char('j'));
    assert_eq!(fixture.app.diff.interaction().cursor, Some(ids[1]));
    assert_eq!(fixture.app.hunk_view().cursor_position(), (2, 2));
    assert_eq!(ids[0].hunk(), ids[1].hunk());
    press(&mut fixture.app, KeyCode::Char('k'));
    assert_eq!(fixture.app.diff.interaction().cursor, Some(ids[0]));
    assert!(fixture.app.hunk_view().can_apply(PatchDirection::Stage));
    assert!(
        fixture
            .app
            .hunk_view()
            .targets(PatchDirection::Stage)
            .is_empty(),
        "a partial edit must not use native hunk application"
    );
}

#[test]
fn test_space_toggles_each_change_without_staging_its_neighbor() {
    for second in [false, true] {
        let mut fixture = fixture();
        if second {
            press(&mut fixture.app, KeyCode::Char('j'));
        }
        let expected = if second {
            Fixture::base().replace("line 15\n", "new helper\nline 15\n")
        } else {
            Fixture::base().replace("line 10\n", "replacement\n")
        };
        for staged in [true, false, true, false] {
            press(&mut fixture.app, KeyCode::Char(' '));
            refresh(&mut fixture.app);
            assert_eq!(
                String::from_utf8(fixture.index()).unwrap(),
                if staged {
                    expected.clone()
                } else {
                    Fixture::base()
                }
            );
            assert_eq!(
                fixture.app.diff.interaction().side,
                Some(if staged {
                    DiffSide::Staged
                } else {
                    DiffSide::Unstaged
                })
            );
            assert_eq!(
                fixture.app.hunk_view().cursor_position(),
                (if second { 2 } else { 1 }, 2)
            );
            assert_eq!(
                std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
                changed()
            );
            assert_eq!(
                fixture
                    .app
                    .diff
                    .document()
                    .patch
                    .as_ref()
                    .unwrap()
                    .display_file_count(),
                1
            );
        }
    }
}

#[test]
fn test_independent_staging_and_unstaging_survive_hunk_merging() {
    let mut fixture = fixture();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Char('j'));
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), changed().as_bytes());
    let patch = fixture.app.diff.document().patch.as_ref().unwrap();
    assert_eq!(patch.hunk_ids().len(), 1);
    assert_eq!(patch.change_ids().len(), 2);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(
        String::from_utf8(fixture.index()).unwrap(),
        Fixture::base().replace("line 10\n", "replacement\n")
    );
}

#[test]
fn test_partial_change_toggle_keeps_its_screen_position() {
    use super::viewport::{draw, find};
    let mut fixture = fixture();
    let mut viewport = crate::ui::components::diff::DiffViewState::default();
    draw(&fixture, &mut viewport, 10);
    press(&mut fixture.app, KeyCode::Char('j'));
    let before = draw(&fixture, &mut viewport, 10);
    let row = find(&before, "new helper").unwrap();
    for _ in 0..4 {
        press(&mut fixture.app, KeyCode::Char(' '));
        let loading = draw(&fixture, &mut viewport, 10);
        assert_eq!(find(&loading, "new helper"), Some(row));
        refresh(&mut fixture.app);
        for _ in 0..2 {
            let after = draw(&fixture, &mut viewport, 10);
            assert_eq!(find(&after, "new helper"), Some(row));
        }
    }
}

#[test]
fn test_partial_stage_changes_only_edit_shading_in_native_display() {
    use super::viewport::{draw, find};
    use crate::ui::theme::UiTheme;
    let mut fixture = fixture();
    let mut viewport = crate::ui::components::diff::DiffViewState::default();
    let before = draw(&fixture, &mut viewport, 20);
    let changed_row = find(&before, "replacement").unwrap();
    let theme = UiTheme::default();
    assert_eq!(before[(40, changed_row)].bg, theme.diff_added_cursor_bg);
    for staged in [true, false, true, false] {
        press(&mut fixture.app, KeyCode::Char(' '));
        refresh(&mut fixture.app);
        let after = draw(&fixture, &mut viewport, 20);
        let patch = fixture.app.diff.document().patch.as_ref().unwrap();
        assert_eq!(
            patch
                .display_lines()
                .iter()
                .filter(|line| line.content.starts_with("@@"))
                .count(),
            1
        );
        for y in 1..19 {
            for x in 4..79 {
                assert_eq!(
                    before[(x, y)].symbol(),
                    after[(x, y)].symbol(),
                    "x={x}, y={y}, staged={staged}"
                );
                if ![find(&before, "line 10").unwrap(), changed_row].contains(&y) {
                    assert_eq!(
                        before[(x, y)].bg,
                        after[(x, y)].bg,
                        "x={x}, y={y}, staged={staged}"
                    );
                }
            }
        }
        assert_eq!(
            after[(40, changed_row)].bg,
            if staged {
                theme.diff_staged_added_cursor_bg
            } else {
                theme.diff_added_cursor_bg
            }
        );
        if staged {
            crate::ui::test_support::assert_golden(
                "diff_native_mixed",
                &crate::ui::test_support::RenderedUi {
                    buffer: after,
                    cursor: None,
                },
            );
        }
    }
}

#[test]
fn test_change_ranges_and_explicit_marks_apply_the_reviewed_blocks() {
    for range in [false, true] {
        let mut fixture = fixture();
        if range {
            press(&mut fixture.app, KeyCode::Char('v'));
        } else {
            palette(&mut fixture.app, "hunk select");
        }
        press(&mut fixture.app, KeyCode::Char('j'));
        assert_eq!(
            fixture.app.hunk_view().marked_hunks().len(),
            if range { 2 } else { 1 }
        );
        press(&mut fixture.app, KeyCode::Char(' '));
        refresh(&mut fixture.app);
        assert_eq!(
            String::from_utf8(fixture.index()).unwrap(),
            if range {
                changed()
            } else {
                Fixture::base().replace("line 10\n", "replacement\n")
            }
        );
        assert_eq!(fixture.app.hunk_view().cursor_position(), (2, 2));
    }
}

#[test]
fn test_discard_and_stale_partial_changes_leave_other_content_intact() {
    let mut fixture = fixture();
    press(&mut fixture.app, KeyCode::Char('j'));
    press(&mut fixture.app, KeyCode::Char('d'));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::base().replace("line 10\n", "replacement\n")
    );
    press(&mut fixture.app, KeyCode::Esc);
    std::fs::write(fixture.dir.path().join("file.txt"), changed()).unwrap();
    load_file(&mut fixture, "file.txt");
    fixture.focus();
    std::fs::write(fixture.dir.path().join("file.txt"), "external change\n").unwrap();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert!(fixture
        .app
        .command_log
        .iter()
        .any(|entry| !entry.success && entry.command.contains("patch changed")));
}
