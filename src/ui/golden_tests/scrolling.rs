use super::*;
use crate::ui::components::organisms::{
    draw_branches_panel, draw_commits_panel, draw_files_panel, draw_stash_panel,
};
use crate::ui::RenderState;
use ratatui::widgets::ListState;

fn fixture(case: usize) -> Fixture {
    let mut fixture = Fixture::populated();
    let app = &mut fixture.app;
    let tree: Vec<_> = (0..30)
        .map(|index| FileTreeNode {
            path: format!("file_{index:02}.rs").into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        })
        .collect();
    app.files.test_view_mut().tree_nodes = tree.clone().into();
    app.files.test_view_mut().panel.select(Some(0));
    app.revisions.commits.items = commits(30).into();
    app.revisions.branches.items = (0..30)
        .map(|index| BranchInfo {
            name: format!("branch_{index:02}"),
            is_current: index == 0,
        })
        .collect();
    app.revisions.stash.items = (0..30)
        .map(|index| StashInfo {
            index,
            message: format!("stash {index}"),
        })
        .collect();
    app.ui.active_panel = match case {
        0 => SidePanel::Files,
        1 | 2 => SidePanel::LocalBranches,
        3 | 4 => SidePanel::Commits,
        _ => SidePanel::Stash,
    };
    match case {
        2 => {
            let branches = &mut app.revisions.branches;
            branches.commits_subview_active = true;
            branches.commits_subview_source = Some("branch_00".into());
            branches.commits_subview.items = commits(30).into();
        }
        4 => {
            app.revisions.commits.tree_mode.active = true;
            app.revisions.commits.tree_mode.selected_source = Some("commit".into());
            app.revisions.commits.tree_mode.nodes = tree.into();
        }
        6 => {
            app.revisions.stash.tree_mode.active = true;
            app.revisions.stash.tree_mode.selected_source = Some(0);
            app.revisions.stash.tree_mode.nodes = tree.into();
        }
        _ => {}
    }
    fixture
}

fn panel(
    fixture: &Fixture,
    case: usize,
    state: &mut RenderState,
    height: u16,
) -> (RenderedUi, ListState) {
    let snapshot = crate::flux::snapshot::AppStateSnapshot::from_app(&fixture.app);
    let ui = render(70, height + 2, |frame| {
        let area = frame.area();
        match case {
            0 => draw_files_panel(frame, area, snapshot.files_panel_props(), &mut state.files),
            1 | 2 => draw_branches_panel(
                frame,
                area,
                snapshot.branches_panel_props(),
                &mut state.branches,
            ),
            3 | 4 => draw_commits_panel(
                frame,
                area,
                snapshot.commits_panel_props(),
                &mut state.commits,
            ),
            _ => draw_stash_panel(frame, area, snapshot.stash_panel_props(), &mut state.stash),
        }
    });
    let viewport = match case {
        0 => state.files,
        1 => state.branches.list,
        2 => *state.branches.commits.state(false, None),
        3 => *state.commits.state(false, None),
        4 => *state.commits.state(true, Some("commit".into())),
        5 => *state.stash.state(false, None),
        _ => *state.stash.state(true, Some("0".into())),
    };
    let selected = fixture.app.selection().selected_index();
    assert_eq!(viewport.selected(), selected, "case={case}");
    if let Some(selected) = selected {
        let row = selected - viewport.offset() + 1;
        assert_eq!(
            ui.buffer[(1, row as u16)].bg,
            crate::ui::theme::UiTheme::default().selection_bg,
            "case={case}, selected={selected}, offset={}",
            viewport.offset(),
        );
    }
    (ui, viewport)
}

fn press(fixture: &mut Fixture, key: char) {
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE),
    );
}

