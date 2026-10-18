use ratagit_core::{
    Action, ActiveLeftView, AppContext, Command, GitResult, PanelFocus, RepositoryItem,
    RepositoryTab as T, UiAction, update,
};

fn dispatch(state: &mut AppContext, action: UiAction) -> Vec<Command> {
    update(state, Action::Ui(action))
}
fn cycle(state: &mut AppContext, previous: bool) -> Vec<Command> {
    dispatch(state, UiAction::CyclePanelTab { previous })
}
fn items() -> Vec<RepositoryItem> {
    (0..10)
        .map(|i| RepositoryItem::Submodule {
            path: format!("module-{i}"),
            url: "local".into(),
            head: "abc".into(),
            status: "initialized".into(),
        })
        .collect()
}
fn complete(state: &mut AppContext, command: &Command, items: Vec<RepositoryItem>) {
    let Command::LoadRepositoryTab { tab, request_id } = command else {
        panic!("expected tab read")
    };
    update(
        state,
        Action::GitResult(GitResult::RepositoryTabLoaded {
            tab: *tab,
            request_id: *request_id,
            result: Ok(items),
        }),
    );
}

#[test]
fn sibling_tabs_wrap_and_retain_selection_and_scroll_independently() {
    let mut state = AppContext::default();
    let read = cycle(&mut state, false);
    assert_eq!(
        state.active_left_view(),
        Some(ActiveLeftView::RepositoryTab(T::Submodules))
    );
    assert!(!read[0].is_mutating());
    assert_eq!(read[0].log_label(), "load_repository_tab");
    assert_eq!(read[0].pending_operation_label(), None);
    assert_eq!(read[0].debounce_key(), None);
    assert_eq!(read[0].refresh_coalescing_key(), None);
    let result = GitResult::RepositoryTabLoaded {
        tab: T::Submodules,
        request_id: 1,
        result: Ok(Vec::new()),
    };
    assert_eq!(result.log_label(), "repository_tab_loaded");
    assert!(result.is_success());
    assert!(
        !GitResult::RepositoryTabLoaded {
            tab: T::Submodules,
            request_id: 1,
            result: Err("failed".into())
        }
        .is_success()
    );
    complete(&mut state, &read[0], items());
    for _ in 0..7 {
        dispatch(
            &mut state,
            UiAction::MoveDownInViewport { visible_lines: 3 },
        );
    }
    let selection = state.ui.tabs.selections[&T::Submodules].clone();
    assert_eq!(selection.selected, 7);
    assert!(selection.scroll_offset > 0);
    cycle(&mut state, false);
    assert_eq!(state.ui.tabs.files, Some(T::Worktrees));
    cycle(&mut state, false);
    assert_eq!(state.ui.tabs.files, None);
    cycle(&mut state, true);
    assert_eq!(state.ui.tabs.files, Some(T::Worktrees));
    cycle(&mut state, true);
    assert_eq!(state.ui.tabs.selections[&T::Submodules], selection);
    for (panel, last) in [
        (PanelFocus::Branches, T::Tags),
        (PanelFocus::Commits, T::Reflog),
    ] {
        dispatch(&mut state, UiAction::FocusPanel { panel });
        cycle(&mut state, true);
        assert_eq!(state.ui.tabs.active(panel), Some(last));
        cycle(&mut state, false);
        assert_eq!(state.ui.tabs.active(panel), None);
    }
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Stash,
        },
    );
    assert!(cycle(&mut state, false).is_empty());
}

#[test]
fn tab_reads_reject_superseded_requests_and_do_not_replace_current_details() {
    let mut state = AppContext::default();
    let old = cycle(&mut state, false).remove(0);
    let refreshed = dispatch(&mut state, UiAction::RefreshAll);
    let new = refreshed
        .iter()
        .find(|cmd| matches!(cmd, Command::LoadRepositoryTab { .. }))
        .unwrap();
    complete(&mut state, &old, items());
    assert!(!state.repo.tabs.contains_key(&T::Submodules));
    complete(&mut state, new, items());
    cycle(&mut state, false);
    assert!(ratagit_core::tabs::details_text(&state, T::Worktrees).contains("Loading"));
    complete(&mut state, &old, Vec::new());
    assert_eq!(state.repo.tabs[&T::Submodules].items.len(), 10);
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Details,
        },
    );
    assert_eq!(
        state.details_left_view(),
        Some(ActiveLeftView::RepositoryTab(T::Worktrees))
    );
}

#[test]
fn read_only_tabs_block_hidden_selection_actions_and_search_is_scoped() {
    let mut state = AppContext::default();
    for panel in [PanelFocus::Files, PanelFocus::Branches, PanelFocus::Commits] {
        dispatch(&mut state, UiAction::FocusPanel { panel });
        let read = cycle(&mut state, false);
        complete(&mut state, &read[0], items());
        for action in [
            UiAction::StageSelectedFile,
            UiAction::ToggleStageAll,
            UiAction::OpenFilePatch,
            UiAction::CheckoutSelectedBranch,
            UiAction::OpenBranchDeleteMenu,
            UiAction::CheckoutSelectedCommitDetached,
            UiAction::DeleteSelectedCommits,
            UiAction::AmendStagedChanges,
            UiAction::OpenCommitEditor,
        ] {
            let before = state.clone();
            assert!(dispatch(&mut state, action).is_empty());
            assert_eq!(state, before);
        }
        dispatch(&mut state, UiAction::StartSearch);
        for ch in "module-7".chars() {
            dispatch(&mut state, UiAction::InputSearchChar(ch));
        }
        dispatch(&mut state, UiAction::ConfirmSearch);
        let tab = state.ui.tabs.active(panel).unwrap();
        assert_eq!(state.ui.tabs.selections[&tab].selected, 7);
        cycle(&mut state, false);
        assert!(state.ui.search.query.is_empty());
    }
}

