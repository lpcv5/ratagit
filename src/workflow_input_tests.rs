use crate::input::{KeyEffect, key_effect_for_key};
use crossterm::event::{KeyCode as K, KeyModifiers as M};
use ratagit_core::workflow::{Feature as F, WorkflowAction as A};
use ratagit_core::{Action, AppContext, GitResult, PanelFocus, UiAction, update};

fn effect(state: &AppContext, code: K, modifiers: M) -> KeyEffect {
    key_effect_for_key(state, code, modifiers, 5, 12, 8)
}
fn state(panel: PanelFocus) -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(ratagit_testkit::fixture_dirty_repo())),
    );
    state.ui.focus = panel;
    state
}

#[test]
fn new_panel_keys_and_modifiers_route_to_their_workflows() {
    for (panel, code, modifiers, feature) in [
        (PanelFocus::Files, K::Char('b'), M::CONTROL, F::Filter),
        (PanelFocus::Files, K::Char('M'), M::SHIFT, F::Conflict),
        (PanelFocus::Files, K::Char('S'), M::SHIFT, F::Stash),
        (PanelFocus::Files, K::Char('`'), M::NONE, F::ToggleTree),
        (PanelFocus::Files, K::Char('-'), M::NONE, F::CollapseAll),
        (PanelFocus::Files, K::Char('='), M::NONE, F::ExpandAll),
        (
            PanelFocus::Branches,
            K::Char('c'),
            M::NONE,
            F::CheckoutByName,
        ),
        (PanelFocus::Branches, K::Char('M'), M::SHIFT, F::Merge),
        (PanelFocus::Branches, K::Char('R'), M::SHIFT, F::Rename),
        (PanelFocus::Branches, K::Char('u'), M::NONE, F::Upstream),
        (PanelFocus::Branches, K::Char('f'), M::NONE, F::FastForward),
        (PanelFocus::Commits, K::Char('C'), M::SHIFT, F::CopyCommits),
        (PanelFocus::Commits, K::Char('V'), M::SHIFT, F::PasteCommits),
        (PanelFocus::Commits, K::Char('t'), M::NONE, F::Revert),
        (PanelFocus::Commits, K::Char('F'), M::SHIFT, F::Fixup),
        (PanelFocus::Commits, K::Char('k'), M::CONTROL, F::MoveUp),
        (PanelFocus::Commits, K::Down, M::ALT, F::MoveDown),
    ] {
        assert_eq!(
            effect(&state(panel), code, modifiers),
            KeyEffect::Dispatch(UiAction::Workflow(A::Open(feature)))
        );
    }
}

#[test]
fn copy_hosts_external_editor_and_modal_precedence() {
    let mut state = state(PanelFocus::Branches);
    assert!(matches!(
        effect(&state, K::Char('o'), M::CONTROL),
        KeyEffect::Host { edit: false, .. }
    ));
    state.ui.focus = PanelFocus::Commits;
    assert!(matches!(
        effect(&state, K::Char('o'), M::CONTROL),
        KeyEffect::Host { edit: false, .. }
    ));
    state.ui.focus = PanelFocus::Files;
    assert_eq!(
        effect(&state, K::Char('C'), M::NONE),
        KeyEffect::CommitEditor
    );
    assert_eq!(
        effect(&state, K::Char('C'), M::SHIFT),
        KeyEffect::CommitEditor
    );
    update(
        &mut state,
        Action::Ui(UiAction::Workflow(A::Open(F::CommitNoVerify))),
    );
    assert_eq!(
        effect(&state, K::Char('M'), M::SHIFT),
        KeyEffect::Dispatch(UiAction::Workflow(A::Input('M')))
    );
    assert_eq!(
        effect(&state, K::Esc, M::NONE),
        KeyEffect::Dispatch(UiAction::Workflow(A::Cancel))
    );
    state.ui.workflow.dialog = None;
    state.ui.editor.kind = Some(ratagit_core::EditorKind::Stash {
        title: String::new(),
        title_cursor: 0,
        scope: ratagit_core::StashScope::All,
    });
    assert!(matches!(
        effect(&state, K::Char('M'), M::SHIFT),
        KeyEffect::Dispatch(UiAction::EditorInputChar('M'))
    ));
}