#[test]
fn test_jk_keeps_two_context_rows_in_every_side_panel_and_subview() {
    for case in 0..7 {
        let mut fixture = fixture(case);
        let mut state = RenderState::default();
        assert_eq!(panel(&fixture, case, &mut state, 10).1.offset(), 0);
        for selected in 1..=20 {
            press(&mut fixture, 'j');
            let (_, viewport) = panel(&fixture, case, &mut state, 10);
            if selected == 7 {
                assert_eq!(viewport.offset(), 0);
            }
            if selected == 8 {
                assert_eq!(viewport.offset(), 1);
            }
        }
        assert_eq!(panel(&fixture, case, &mut state, 10).1.offset(), 13);
        for _ in 0..5 {
            press(&mut fixture, 'k');
            assert_eq!(panel(&fixture, case, &mut state, 10).1.offset(), 13);
        }
        press(&mut fixture, 'k');
        let (ui, viewport) = panel(&fixture, case, &mut state, 10);
        assert_eq!(viewport.offset(), 12, "case={case}");
        // Repainting and reversing direction inside the safe area must not jump.
        assert_eq!(
            serialize(&panel(&fixture, case, &mut state, 10).0),
            serialize(&ui)
        );
        press(&mut fixture, 'j');
        assert_eq!(panel(&fixture, case, &mut state, 10).1.offset(), 12);
        assert_eq!(panel(&fixture, case, &mut state, 6).1.offset(), 12);
        assert_eq!(panel(&fixture, case, &mut state, 20).1.offset(), 10);
        for height in [1, 2, 3, 4, 10] {
            panel(&fixture, case, &mut state, height);
        }
        for _ in 0..30 {
            press(&mut fixture, 'k');
            panel(&fixture, case, &mut state, 10);
        }
        assert_eq!(panel(&fixture, case, &mut state, 10).1.offset(), 0);
        for _ in 0..35 {
            press(&mut fixture, 'j');
            panel(&fixture, case, &mut state, 10);
        }
        assert_eq!(panel(&fixture, case, &mut state, 10).1.offset(), 20);
    }
}

#[test]
fn test_full_layout_retains_viewport_between_published_frames() {
    let mut fixture = fixture(0);
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let mut state = RenderState::default();
    let render =
        |fixture: &mut Fixture, state: &mut RenderState, terminal: &mut Terminal<TestBackend>| {
            let snapshot = fixture.snapshot();
            draw(terminal, |frame| {
                render_layout(frame, &snapshot.as_snapshot(), state)
            });
        };
    render(&mut fixture, &mut state, &mut terminal);
    for _ in 0..20 {
        press(&mut fixture, 'j');
        render(&mut fixture, &mut state, &mut terminal);
    }
    assert!(state.files.offset() > 0);
    while state.files.selected().unwrap() > state.files.offset() + 2 {
        press(&mut fixture, 'k');
        render(&mut fixture, &mut state, &mut terminal);
    }
    let previous = state.files.offset();
    press(&mut fixture, 'k');
    render(&mut fixture, &mut state, &mut terminal);
    assert_eq!(state.files.offset(), previous - 1);
    let previous = state.files;
    render(&mut fixture, &mut state, &mut terminal);
    assert_eq!(state.files, previous);
}

#[test]
fn test_help_scrolls_up_early_and_resets_after_filter_or_close() {
    let mut fixture = Fixture::populated();
    press(&mut fixture, '?');
    let mut state = RenderState::default();
    let draw = |fixture: &Fixture, state: &mut RenderState| {
        render(100, 30, |frame| {
            render_layout(
                frame,
                &crate::flux::snapshot::AppStateSnapshot::from_app(&fixture.app),
                state,
            );
        })
    };
    draw(&fixture, &mut state);
    for _ in 0..25 {
        press(&mut fixture, 'j');
        draw(&fixture, &mut state);
    }
    let offset = state.help.list.offset();
    assert!(offset > 0);
    // Use the displayed row index: help groups add non-selectable header rows.
    while state.help.list.selected().unwrap() > state.help.list.offset() + 2 {
        press(&mut fixture, 'k');
        draw(&fixture, &mut state);
    }
    let offset = state.help.list.offset();
    press(&mut fixture, 'k');
    draw(&fixture, &mut state);
    assert!(state.help.list.offset() < offset);
    press(&mut fixture, '/');
    press(&mut fixture, 'z');
    draw(&fixture, &mut state);
    assert_eq!(state.help.list.offset(), 0);
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    draw(&fixture, &mut state);
    assert!(state.help.context.is_none());
}

