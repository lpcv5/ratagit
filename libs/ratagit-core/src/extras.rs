use crate::{AppContext, Command, FileRowKind, PanelFocus, push_notice, with_pending};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryOperation {
    Workflow(crate::workflow::Operation),
    Fetch,
    Ignore(String),
    StashApply(String),
    StashDrop(String),
}

pub(crate) fn run(state: &mut AppContext, operation: RepositoryOperation) -> Vec<Command> {
    if state.work.mutation.operation_pending.is_some() {
        return Vec::new();
    }
    with_pending(state, vec![Command::RepositoryOperation(operation)])
}

pub(crate) fn toggle_stage_all(state: &mut AppContext) -> Vec<Command> {
    if state.work.mutation.operation_pending.is_some() || state.repo.files.items.is_empty() {
        return Vec::new();
    }
    if state.repo.files.items.iter().any(|file| file.conflicted) {
        push_notice(state, "Resolve conflicts before staging all files");
        return Vec::new();
    }
    let unstage = state
        .repo
        .files
        .items
        .iter()
        .all(|file| file.staged && !file.unstaged);
    let paths = state
        .repo
        .files
        .items
        .iter()
        .filter(|file| if unstage { file.staged } else { file.unstaged })
        .map(|file| file.path.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    with_pending(
        state,
        vec![if unstage {
            Command::UnstageFiles { paths }
        } else {
            Command::StageFiles { paths }
        }],
    )
}

pub(crate) fn ignore_selected(state: &mut AppContext) -> Vec<Command> {
    if state.ui.focus != PanelFocus::Files {
        return Vec::new();
    }
    let Some(row) = crate::selected_row(&state.repo.files.items, &state.ui.files) else {
        return Vec::new();
    };
    if row.kind == FileRowKind::File
        && !state
            .repo
            .files
            .items
            .iter()
            .any(|file| file.path == row.path && file.untracked)
    {
        push_notice(
            state,
            "Ignore is available for untracked files and directories",
        );
        return Vec::new();
    }
    run(state, RepositoryOperation::Ignore(row.path))
}

pub(crate) fn stash(state: &mut AppContext, drop: bool) -> Vec<Command> {
    if state.ui.focus != PanelFocus::Stash {
        return Vec::new();
    }
    let Some(id) = crate::selected_stash_id(state) else {
        return Vec::new();
    };
    if drop {
        state.ui.details.repository_confirm = Some(RepositoryOperation::StashDrop(id));
        Vec::new()
    } else {
        run(state, RepositoryOperation::StashApply(id))
    }
}

pub(crate) fn completed(
    state: &mut AppContext,
    operation: RepositoryOperation,
    result: Result<(), String>,
) -> Vec<Command> {
    state
        .work
        .record_operation_completed("repository_operation");
    if let RepositoryOperation::Workflow(ref op) = operation {
        use crate::workflow::{Operation as O, commit_files::PatchAction};
        let historical = matches!(
            op,
            O::CommitFileCheckout { .. }
                | O::CommitFileDiscard { .. }
                | O::CommitPatch { .. }
                | O::CustomPatch { .. }
        );
        if historical {
            state.last_operation = Some(op.description().to_lowercase());
            crate::details::clear_caches(state);
            let rewrite = matches!(
                op,
                O::CommitFileDiscard { .. }
                    | O::CommitPatch {
                        action: PatchAction::Discard,
                        ..
                    }
                    | O::CustomPatch {
                        action: PatchAction::Discard,
                        ..
                    }
            );
            match result {
                Ok(()) => {
                    state.repo.status.last_error = None;
                    push_notice(state, &format!("{} completed", op.description()));
                    if rewrite {
                        crate::custom_patch::update(
                            state,
                            crate::custom_patch::CustomPatchAction::Reset,
                        );
                        if state.ui.details.patch.active {
                            crate::partial::close(state);
                        }
                        return with_pending(state, Command::refresh_all_commands());
                    }
                }
                Err(error) => {
                    state.repo.status.last_error = Some(error.clone());
                    push_notice(state, &error);
                }
            }
            return with_pending(state, vec![Command::RefreshFiles]);
        }
    }
    state.last_operation = Some(format!("{operation:?}").to_lowercase());
    match result {
        Ok(()) => {
            if matches!(
                operation,
                RepositoryOperation::Workflow(crate::workflow::Operation::CherryPick(_))
            ) {
                state.ui.workflow.copied_commits.clear();
            }
            let message = match &operation {
                RepositoryOperation::Workflow(operation) => {
                    format!("{} completed", operation.description())
                }
                _ => format!("{operation:?} completed"),
            };
            push_notice(state, &message);
            crate::details::clear_caches(state);
            with_pending(state, Command::refresh_all_commands())
        }
        Err(error) => {
            state.repo.status.last_error = Some(error.clone());
            push_notice(state, &error);
            // A failed merge, cherry-pick or revert may leave a useful conflict
            // state. Surface it instead of keeping the pre-operation snapshot.
            crate::details::clear_caches(state);
            with_pending(state, Command::refresh_all_commands())
        }
    }
}
