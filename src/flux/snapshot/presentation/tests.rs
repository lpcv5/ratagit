use crate::app::help::HelpAction;
use crate::app::{dispatch_test_action, dispatch_test_key, App, InputMode, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::snapshot::{AppStateSnapshot, AppStateSnapshotOwned};
use crate::flux::stores::test_support::MockRepo;
use crate::ui::components::diff::DiffStatus;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_help_display_order_and_execute_target_agree_in_retained_snapshot() {
    let mut app = App::from_repo(Box::new(MockRepo)).unwrap();
    app.ui.active_panel = SidePanel::LocalBranches;
    dispatch_test_action(&mut app, DomainAction::Help(HelpAction::Open));
    dispatch_test_action(&mut app, DomainAction::Help(HelpAction::Filter));
    for c in "branch new".chars() {
        dispatch_test_action(&mut app, DomainAction::Help(HelpAction::Char(c)));
    }
    let snapshot = AppStateSnapshotOwned::from_app(&app);
    let view = snapshot.as_snapshot();
    let props = view.keybindings_props();
    assert_eq!(props.entries.len(), 1);
    assert_eq!(props.entries[props.selected].name, "branch new");
    assert_eq!(
        AppStateSnapshot::from_app(&app).keybindings_props().entries[0].name,
        props.entries[0].name
    );

    dispatch_test_key(&mut app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(app.input.mode, Some(InputMode::CreateBranch));
    assert_eq!(props.query, "branch new");
    assert_eq!(props.entries[props.selected].name, "branch new");
}

#[test]
fn test_diff_presentation_keeps_retained_preview_loading_and_non_actionable() {
    let app = App::from_repo(Box::new(MockRepo)).unwrap();
    let mut snapshot = AppStateSnapshotOwned::from_app(&app);
    snapshot.diff_ready = false;
    snapshot.diff_loading = false;
    snapshot.diff_interaction.focused = true;
    snapshot.active_panel = SidePanel::Commits;
    let patch = std::sync::Arc::new(crate::git::patch::Patch {
        request: crate::git::patch::PatchRequest::Worktree("previous.rs".into()),
        files: vec![],
        truncated: false,
        preview: None,
    });
    snapshot.retained_diff_patch = Some(patch.clone());
    let view = snapshot.as_snapshot();
    let presentation = view.diff_presentation();
    let model = presentation.view();
    assert_eq!(model.status, DiffStatus::Loading);
    assert_eq!(model.selection.position.current, 0);
    assert_eq!(model.selection.position.total, 0);
    assert!(std::ptr::eq(model.patch.unwrap(), patch.as_ref()));
    assert!(view.command_env().diff.patch.is_none());
    assert_eq!(model.empty_message, "Select a commit/file to view diff");
}