#[test]
fn palette_search_enter_does_not_copy_branch_or_commit() {
    for (panel, feature) in [
        (PanelFocus::Branches, F::CopyBranch),
        (PanelFocus::Commits, F::CopyCommit),
    ] {
        let mut ctx = state(panel);
        update(&mut ctx, Action::Ui(UiAction::OpenCommandPalette));
        ctx.ui.command_palette.selected = ctx
            .command_palette_entries()
            .iter()
            .position(|entry| {
                entry.command == ratagit_core::CommandPaletteCommand::Workflow(feature)
            })
            .expect("copy palette command");
        let copy = effect(&ctx, K::Enter, M::NONE);
        assert!(matches!(copy, KeyEffect::Host { edit: false, .. }));
        update(&mut ctx, Action::Ui(UiAction::StartPaletteSearch));
        assert_eq!(
            effect(&ctx, K::Enter, M::NONE),
            KeyEffect::Dispatch(UiAction::FinishPaletteSearch)
        );
        assert_eq!(crate::workflow_input::host(&ctx, K::Enter, M::NONE), None);
        update(&mut ctx, Action::Ui(UiAction::FinishPaletteSearch));
        assert_eq!(effect(&ctx, K::Enter, M::NONE), copy);
    }
}

#[test]
fn workflow_and_palette_own_capital_c_instead_of_launching_the_editor() {
    let mut ctx = state(PanelFocus::Files);
    update(
        &mut ctx,
        Action::Ui(UiAction::Workflow(A::Open(F::CommitNoVerify))),
    );
    assert_eq!(
        effect(&ctx, K::Char('C'), M::SHIFT),
        KeyEffect::Dispatch(UiAction::Workflow(A::Input('C')))
    );
    update(&mut ctx, Action::Ui(UiAction::Workflow(A::Cancel)));
    update(&mut ctx, Action::Ui(UiAction::OpenCommandPalette));
    assert_eq!(effect(&ctx, K::Char('C'), M::SHIFT), KeyEffect::Ignore);
    update(&mut ctx, Action::Ui(UiAction::StartPaletteSearch));
    assert_eq!(
        effect(&ctx, K::Char('C'), M::SHIFT),
        KeyEffect::Dispatch(UiAction::PaletteInput('C'))
    );
    ctx.ui.command_palette.query = "commit with external editor".into();
    ctx.ui.command_palette.selected = 0;
    assert_eq!(
        effect(&ctx, K::Enter, M::NONE),
        KeyEffect::Dispatch(UiAction::FinishPaletteSearch)
    );
    assert_eq!(crate::workflow_input::host(&ctx, K::Enter, M::NONE), None);
    update(&mut ctx, Action::Ui(UiAction::FinishPaletteSearch));
    assert_eq!(effect(&ctx, K::Enter, M::NONE), KeyEffect::CommitEditor);
    update(&mut ctx, Action::Ui(UiAction::CloseCommandPalette));
    update(&mut ctx, Action::Ui(UiAction::Workflow(A::Open(F::Stash))));
    assert_eq!(
        effect(&ctx, K::Char('C'), M::SHIFT),
        KeyEffect::Dispatch(UiAction::Workflow(A::Input('C')))
    );
    let mut commits = state(PanelFocus::Commits);
    update(
        &mut commits,
        Action::Ui(UiAction::Workflow(A::Open(F::CopyAttributes))),
    );
    assert!(matches!(
        effect(&commits, K::Enter, M::NONE),
        KeyEffect::Host { edit: false, .. }
    ));
}
