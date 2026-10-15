use super::*;

#[test]
fn test_keybindings_help_preserves_diff_focus_range_and_git_state() {
    use crate::app::help::HelpGroup;
    let mut fixture = Fixture::new();
    press(&mut fixture.app, KeyCode::Enter);
    press(&mut fixture.app, KeyCode::Char('a'));
    press(&mut fixture.app, KeyCode::Char('v'));
    press(&mut fixture.app, KeyCode::Char('j'));
    let interaction = format!("{:?}", fixture.app.diff.interaction());
    let file = fixture.app.files.view().panel.selected();
    let scroll = fixture.app.diff.scroll();
    let index = fixture.index();
    press(&mut fixture.app, KeyCode::Char('?'));
    assert_eq!(fixture.app.input.mode, Some(InputMode::Keybindings));
    assert_eq!(fixture.app.input.help.context, "Diff / Lines");
    let entries = &fixture.app.input.help.entries;
    let stage = entries
        .iter()
        .find(|entry| entry.name == "diff stage")
        .unwrap();
    assert_eq!(stage.group, HelpGroup::Local);
    assert_eq!(stage.keys, ["Space"]);
    assert!(entries
        .iter()
        .any(|entry| entry.name == "hunk next" && entry.keys.contains(&"l".into())));
    assert!(entries.iter().all(|entry| entry.name != "stage all"));
    for code in [
        KeyCode::Char('j'),
        KeyCode::Char('a'),
        KeyCode::Char('d'),
        KeyCode::Tab,
        KeyCode::Esc,
    ] {
        press(&mut fixture.app, code);
    }
    assert!(fixture.app.input.mode.is_none());
    assert_eq!(format!("{:?}", fixture.app.diff.interaction()), interaction);
    assert_eq!(fixture.app.files.view().panel.selected(), file);
    assert_eq!(fixture.app.diff.scroll(), scroll);
    assert_eq!(fixture.index(), index);
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );

    press(&mut fixture.app, KeyCode::Char('a'));
    press(&mut fixture.app, KeyCode::Char('?'));
    assert_eq!(fixture.app.input.help.context, "Diff / Hunks");
    let entries = &fixture.app.input.help.entries;
    assert!(entries
        .iter()
        .any(|entry| entry.name == "diff line mode" && entry.description == "Switch to line mode"));
    press(&mut fixture.app, KeyCode::Char('?'));
    assert!(fixture.app.diff.interaction().focused);
    assert!(!fixture.app.diff.interaction().line_mode);

    // The staged side shows unstage descriptions, using the same effective key.
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Char('?'));
    let entry = fixture
        .app
        .input
        .help
        .entries
        .iter()
        .find(|entry| entry.name == "diff stage")
        .unwrap();
    assert_eq!(entry.description, "Unstage selected lines / hunks");
}

#[test]
fn test_enter_during_pending_diff_load_is_not_lost() {
    let mut fixture = Fixture::new();
    fixture.app.schedule_diff_reload();
    assert!(fixture.app.diff.is_loading());
    press(&mut fixture.app, KeyCode::Enter);
    assert!(fixture.app.diff.interaction().focused);
    refresh(&mut fixture.app);
    assert!(fixture.app.diff.interaction().focused);
    assert!(fixture.app.diff.interaction().line_cursor.is_some());
    assert!(help(&fixture.app, "diff stage").is_some());
}

#[test]
fn test_enter_focuses_hunks_and_space_toggles_same_hunk_without_switching_side() {
    let mut fixture = Fixture::new();
    let selection = fixture.app.files.view().panel.selected();
    press(&mut fixture.app, KeyCode::Enter);
    assert!(fixture.app.diff.interaction().focused);
    assert!(!fixture.app.diff.interaction().line_mode);
    assert_eq!(fixture.app.files.view().panel.selected(), selection);
    assert!(help(&fixture.app, "diff stage")
        .unwrap()
        .keys
        .contains(&"Space".into()));
    assert!(help(&fixture.app, "hunk stage").unwrap().keys.is_empty());
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(
        fixture.index(),
        Fixture::base()
            .replace("line 3\n", "first change\n")
            .as_bytes()
    );
    assert_eq!(fixture.app.diff.interaction().side, Some(DiffSide::Staged));
    assert_eq!(help(&fixture.app, "diff stage").unwrap().label, "Unstage");
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert_eq!(
        fixture.app.diff.interaction().side,
        Some(DiffSide::Unstaged)
    );
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 0);
    assert_eq!(help(&fixture.app, "diff stage").unwrap().label, "Stage");
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
    press(&mut fixture.app, KeyCode::Esc);
    assert!(!fixture.app.diff.interaction().focused);
}

