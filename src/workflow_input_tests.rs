use crate::input::{KeyEffect, key_effect_for_key};
use crossterm::event::{KeyCode as K, KeyModifiers as M};
use ratagit_core::workflow::{Feature as F, WorkflowAction as A};
use ratagit_core::{Action, AppContext, GitResult, PanelFocus, UiAction, update};

fn effect(state: &AppContext, code: K, modifiers: M) -> KeyEffect {
    key_effect_for_key(state, code, modifiers, 5, 12, 8)
}

#[test]
fn custom_patch_menu_shortcuts_copy_and_modal_ownership_match_lazygit_keys() {
    use ratagit_core::custom_patch::{CustomPatchAction as C, OPTIONS_TITLE};
    use ratagit_core::workflow::{Choice, ChoiceAction, Dialog};
    let mut ctx = state(PanelFocus::Files);
    assert_eq!(
        effect(&ctx, K::Char('p'), M::CONTROL),
        KeyEffect::Dispatch(UiAction::CustomPatch(C::Options))
    );
    let mut choices = vec![
        Choice {
            label: "clear".into(),
            action: ChoiceAction::CustomPatch(C::Reset)
        };
        5
    ];
    choices.push(Choice {
        label: "copy".into(),
        action: ChoiceAction::Copy("exact native patch".into()),
    });
    ctx.ui.workflow.dialog = Some(Dialog::Menu {
        title: OPTIONS_TITLE,
        choices,
        selected: 0,
    });
    assert_eq!(
        effect(&ctx, K::Char('y'), M::NONE),
        KeyEffect::Host {
            edit: false,
            path: "exact native patch".into()
        }
    );
    for key in ['c', 'a', 'r', 'd', 'p'] {
        assert_eq!(
            effect(&ctx, K::Char(key), M::NONE),
            KeyEffect::Dispatch(UiAction::CustomPatch(C::MenuKey(key)))
        );
    }
    assert_eq!(
        effect(&ctx, K::Char('j'), M::NONE),
        KeyEffect::Dispatch(UiAction::Workflow(A::Down))
    );
    assert_eq!(effect(&ctx, K::Char('p'), M::CONTROL), KeyEffect::Ignore);
    ctx.ui.workflow.dialog = None;
    ctx.ui.editor.kind = Some(ratagit_core::EditorKind::Stash {
        title: String::new(),
        title_cursor: 0,
        scope: ratagit_core::StashScope::All,
    });
    assert_eq!(effect(&ctx, K::Char('p'), M::CONTROL), KeyEffect::Ignore);
    ctx.ui.editor.kind = None;
    ctx.ui.custom_patch.preview_return_focus = Some(PanelFocus::Files);
    assert_eq!(
        effect(&ctx, K::Tab, M::NONE),
        KeyEffect::Dispatch(UiAction::CustomPatch(C::ClosePreview))
    );
    assert_eq!(effect(&ctx, K::Char(' '), M::NONE), KeyEffect::Ignore);
    assert_eq!(
        effect(&ctx, K::Char('d'), M::CONTROL),
        KeyEffect::Dispatch(UiAction::CustomPatch(C::Scroll {
            up: false,
            lines: 5
        }))
    );
}

#[test]
fn commit_files_keys_and_host_actions_respect_current_view_and_dialogs() {
    let mut ctx = state(PanelFocus::Commits);
    ctx.ui.last_left_focus = PanelFocus::Commits;
    update(&mut ctx, Action::Ui(UiAction::OpenCommitFilesPanel));
    update(
        &mut ctx,
        Action::GitResult(GitResult::CommitFiles {
            commit_id: "abc1234".into(),
            result: Ok(vec![ratagit_core::CommitFileEntry {
                path: "a.txt".into(),
                old_path: None,
                status: ratagit_core::CommitFileStatus::Modified,
            }]),
        }),
    );
    for (key, feature) in [
        ('c', F::CommitFileCheckout),
        ('d', F::CommitFileDiscard),
        (' ', F::CommitFilePatch),
        ('y', F::CommitFileCopy),
        ('a', F::CommitFilePatchAll),
    ] {
        assert_eq!(
            effect(&ctx, K::Char(key), M::NONE),
            KeyEffect::Dispatch(UiAction::Workflow(A::Open(feature)))
        );
    }
    assert_eq!(
        effect(&ctx, K::Enter, M::NONE),
        KeyEffect::Dispatch(UiAction::OpenCommitFilePatch)
    );
    assert_eq!(
        effect(&ctx, K::Char('p'), M::CONTROL),
        KeyEffect::Dispatch(UiAction::CustomPatch(
            ratagit_core::custom_patch::CustomPatchAction::Options
        ))
    );
    assert_eq!(
        effect(&ctx, K::Tab, M::NONE),
        KeyEffect::Dispatch(UiAction::Workflow(A::Open(F::CustomPatchPreview)))
    );
    assert_eq!(
        effect(&ctx, K::Char('o'), M::CONTROL),
        KeyEffect::Host {
            edit: false,
            path: "a.txt".into()
        }
    );
    assert_eq!(
        effect(&ctx, K::Char('e'), M::NONE),
        KeyEffect::Host {
            edit: true,
            path: "a.txt".into()
        }
    );
    update(
        &mut ctx,
        Action::Ui(UiAction::Workflow(A::Open(F::CommitFileCheckout))),
    );
    assert_eq!(
        effect(&ctx, K::Char('e'), M::NONE),
        KeyEffect::Dispatch(UiAction::Workflow(A::Input('e')))
    );
    assert_eq!(
        effect(&ctx, K::Enter, M::NONE),
        KeyEffect::Dispatch(UiAction::Workflow(A::Confirm))
    );
    update(&mut ctx, Action::Ui(UiAction::Workflow(A::Cancel)));
    update(&mut ctx, Action::Ui(UiAction::OpenCommandPalette));
    ctx.ui.command_palette.selected = ctx
        .command_palette_entries()
        .iter()
        .position(|entry| entry.command == ratagit_core::CommandPaletteCommand::CopyPath)
        .unwrap();
    assert_eq!(
        effect(&ctx, K::Enter, M::NONE),
        KeyEffect::Host {
            edit: false,
            path: "a.txt".into()
        }
    );
    update(&mut ctx, Action::Ui(UiAction::StartPaletteSearch));
    assert_eq!(
        effect(&ctx, K::Enter, M::NONE),
        KeyEffect::Dispatch(UiAction::FinishPaletteSearch)
    );
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
