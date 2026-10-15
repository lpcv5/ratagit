use super::*;
use crate::app::{dispatch_test_key, map_test_key, App, SidePanel};
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crate::flux::action::Action;
use crate::flux::dispatcher::Dispatcher;
use crate::flux::snapshot::{AppStateSnapshot, AppStateSnapshotOwned};
use crate::flux::stores::test_support::MockRepo;

fn app_with_keymap(mut keymap: Keymap) -> App {
    keymap.rebuild_indexes();
    let mut app = App::from_repo_with_keymap(Box::new(MockRepo), keymap).unwrap();
    app.files.test_view_mut().tree_nodes = vec![FileTreeNode {
        path: "new.txt".into(),
        status: FileTreeNodeStatus::Untracked,
        depth: 0,
        is_dir: false,
        is_expanded: false,
    }]
    .into();
    app.files.test_view_mut().panel.select(Some(0));
    app
}

fn press(app: &mut App, code: KeyCode) {
    dispatch_test_key(app, KeyEvent::new(code, KeyModifiers::NONE));
}

fn open(app: &mut App) {
    press(app, KeyCode::Char('?'));
    assert_eq!(app.input.mode, Some(InputMode::Keybindings));
}

#[test]
fn test_help_uses_effective_contextual_keys_and_all_three_groups() {
    let mut app = app_with_keymap(Keymap::default());
    open(&mut app);
    let entries = &app.input.help.entries;
    let entry = |name| entries.iter().find(|entry| entry.name == name).unwrap();
    assert_eq!(entry("stage").group, HelpGroup::Local);
    assert_eq!(entry("stage").keys, ["Space"]);
    assert_eq!(entry("help").group, HelpGroup::Global);
    assert_eq!(entry("help").keys, ["?"]);
    assert!(entries.iter().all(|entry| !entry.keys.is_empty()));
    assert_eq!(
        HelpGroup::ALL.map(HelpGroup::title),
        ["Local", "Global", "Custom"]
    );
    assert!(entries.iter().all(|entry| entry.name != "checkout"));
    press(&mut app, KeyCode::Esc);
    app.ui.active_panel = SidePanel::LocalBranches;
    open(&mut app);
    assert_eq!(app.input.help.context, "Branches");
    assert!(app
        .input
        .help
        .entries
        .iter()
        .any(|entry| entry.name == "checkout"));
    assert!(app
        .input
        .help
        .entries
        .iter()
        .all(|entry| entry.name != "stage all"));
}

#[test]
fn test_help_lists_overrides_in_custom_and_omits_unbound_and_conflicting_keys() {
    let mut keymap = Keymap::default();
    keymap
        .files
        .bindings
        .insert("copy_path".into(), vec!["x".into()]);
    keymap
        .files
        .bindings
        .insert("ignore_file".into(), Vec::new());
    // quit has higher registry priority than stage-all.
    keymap
        .files
        .bindings
        .insert("toggle_stage_all".into(), vec!["q".into()]);
    let mut app = app_with_keymap(keymap);
    open(&mut app);
    let entries = &app.input.help.entries;
    let custom = entries
        .iter()
        .find(|entry| entry.name == "copy path")
        .unwrap();
    assert_eq!(custom.group, HelpGroup::Custom);
    assert_eq!(custom.keys, ["x"]);
    assert!(entries
        .iter()
        .all(|entry| !matches!(entry.name, "ignore" | "stage all")));
    assert!(!entries
        .iter()
        .any(|entry| entry.keys.contains(&"C-o".into())));
}

#[test]
fn test_help_closes_without_quitting_and_blocks_underlying_actions() {
    let mut app = app_with_keymap(Keymap::default());
    for close in [KeyCode::Char('?'), KeyCode::Char('q'), KeyCode::Esc] {
        open(&mut app);
        let selected = app.files.view().panel.selected();
        let log = app.command_log.len();
        for code in ['a', 'A', 'd', 'i', 'e', 'c', '1', 'v', 'r'] {
            assert!(
                map_test_key(&app, KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE))
                    .is_none()
            );
            press(&mut app, KeyCode::Char(code));
        }
        press(&mut app, KeyCode::Char('j'));
        assert_eq!(app.input.help.selected, 1);
        assert_eq!(app.files.view().panel.selected(), selected);
        assert_eq!(app.command_log.len(), log);
        press(&mut app, close);
        assert!(app.input.mode.is_none());
        assert!(app.running);
    }
}