#[test]
fn test_tab_cycles_side_panels_and_never_enters_diff() {
    let mut fixture = Fixture::new();
    press(&mut fixture.app, KeyCode::Tab);
    assert_eq!(fixture.app.ui.active_panel, SidePanel::LocalBranches);
    assert!(!fixture.app.diff.interaction().focused);
    press(&mut fixture.app, KeyCode::BackTab);
    assert_eq!(fixture.app.ui.active_panel, SidePanel::Files);
    assert!(!fixture.app.diff.interaction().focused);
}

#[test]
fn test_enter_on_directory_toggles_tree_and_zero_can_focus_directory_diff() {
    let mut fixture = directory_fixture();
    let node = fixture.app.file_selection().selected_tree_node().unwrap();
    assert!(node.is_dir);
    let expanded = node.is_expanded;
    assert!(matches!(
        map_test_key(
            &fixture.app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        ),
        Some(DomainAction::ToggleDir)
    ));
    press(&mut fixture.app, KeyCode::Enter);
    refresh(&mut fixture.app);
    assert!(!fixture.app.diff.interaction().focused);
    assert_ne!(
        fixture
            .app
            .file_selection()
            .selected_tree_node()
            .unwrap()
            .is_expanded,
        expanded
    );
    press(&mut fixture.app, KeyCode::Char('0'));
    assert!(fixture.app.diff.interaction().focused);
}

#[test]
fn test_diff_hjkl_ranges_and_escape_follow_lazygit_context() {
    let mut fixture = Fixture::new();
    press(&mut fixture.app, KeyCode::Enter);
    press(&mut fixture.app, KeyCode::Char('a'));
    let first = fixture.app.diff.interaction().line_cursor.unwrap();
    press(&mut fixture.app, KeyCode::Char('j'));
    assert_eq!(
        fixture.app.diff.interaction().line_cursor.unwrap().hunk,
        first.hunk
    );
    press(&mut fixture.app, KeyCode::Char('l'));
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
    assert_eq!(fixture.app.ui.active_panel, SidePanel::Files);
    press(&mut fixture.app, KeyCode::Left);
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 0);
    press(&mut fixture.app, KeyCode::Char('v'));
    press(&mut fixture.app, KeyCode::Down);
    assert_eq!(fixture.app.hunk_view().marked_lines().len(), 2);
    press(&mut fixture.app, KeyCode::Esc);
    assert!(!fixture.app.diff.interaction().focused);
    assert!(!fixture.app.diff.interaction().has_selection());
    press(&mut fixture.app, KeyCode::Enter);
    press(&mut fixture.app, KeyCode::Char('a'));
    assert!(!fixture.app.diff.interaction().line_mode);
    press(&mut fixture.app, KeyCode::Char('v'));
    press(&mut fixture.app, KeyCode::Right);
    assert_eq!(fixture.app.hunk_view().marked_hunks().len(), 2);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::changed().as_bytes());
}

#[test]
fn test_next_previous_file_preserve_diff_focus_and_skip_directory_rows() {
    let mut fixture = directory_fixture();
    load_file(&mut fixture, "dir/a.txt");
    press(&mut fixture.app, KeyCode::Enter);
    press(&mut fixture.app, KeyCode::Char('n'));
    refresh(&mut fixture.app);
    assert_eq!(
        fixture
            .app
            .file_selection()
            .selected_tree_node()
            .unwrap()
            .path,
        Path::new("dir/b.txt")
    );
    assert!(fixture.app.diff.interaction().focused);
    press(&mut fixture.app, KeyCode::Char('N'));
    refresh(&mut fixture.app);
    assert_eq!(
        fixture
            .app
            .file_selection()
            .selected_tree_node()
            .unwrap()
            .path,
        Path::new("dir/a.txt")
    );
    assert!(fixture.app.diff.interaction().focused);
    assert!(
        matches!(map_test_key(&fixture.app, KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)), Some(DomainAction::EditFile(path)) if path == Path::new("dir/a.txt"))
    );
    assert!(matches!(
        map_test_key(
            &fixture.app,
            KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE)
        ),
        Some(DomainAction::StartCommitInput)
    ));
}

#[test]
fn test_space_is_disabled_for_read_only_or_pending_diff_and_modal_input_still_wins() {
    let mut fixture = Fixture::new();
    press(&mut fixture.app, KeyCode::Enter);
    fixture
        .app
        .apply_repository_action(&DomainAction::ApplySelectedHunks(PatchDirection::Stage));
    assert!(map_test_key(
        &fixture.app,
        KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)
    )
    .is_none());
    fixture.app.start_commit_editor();
    assert!(matches!(
        map_test_key(
            &fixture.app,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
        ),
        Some(DomainAction::InputTab)
    ));
    assert!(matches!(
        map_test_key(
            &fixture.app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
        ),
        Some(DomainAction::InputEnter)
    ));
    assert!(matches!(
        map_test_key(
            &fixture.app,
            KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)
        ),
        Some(DomainAction::InputChar(' '))
    ));
}
