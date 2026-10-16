use ratagit_core::extras::RepositoryOperation;
use ratagit_core::workflow::{Dialog, Feature, Operation, WorkflowAction};
use ratagit_core::{
    Action, AppContext, Command, GitResult, PanelFocus, ResetMode, UiAction, update,
};

fn dispatch(state: &mut AppContext, action: WorkflowAction) -> Vec<Command> {
    update(state, Action::Ui(UiAction::Workflow(action)))
}

#[test]
fn remote_request_ownership_cancel_and_metadata() {
    let mut state = AppContext::default();
    state.ui.focus = PanelFocus::Branches;
    let commands = dispatch(&mut state, WorkflowAction::Open(Feature::RemoteBranches));
    let request = state.ui.workflow.remote_request.unwrap();
    assert_eq!(
        commands,
        vec![Command::RemoteBranches {
            request_id: request
        }]
    );
    let command = &commands[0];
    assert_eq!(command.log_label(), "remote_branches");
    assert!(!command.is_mutating());
    assert_eq!(command.debounce_key(), None);
    assert_eq!(command.refresh_coalescing_key(), None);
    assert_eq!(command.pending_operation_label(), None);
    dispatch(&mut state, WorkflowAction::Cancel);
    update(
        &mut state,
        Action::GitResult(GitResult::RemoteBranches {
            request_id: request,
            result: Ok(vec!["origin/old".into()]),
        }),
    );
    assert!(state.ui.workflow.dialog.is_none());
    dispatch(&mut state, WorkflowAction::Open(Feature::RemoteBranches));
    let current = state.ui.workflow.remote_request.unwrap();
    update(
        &mut state,
        Action::GitResult(GitResult::RemoteBranches {
            request_id: request,
            result: Ok(vec!["origin/stale".into()]),
        }),
    );
    assert_eq!(state.ui.workflow.remote_request, Some(current));
    assert!(state.ui.workflow.dialog.is_none());
    let result = GitResult::RemoteBranches {
        request_id: current,
        result: Ok(vec!["origin/current".into()]),
    };
    assert_eq!(result.log_label(), "remote_branches");
    assert!(result.is_success());
    update(&mut state, Action::GitResult(result));
    assert!(matches!(
        state.ui.workflow.dialog,
        Some(Dialog::Menu { .. })
    ));
    state.ui.workflow.remote_request = Some(current + 1);
    update(&mut state, Action::Ui(UiAction::Pull));
    assert!(state.ui.workflow.remote_request.is_none());
}

#[test]
fn destructive_confirmation_captures_target_and_blocks_duplicate_mutation() {
    let mut state = AppContext::default();
    let operation = Operation::Reset {
        target: "frozen-target".into(),
        mode: ResetMode::Hard,
    };
    state.ui.workflow.dialog = Some(Dialog::Confirm(operation.clone()));
    state.work.mutation.operation_pending = Some("other".into());
    assert!(dispatch(&mut state, WorkflowAction::Confirm).is_empty());
    assert_eq!(
        state.ui.workflow.dialog,
        Some(Dialog::Confirm(operation.clone()))
    );
    state.work.mutation.operation_pending = None;
    assert_eq!(
        dispatch(&mut state, WorkflowAction::Confirm),
        vec![Command::RepositoryOperation(RepositoryOperation::Workflow(
            operation
        ))]
    );
    assert!(state.ui.workflow.dialog.is_none());
    assert!(dispatch(&mut state, WorkflowAction::Confirm).is_empty());
}

#[test]
fn failed_mutation_refreshes_conflict_state_and_retains_copied_commits() {
    let mut state = AppContext::default();
    state.ui.workflow.copied_commits = vec!["first".into(), "second".into()];
    let operation = RepositoryOperation::Workflow(Operation::CherryPick(
        state.ui.workflow.copied_commits.clone(),
    ));
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::RepositoryOperation {
            operation: operation.clone(),
            result: Err("conflict".into()),
        }),
    );
    assert_eq!(commands, Command::refresh_all_commands());
    assert_eq!(state.ui.workflow.copied_commits, vec!["first", "second"]);
    assert!(state.notices.iter().any(|line| line == "conflict"));
    update(
        &mut state,
        Action::GitResult(GitResult::RepositoryOperation {
            operation,
            result: Ok(()),
        }),
    );
    assert!(state.ui.workflow.copied_commits.is_empty());
}

#[test]
fn unicode_prompt_cursor_edits_and_empty_validation_preserve_dialog() {
    let mut state = AppContext::default();
    state.ui.focus = PanelFocus::Branches;
    dispatch(&mut state, WorkflowAction::Open(Feature::CheckoutByName));
    for ch in "a你好".chars() {
        dispatch(&mut state, WorkflowAction::Input(ch));
    }
    dispatch(&mut state, WorkflowAction::CursorLeft);
    dispatch(&mut state, WorkflowAction::Backspace);
    assert!(
        matches!(&state.ui.workflow.dialog, Some(Dialog::Prompt { text, cursor: 1, .. }) if text == "a好")
    );
    dispatch(&mut state, WorkflowAction::CursorEnd);
    dispatch(&mut state, WorkflowAction::Backspace);
    dispatch(&mut state, WorkflowAction::Backspace);
    assert!(dispatch(&mut state, WorkflowAction::Confirm).is_empty());
    assert!(
        matches!(&state.ui.workflow.dialog, Some(Dialog::Prompt { text, .. }) if text.is_empty())
    );
}

#[test]
fn filtered_folder_targets_and_flat_directory_markers_stay_coherent() {
    use ratagit_core::workflow::FileFilter;
    use ratagit_core::{CommitFileStatus, FileEntry, FilesUiState};
    let items = vec![
        FileEntry {
            path: "src/a.rs".into(),
            staged: true,
            unstaged: false,
            untracked: false,
            status: CommitFileStatus::Modified,
            conflicted: false,
        },
        FileEntry {
            path: "src/b.rs".into(),
            staged: false,
            unstaged: true,
            untracked: false,
            status: CommitFileStatus::Modified,
            conflicted: false,
        },
        FileEntry {
            path: "unknown/".into(),
            staged: false,
            unstaged: true,
            untracked: true,
            status: CommitFileStatus::Unknown,
            conflicted: false,
        },
    ];
    let mut files = FilesUiState::default();
    ratagit_core::initialize_tree_with_initial_expansion(&items, &mut files, false);
    files.filter = FileFilter::Staged;
    ratagit_core::refresh_tree_projection(&items, &mut files);
    assert_eq!(
        ratagit_core::selected_target_paths(&items, &files),
        vec!["src/a.rs"]
    );
    let bounded = ratagit_core::selected_diff_targets_bounded(&items, &files, 10);
    assert_eq!(bounded.targets.len(), 1);
    assert_eq!(bounded.targets[0].path, "src/a.rs");
    files.filter = FileFilter::All;
    files.flat = true;
    ratagit_core::refresh_tree_projection(&items, &mut files);
    assert_eq!(
        files
            .tree_rows
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        vec!["src/a.rs", "src/b.rs", "unknown"]
    );
    assert_eq!(items.len(), 3);
}