fn diff_panel(fixture: &Fixture, state: &mut RenderState, height: u16) -> RenderedUi {
    let snapshot = crate::flux::snapshot::AppStateSnapshot::from_app(&fixture.app);
    render(80, height + 2, |frame| {
        render_diff_panel(
            frame,
            frame.area(),
            snapshot.diff_presentation().view(),
            &mut state.diff,
        );
    })
}

fn diff_cursor_row(ui: &RenderedUi) -> u16 {
    (1..ui.buffer.area.height - 1)
        .find(|y| ui.buffer[(1, *y)].symbol() == ">")
        .expect("diff cursor should be visible")
}

#[test]
fn test_diff_line_navigation_keeps_context_when_moving_and_reversing() {
    let mut fixture = Fixture::populated();
    let mut patch = patch();
    patch.files[0].hunks[0].lines = (0..40)
        .map(|index| PatchLine {
            kind: PatchLineKind::Added,
            content: format!("line {index}\n").into_bytes(),
            old_line: None,
            new_line: Some(index + 1),
        })
        .collect();
    fixture
        .app
        .diff
        .set_test_document(crate::git::patch::DiffDocument {
            lines: patch.display_lines().into(),
            patch: Some(Arc::new(patch)),
        });
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffFocus);
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffLineMode);
    let mut state = RenderState::default();
    diff_panel(&fixture, &mut state, 10);
    for _ in 0..20 {
        press(&mut fixture, 'j');
        assert!(diff_cursor_row(&diff_panel(&fixture, &mut state, 10)) <= 8);
    }
    assert_eq!(diff_cursor_row(&diff_panel(&fixture, &mut state, 10)), 8);
    for _ in 0..5 {
        press(&mut fixture, 'k');
        diff_panel(&fixture, &mut state, 10);
    }
    assert_eq!(diff_cursor_row(&diff_panel(&fixture, &mut state, 10)), 3);
    press(&mut fixture, 'k');
    let ui = diff_panel(&fixture, &mut state, 10);
    assert_eq!(diff_cursor_row(&ui), 3);
    assert_eq!(
        serialize(&diff_panel(&fixture, &mut state, 10)),
        serialize(&ui)
    );
    for height in [1, 2, 3, 4, 6, 20] {
        diff_cursor_row(&diff_panel(&fixture, &mut state, height));
    }
    // The UI can receive a snapshot after multiple input actions in one frame.
    for _ in 0..5 {
        press(&mut fixture, 'j');
    }
    assert!(diff_cursor_row(&diff_panel(&fixture, &mut state, 10)) <= 8);
}

#[test]
fn test_diff_hunk_navigation_keeps_header_context_in_both_directions() {
    let mut fixture = Fixture::populated();
    let mut patch = patch();
    let hunk = patch.files[0].hunks[0].clone();
    patch.files[0].hunks = vec![hunk; 12];
    fixture
        .app
        .diff
        .set_test_document(crate::git::patch::DiffDocument {
            lines: patch.display_lines().into(),
            patch: Some(Arc::new(patch)),
        });
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffFocus);
    let mut state = RenderState::default();
    diff_panel(&fixture, &mut state, 10);
    for _ in 0..6 {
        press(&mut fixture, 'j');
        assert!(diff_cursor_row(&diff_panel(&fixture, &mut state, 10)) <= 8);
    }
    press(&mut fixture, 'k');
    diff_panel(&fixture, &mut state, 10);
    press(&mut fixture, 'k');
    assert_eq!(diff_cursor_row(&diff_panel(&fixture, &mut state, 10)), 3);
    // Explicit page scroll is respected until the cursor moves again.
    dispatch_test_action(&mut fixture.app, DomainAction::DiffScrollDown);
    let manual = diff_panel(&fixture, &mut state, 10);
    assert_eq!(
        serialize(&diff_panel(&fixture, &mut state, 10)),
        serialize(&manual)
    );
    press(&mut fixture, 'j');
    assert!((3..=8).contains(&diff_cursor_row(&diff_panel(&fixture, &mut state, 10))));
}
