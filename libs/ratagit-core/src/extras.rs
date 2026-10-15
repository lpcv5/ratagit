use crate::{AppContext, Command, FileRowKind, PanelFocus, push_notice, with_pending};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepositoryOperation {
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
    let unstage = state.repo.files.items.iter().all(|file| file.staged);
    let paths = state
        .repo
        .files
        .items
        .iter()
        .filter(|file| file.staged == unstage)
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
    state.last_operation = Some(format!("{operation:?}").to_lowercase());
    match result {
        Ok(()) => {
            push_notice(state, &format!("{operation:?} completed"));
            crate::details::clear_caches(state);
            with_pending(state, Command::refresh_all_commands())
        }
        Err(error) => {
            state.repo.status.last_error = Some(error.clone());
            push_notice(state, &error);
            Vec::new()
        }
    }
}