#[test]
fn auxiliary_details_refresh_and_global_dialogs_keep_working() {
    use ratagit_core::workflow::{Feature as F, WorkflowAction as W};
    let mut state = AppContext::default();
    cycle(&mut state, false);
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Details,
        },
    );
    let refreshed = dispatch(&mut state, UiAction::RefreshAll);
    assert!(refreshed.iter().any(|cmd| matches!(
        cmd,
        Command::LoadRepositoryTab {
            tab: T::Submodules,
            ..
        }
    )));
    for action in [
        UiAction::CreateCommit {
            message: "hidden".into(),
        },
        UiAction::AmendStagedChanges,
        UiAction::StashPopSelected,
    ] {
        let before = state.clone();
        assert!(dispatch(&mut state, action).is_empty());
        assert_eq!(state, before);
    }
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Files,
        },
    );
    dispatch(&mut state, UiAction::Workflow(W::Open(F::Sequence)));
    assert!(state.ui.workflow.dialog.is_some());
    dispatch(&mut state, UiAction::Workflow(W::Down));
    assert!(cycle(&mut state, false).is_empty());
    assert_eq!(state.ui.tabs.files, Some(T::Submodules));
    dispatch(&mut state, UiAction::Workflow(W::Cancel));
    assert!(state.ui.workflow.dialog.is_none());
    dispatch(
        &mut state,
        UiAction::CustomPatch(ratagit_core::custom_patch::CustomPatchAction::Options),
    );
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.to_lowercase().contains("custom patch"))
    );
    dispatch(&mut state, UiAction::Workflow(W::Cancel));
    assert!(state.ui.workflow.dialog.is_none());
}

#[test]
fn refresh_reloads_all_visible_tabs_and_search_resets_details_scroll() {
    let mut state = AppContext::default();
    let read = cycle(&mut state, false);
    complete(&mut state, &read[0], items());
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        },
    );
    let read = cycle(&mut state, false);
    complete(&mut state, &read[0], items());
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Details,
        },
    );
    let commands = dispatch(&mut state, UiAction::RefreshAll);
    assert_eq!(
        commands
            .iter()
            .filter(|cmd| matches!(cmd, Command::LoadRepositoryTab { .. }))
            .count(),
        2
    );
    for command in commands
        .iter()
        .filter(|cmd| matches!(cmd, Command::LoadRepositoryTab { .. }))
    {
        complete(&mut state, command, items());
    }
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        },
    );
    state.ui.details.scroll_offset = 50;
    dispatch(&mut state, UiAction::StartSearch);
    for ch in "module-7".chars() {
        dispatch(&mut state, UiAction::InputSearchChar(ch));
    }
    dispatch(&mut state, UiAction::ConfirmSearch);
    assert_eq!(state.ui.details.scroll_offset, 0);
    // A real mutation invalidates the lists; its completion reloads both visible tabs
    // even when Details retains focus.
    dispatch(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Details,
        },
    );
    dispatch(&mut state, UiAction::Fetch);
    assert!(state.work.tabs.pending.is_empty());
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::RepositoryOperation {
            operation: ratagit_core::extras::RepositoryOperation::Fetch,
            result: Ok(()),
        }),
    );
    assert_eq!(
        commands
            .iter()
            .filter(|cmd| matches!(cmd, Command::LoadRepositoryTab { .. }))
            .count(),
        2
    );
}

#[test]
fn tab_switch_preserves_branch_drilldown_and_commit_file_state() {
    let mut state = AppContext::default();
    state.ui.focus = PanelFocus::Branches;
    state.ui.last_left_focus = PanelFocus::Branches;
    state.ui.branches.subview = ratagit_core::BranchesSubview::CommitFiles;
    state.ui.branches.subview_branch = Some("topic".into());
    state.ui.branches.commit_files.active = true;
    state.ui.branches.commit_files.commit_id = Some("abcdef".into());
    state.ui.branches.commit_files.selected = 4;
    let saved = state.ui.branches.clone();
    cycle(&mut state, false);
    cycle(&mut state, true);
    assert_eq!(state.ui.branches, saved);
    assert_eq!(
        state.active_left_view(),
        Some(ActiveLeftView::BranchCommitFiles)
    );
    state.ui.focus = PanelFocus::Commits;
    state.ui.commits.files.active = true;
    state.ui.commits.files.commit_id = Some("abcdef".into());
    state.ui.commits.files.selected = 3;
    let saved = state.ui.commits.clone();
    cycle(&mut state, false);
    cycle(&mut state, false);
    assert_eq!(state.ui.commits, saved);
    assert_eq!(state.active_left_view(), Some(ActiveLeftView::CommitFiles));
}

#[test]
fn tab_error_can_be_retried_and_old_details_are_invalidated() {
    let mut state = AppContext::default();
    state.work.details.details_request = Some(ratagit_core::DetailsRequest {
        id: ratagit_core::DetailsRequestId(1),
        target: ratagit_core::DetailsRequestTarget::BranchLog {
            branch: "main".into(),
        },
    });
    let read = cycle(&mut state, false);
    assert!(state.work.details.details_request.is_none());
    let Command::LoadRepositoryTab { tab, request_id } = read[0] else {
        panic!()
    };
    update(
        &mut state,
        Action::GitResult(GitResult::RepositoryTabLoaded {
            tab,
            request_id,
            result: Err("read failed".into()),
        }),
    );
    assert!(ratagit_core::tabs::details_text(&state, tab).contains("read failed"));
    let retry = dispatch(&mut state, UiAction::RefreshAll);
    assert!(retry.iter().any(|cmd| matches!(cmd, Command::LoadRepositoryTab { request_id: next, .. } if *next > request_id)));
}