#[test]
fn test_help_navigation_filter_and_no_match_are_bounded() {
    let mut app = app_with_keymap(Keymap::default());
    open(&mut app);
    press(&mut app, KeyCode::PageDown);
    assert_eq!(app.input.help.selected, 10);
    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.input.help.selected, 0);
    press(&mut app, KeyCode::Up);
    assert_eq!(app.input.help.selected, 0);
    press(&mut app, KeyCode::End);
    let last = app.input.help.entries.len() - 1;
    assert_eq!(app.input.help.selected, last);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.input.help.selected, last);
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Char('/'));
    for c in "COPY PATH".chars() {
        press(&mut app, KeyCode::Char(c));
    }
    assert_eq!(app.input.help.filtered_entries().len(), 1);
    assert_eq!(app.input.help.filtered_entries()[0].name, "copy path");
    assert_eq!(app.input.help.selected, 0);
    // Filtering owns normal command letters, including quit and help.
    for c in ['q', '?', '界'] {
        press(&mut app, KeyCode::Char(c));
    }
    press(&mut app, KeyCode::Backspace);
    assert_eq!(app.input.help.query, "COPY PATHq?");
    assert!(app.input.help.filtered_entries().is_empty());
    press(&mut app, KeyCode::Down);
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.input.mode, Some(InputMode::Keybindings));
    assert_eq!(app.input.help.selected, 0);
    press(&mut app, KeyCode::Esc);
    assert_eq!(app.input.mode, Some(InputMode::Keybindings));
    assert!(!app.input.help.filtering);
    assert!(app.input.help.query.is_empty());
    press(&mut app, KeyCode::Esc);
    assert!(app.input.mode.is_none());
}

#[test]
fn test_help_honors_custom_open_and_navigation_bindings() {
    let mut keymap = Keymap::default();
    keymap
        .global
        .bindings
        .insert("help".into(), vec!["F1".into()]);
    keymap
        .global
        .bindings
        .insert("list_down".into(), vec!["J".into()]);
    let mut app = app_with_keymap(keymap);
    press(&mut app, KeyCode::Char('?'));
    assert!(app.input.mode.is_none());
    press(&mut app, KeyCode::F(1));
    assert_eq!(app.input.mode, Some(InputMode::Keybindings));
    press(&mut app, KeyCode::Char('j'));
    assert_eq!(app.input.help.selected, 0);
    press(&mut app, KeyCode::Char('J'));
    assert_eq!(app.input.help.selected, 1);
    press(&mut app, KeyCode::F(1));
    assert!(app.input.mode.is_none());
}

#[test]
fn test_question_mark_remains_text_in_existing_editors() {
    for mode in [
        InputMode::CommitEditor,
        InputMode::CommandPalette,
        InputMode::CreateBranch,
        InputMode::StashEditor,
        InputMode::Search,
    ] {
        let mut app = app_with_keymap(Keymap::default());
        app.input.mode = Some(mode);
        press(&mut app, KeyCode::Char('?'));
        assert_eq!(app.input.mode, Some(mode));
        let buffer = match mode {
            InputMode::CommitEditor => &app.input.commit_message_buffer,
            InputMode::StashEditor => &app.input.stash_message_buffer,
            _ => &app.input.buffer,
        };
        assert_eq!(buffer, "?");
    }
}

#[test]
fn test_help_executes_selected_command_through_flux_and_revalidates_availability() {
    for execute in [KeyCode::Enter, KeyCode::Char(' ')] {
        let mut app = app_with_keymap(Keymap::default());
        open(&mut app);
        app.input.help.selected = app
            .input
            .help
            .entries
            .iter()
            .position(|entry| entry.name == "stage all")
            .unwrap();
        let action = map_test_key(&app, KeyEvent::new(execute, KeyModifiers::NONE)).unwrap();
        let mut dispatcher = Dispatcher::with_default_stores();
        let envelope = dispatcher.next_envelope(Action::Domain(action));
        let output = dispatcher.dispatch(&mut app.state, envelope);
        assert!(app.input.mode.is_none());
        assert!(matches!(
            output.commands.as_slice(),
            [crate::app::Command::Sync(DomainAction::ToggleStageAll)]
        ));
    }
    let mut app = app_with_keymap(Keymap::default());
    open(&mut app);
    app.input.help.selected = app
        .input
        .help
        .entries
        .iter()
        .position(|entry| entry.name == "edit")
        .unwrap();
    // Simulate a background status update removing the selected file.
    app.files.test_view_mut().tree_nodes.clear();
    let mut dispatcher = Dispatcher::with_default_stores();
    let envelope =
        dispatcher.next_envelope(Action::Domain(DomainAction::Help(HelpAction::Execute)));
    let output = dispatcher.dispatch(&mut app.state, envelope);
    assert!(output.commands.is_empty());
    assert!(app.input.mode.is_none());
}

#[test]
fn test_help_snapshots_own_navigation_and_share_immutable_entries() {
    let mut app = app_with_keymap(Keymap::default());
    open(&mut app);
    let owned = AppStateSnapshotOwned::from_app(&app);
    assert!(owned.help.entries.ptr_eq(&app.input.help.entries));
    let borrowed = AppStateSnapshot::from_app(&app);
    assert_eq!(borrowed.help.context, owned.as_snapshot().help.context);
    press(&mut app, KeyCode::End);
    assert_eq!(owned.help.selected, 0);
    assert!(app.input.help.selected > 0);
}
