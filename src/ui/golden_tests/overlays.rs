use super::*;

fn text(ui: &RenderedUi) -> String {
    ui.buffer.content.iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn test_all_overlays_bound_cursors_on_resize_and_long_unicode_input() {
    let mut fixture = Fixture::populated();
    for mode in [
        InputMode::CommitAllConfirm,
        InputMode::BranchSwitchConfirm,
        InputMode::CreateBranch,
        InputMode::CommitEditor,
        InputMode::StashEditor,
        InputMode::CommandPalette,
        InputMode::Keybindings,
    ] {
        fixture.app.input.mode = Some(mode);
        fixture.app.input.buffer = "分支e\u{301}👩\u{200d}💻".repeat(30);
        fixture.app.input.commit_message_buffer = fixture.app.input.buffer.clone();
        fixture.app.input.commit_description_buffer = "中文说明\n".repeat(40);
        fixture.app.input.stash_message_buffer = fixture.app.input.buffer.clone();
        fixture.app.input.branch_switch_target = Some("feature/".repeat(25));
        for focus in [CommitFieldFocus::Message, CommitFieldFocus::Description] {
            fixture.app.input.commit_focus = focus;
            let snapshot = fixture.snapshot();
            for (width, height) in [
                (1, 1),
                (2, 2),
                (20, 6),
                (40, 10),
                (60, 18),
                (120, 40),
                (240, 80),
            ] {
                let rendered = layout(&snapshot, width, height);
                if let Some(cursor) = rendered.cursor {
                    assert!(
                        rendered.buffer.area.contains(cursor),
                        "{mode:?}: {cursor:?} in {width}x{height}"
                    );
                }
                let borrowed = render(width, height, |frame| {
                    render_layout(
                        frame,
                        &crate::flux::snapshot::AppStateSnapshot::from_app(&fixture.app),
                        &mut Default::default(),
                    )
                });
                assert_eq!(serialize(&rendered), serialize(&borrowed));
            }
        }
    }
}

#[test]
fn test_confirmation_is_compact_wraps_and_preserves_both_actions() {
    let mut fixture = Fixture::populated();
    fixture.app.input.mode = Some(InputMode::BranchSwitchConfirm);
    fixture.app.input.branch_switch_target =
        Some("feature/a-long-branch-name-with-uncommitted-work".into());
    let snapshot = fixture.snapshot();
    let narrow = layout(&snapshot, 40, 18);
    let content = text(&narrow);
    assert!(content.contains("Cancel"));
    assert!(content.contains("Stash + switch"));
    let roomy = layout(&snapshot, 240, 80);
    let top = (0..80)
        .find(|y| roomy.buffer[(84, *y)].symbol() == "╭")
        .unwrap();
    let bottom = (top..80)
        .find(|y| roomy.buffer[(84, *y)].symbol() == "╰")
        .unwrap();
    assert!(top > 25);
    assert!(
        bottom - top < 15,
        "confirmation must stay compact on a tall terminal"
    );
    assert_golden("overlay_switch_narrow", &narrow);
}

#[test]
fn test_form_validation_stays_open_and_preserves_other_fields() {
    for mode in [
        InputMode::CreateBranch,
        InputMode::StashEditor,
        InputMode::CommitEditor,
    ] {
        let mut fixture = Fixture::populated();
        fixture.app.input.mode = Some(mode);
        fixture.app.input.commit_description_buffer = "Keep this draft".into();
        fixture.app.input.stash_targets = vec!["notes.txt".into()];
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
        assert_eq!(fixture.app.input.mode, Some(mode));
        assert!(fixture.app.input.error.is_some());
        assert_eq!(
            fixture.app.input.commit_description_buffer,
            "Keep this draft"
        );
        assert_eq!(
            fixture.app.input.stash_targets,
            vec![std::path::PathBuf::from("notes.txt")]
        );
        let snapshot = fixture.snapshot();
        assert!(text(&layout(&snapshot, 80, 24)).contains("required."));
        if mode == InputMode::CreateBranch {
            assert_golden("input_branch_error", &layout(&snapshot, 80, 24));
        }
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE),
        );
        assert!(fixture.app.input.error.is_none());
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
        );
        assert!(fixture.app.input.mode.is_none());
    }
}

#[test]
fn test_palette_selected_result_scrolls_and_is_retained_in_snapshot() {
    let mut fixture = Fixture::populated();
    dispatch_test_action(&mut fixture.app, DomainAction::StartCommandPalette);
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::End, KeyModifiers::NONE),
    );
    let retained = fixture.snapshot();
    let selected = retained.as_snapshot().command_palette_props().commands
        [retained.palette_selected]
        .name
        .to_owned();
    assert!(text(&layout(&retained, 80, 18)).contains(&selected));
    assert_golden("palette_selected", &layout(&retained, 80, 18));
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
    );
    assert_eq!(fixture.app.input.palette_selected, 0);
    assert_ne!(retained.palette_selected, 0);
    assert_eq!(
        fixture
            .snapshot()
            .as_snapshot()
            .command_palette_props()
            .commands[0]
            .name,
        "quit"
    );
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert!(!fixture.app.running);
}

#[test]
fn test_palette_navigation_executes_visible_result_and_no_match_stays_open() {
    let mut fixture = Fixture::populated();
    dispatch_test_action(&mut fixture.app, DomainAction::StartCommandPalette);
    for c in "panel".chars() {
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    let candidates = fixture.app.available_commands();
    let candidates = crate::app::commands::filter_commands("panel", &candidates);
    let branch_index = candidates
        .iter()
        .position(|command| command.name == "branches")
        .unwrap();
    assert!(branch_index > 0);
    for _ in 0..branch_index {
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
        );
    }
    let snapshot = fixture.snapshot();
    let props = snapshot.as_snapshot();
    let props = props.command_palette_props();
    let expected = fixture
        .app
        .resolve_command_palette_command(props.commands[props.selected].name)
        .unwrap();
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    let DomainAction::PanelGoto(index) = expected else {
        panic!("expected a panel action")
    };
    assert_eq!(
        fixture.app.ui.active_panel,
        match index {
            1 => SidePanel::Files,
            2 => SidePanel::LocalBranches,
            3 => SidePanel::Commits,
            _ => SidePanel::Stash,
        }
    );
    dispatch_test_action(&mut fixture.app, DomainAction::StartCommandPalette);
    for c in "no-such-command".chars() {
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE),
        );
    }
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
    );
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    assert_eq!(fixture.app.input.mode, Some(InputMode::CommandPalette));
    assert_eq!(fixture.app.input.buffer, "no-such-command");
    assert_eq!(fixture.app.input.palette_selected, 0);
}
