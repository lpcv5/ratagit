use super::*;
use crate::ui::components::diff::DiffViewState;
use ratatui::buffer::Buffer;

pub(super) fn draw(fixture: &Fixture, state: &mut DiffViewState, height: u16) -> Buffer {
    let snapshot = AppStateSnapshotOwned::from_app(&fixture.app);
    let view = snapshot.as_snapshot();
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(80, height)).unwrap();
    terminal
        .draw(|frame| {
            crate::ui::panels::render_diff_panel(
                frame,
                frame.area(),
                view.diff_presentation().view(),
                state,
            );
        })
        .unwrap();
    terminal.backend().buffer().clone()
}

fn row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

pub(super) fn find(buffer: &Buffer, needle: &str) -> Option<u16> {
    (1..buffer.area.height - 1).find(|y| row(buffer, *y).contains(needle))
}

#[test]
fn test_keyboard_hunk_navigation_and_index_toggle_keep_visible_content_in_place() {
    let mut fixture = Fixture::new();
    fixture.focus();
    let mut state = DiffViewState::default();
    draw(&fixture, &mut state, 12);
    press(&mut fixture.app, KeyCode::Char('j'));
    let before = draw(&fixture, &mut state, 12);
    assert!(find(&before, "line 27").is_some());
    assert!(find(&before, "line 32").is_some());
    let screen_row = find(&before, "second change").unwrap();
    for _ in 0..4 {
        press(&mut fixture.app, KeyCode::Char(' '));
        let loading = draw(&fixture, &mut state, 12);
        assert_eq!(find(&loading, "second change"), Some(screen_row));
        refresh(&mut fixture.app);
        for _ in 0..2 {
            let after = draw(&fixture, &mut state, 12);
            assert_eq!(find(&after, "second change"), Some(screen_row));
        }
    }
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
}

#[test]
fn test_staging_marked_hunks_preserves_the_current_unmarked_hunk_and_viewport() {
    let mut fixture = Fixture::new();
    fixture.focus();
    palette(&mut fixture.app, "hunk select");
    let mut state = DiffViewState::default();
    draw(&fixture, &mut state, 12);
    press(&mut fixture.app, KeyCode::Char('j'));
    let before = draw(&fixture, &mut state, 12);
    let screen_row = find(&before, "second change").unwrap();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    let after = draw(&fixture, &mut state, 12);
    assert_eq!(find(&after, "second change"), Some(screen_row));
    let index = String::from_utf8(fixture.index()).unwrap();
    assert!(index.contains("first change"));
    assert!(!index.contains("second change"));
    assert_eq!(
        fixture.app.diff.interaction().side,
        Some(DiffSide::Unstaged)
    );
}

#[test]
fn test_index_toggle_preserves_manual_viewport_inside_large_and_untracked_hunks() {
    for untracked in [false, true] {
        let mut fixture = Fixture::new();
        let name = if untracked { "new.txt" } else { "file.txt" };
        let content: String = (1..=60).map(|n| format!("changed line {n}\n")).collect();
        std::fs::write(fixture.dir.path().join(name), &content).unwrap();
        load_file(&mut fixture, name);
        fixture.focus();
        let mut state = DiffViewState::default();
        draw(&fixture, &mut state, 10);
        for _ in 0..2 {
            dispatch_test_action(&mut fixture.app, DomainAction::DiffScrollDown);
        }
        let before = draw(&fixture, &mut state, 10);
        let previous_scroll = fixture.app.diff.scroll();
        for code in [KeyCode::Char('j'), KeyCode::Char('k')] {
            press(&mut fixture.app, code);
            let clamped = draw(&fixture, &mut state, 10);
            for y in 1..9 {
                assert_eq!(row(&before, y), row(&clamped, y));
            }
        }
        for _ in 0..4 {
            press(&mut fixture.app, KeyCode::Char(' '));
            let loading = draw(&fixture, &mut state, 10);
            for y in 1..9 {
                assert_eq!(row(&before, y), row(&loading, y));
            }
            refresh(&mut fixture.app);
            for _ in 0..2 {
                let after = draw(&fixture, &mut state, 10);
                for y in 1..9 {
                    assert_eq!(row(&before, y), row(&after, y), "untracked={untracked}");
                }
            }
        }
        assert_eq!(fixture.app.diff.scroll(), previous_scroll);
        dispatch_test_action(&mut fixture.app, DomainAction::DiffScrollDown);
        let next = draw(&fixture, &mut state, 10);
        let expected = &fixture.app.diff.document().lines[previous_scroll + 10].content;
        assert!(row(&next, 1).contains(expected.trim_end()));
        assert_eq!(
            std::fs::read_to_string(fixture.dir.path().join(name)).unwrap(),
            content
        );
    }
}

#[test]
fn test_staging_adjacent_changes_keeps_the_changed_line_fixed_when_hunks_merge() {
    for identical in [false, true] {
        let mut fixture = Fixture::new();
        let path = fixture.dir.path().join("file.txt");
        let repo = Git2Repository::open(fixture.dir.path()).unwrap();
        let (staged, changed) = if identical {
            let base = Fixture::base()
                .replace("line 3\n", "marker\n")
                .replace("line 8\n", "marker\n");
            std::fs::write(&path, &base).unwrap();
            crate::git::WorktreeRepository::stage(&repo, Path::new("file.txt")).unwrap();
            crate::git::CommitRepository::commit(&repo, "identical nearby changes").unwrap();
            (
                base.replacen("marker\n", "changed marker\n", 1),
                base.replace("marker\n", "changed marker\n")
                    .replace("line 30\n", "second change\n"),
            )
        } else {
            (
                Fixture::base().replace("line 3\n", "first change\n"),
                Fixture::changed().replace("line 8\n", "nearby change\n"),
            )
        };
        std::fs::write(&path, staged).unwrap();
        crate::git::WorktreeRepository::stage(&repo, Path::new("file.txt")).unwrap();
        std::fs::write(&path, &changed).unwrap();
        load_file(&mut fixture, "file.txt");
        fixture.focus();
        let mut state = DiffViewState::default();
        let before = draw(&fixture, &mut state, 12);
        let screen_row = find(&before, "line 9").unwrap();
        press(&mut fixture.app, KeyCode::Char(' '));
        let loading = draw(&fixture, &mut state, 12);
        assert_eq!(find(&loading, "line 9"), Some(screen_row));
        refresh(&mut fixture.app);
        let patch = fixture.app.diff.document().patch.as_ref().unwrap();
        assert_eq!(patch.hunk_ids().len(), 2);
        for _ in 0..3 {
            let after = draw(&fixture, &mut state, 12);
            assert_eq!(
                find(&after, "line 9"),
                Some(screen_row),
                "identical={identical}"
            );
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), changed);
    }
}
