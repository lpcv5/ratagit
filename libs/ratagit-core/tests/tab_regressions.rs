use ratagit_core::{
    Action, AppContext, BranchDeleteMode, Command, GitErrorKind, GitFailure, GitResult, PanelFocus,
    RepositoryItem as I, RepositoryTab as T, UiAction, update,
};
fn dispatch(state: &mut AppContext, action: UiAction) -> Vec<Command> {
    update(state, Action::Ui(action))
}
fn finish(state: &mut AppContext, tab: T, items: Vec<I>) {
    let id = state.work.tabs.pending[&tab];
    update(
        state,
        Action::GitResult(GitResult::RepositoryTabLoaded {
            tab,
            request_id: id,
            result: Ok(items),
        }),
    );
}
#[test]
fn captured_force_delete_confirmation_can_cancel_or_confirm_after_switching_tabs() {
    for confirm in [false, true] {
        let mut state = AppContext::default();
        state.repo.branches.items = vec![
            ratagit_core::BranchEntry {
                name: "main".into(),
                is_current: true,
            },
            ratagit_core::BranchEntry {
                name: "feature/mvp".into(),
                is_current: false,
            },
        ];
        dispatch(
            &mut state,
            UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            },
        );
        dispatch(&mut state, UiAction::MoveDown);
        dispatch(&mut state, UiAction::OpenBranchDeleteMenu);
        assert!(
            dispatch(&mut state, UiAction::ConfirmBranchDeleteMenu)
                .iter()
                .any(|cmd| matches!(cmd, Command::DeleteBranch { force: false, .. }))
        );
        dispatch(&mut state, UiAction::CyclePanelTab { previous: false });
        update(
            &mut state,
            Action::GitResult(GitResult::DeleteBranch {
                name: "feature/mvp".into(),
                mode: BranchDeleteMode::Local,
                force: false,
                result: Err(GitFailure::new(
                    GitErrorKind::UnmergedBranchDelete,
                    "not merged",
                )),
            }),
        );
        assert!(state.ui.branches.force_delete_confirm.active);
        let commands = dispatch(
            &mut state,
            if confirm {
                UiAction::ConfirmBranchForceDelete
            } else {
                UiAction::CancelBranchForceDelete
            },
        );
        assert!(!state.ui.branches.force_delete_confirm.active);
        assert_eq!(commands.iter().any(|cmd| matches!(cmd, Command::DeleteBranch { name, force: true, .. } if name == "feature/mvp")), confirm);
    }
}
#[test]
fn remote_header_search_and_refresh_keep_variant_identity() {
    let mut state = AppContext::default();
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        },
    );
    dispatch(&mut state, UiAction::CyclePanelTab { previous: false });
    let branch = I::RemoteBranch {
        name: "team/service".into(),
        target: "abc".into(),
    };
    let remote = I::Remote {
        name: "team/service".into(),
        fetch_url: "unique-url".into(),
        push_url: "push-url".into(),
    };
    assert_ne!(branch.key(), remote.key());
    finish(&mut state, T::Remotes, vec![branch.clone(), remote.clone()]);
    dispatch(&mut state, UiAction::StartSearch);
    for ch in "unique-url".chars() {
        dispatch(&mut state, UiAction::InputSearchChar(ch));
    }
    dispatch(&mut state, UiAction::ConfirmSearch);
    assert!(matches!(
        ratagit_core::tabs::selected_item(&state, T::Remotes),
        Some(I::Remote { .. })
    ));
    dispatch(&mut state, UiAction::RefreshAll);
    finish(&mut state, T::Remotes, vec![remote, branch]);
    assert_eq!(state.ui.tabs.selections[&T::Remotes].selected, 0);
}
#[test]
fn cached_tab_focus_resets_details_scroll_for_all_focus_actions() {
    let mut state = AppContext::default();
    dispatch(&mut state, UiAction::CyclePanelTab { previous: true });
    finish(
        &mut state,
        T::Worktrees,
        vec![I::Worktree {
            path: "/repo".into(),
            head: "abc".into(),
            branch: "main".into(),
            detached: false,
            locked: None,
            prunable: None,
            main: true,
        }],
    );
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        },
    );
    dispatch(&mut state, UiAction::CyclePanelTab { previous: true });
    finish(
        &mut state,
        T::Tags,
        vec![I::Tag {
            name: "long".into(),
            target: "abc".into(),
            summary: "annotation\n".repeat(100),
        }],
    );
    for action in [
        UiAction::FocusPanel {
            panel: PanelFocus::Files,
        },
        UiAction::FocusPrev,
        UiAction::FocusNext,
    ] {
        dispatch(
            &mut state,
            UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            },
        );
        dispatch(
            &mut state,
            UiAction::DetailsScrollDown {
                lines: 50,
                visible_lines: 5,
            },
        );
        assert!(state.ui.details.scroll_offset > 7);
        dispatch(&mut state, action);
        assert_eq!(state.ui.details.scroll_offset, 0);
    }
}
