//! Historical file operations capture their targets before opening dialogs.
use super::*;
use crate::patch::{Patch, PatchDirection, PatchRequest, PatchSource};
use crate::{CommitFileEntry, CommitFilesTarget, CommitFilesUiState, PanelFocus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatchAction {
    Apply,
    Reverse,
    Discard,
}

pub fn selection(state: &AppContext) -> Option<(&str, &[CommitFileEntry], &CommitFilesUiState)> {
    let (items, ui) = match state.details_commit_files_target()? {
        CommitFilesTarget::Main => (&state.repo.commits.files.items, &state.ui.commits.files),
        CommitFilesTarget::Branch => (
            &state.repo.branches.commit_files.items,
            &state.ui.branches.commit_files,
        ),
    };
    ui.active
        .then_some((ui.commit_id.as_deref()?, items.as_slice(), ui))
}

pub fn paths(state: &AppContext) -> Vec<String> {
    selected_paths(state, false)
}

pub(crate) fn selected_paths(state: &AppContext, include_old: bool) -> Vec<String> {
    let Some((_, items, ui)) = selection(state) else {
        return Vec::new();
    };
    let targets = crate::selected_commit_file_targets(items, ui);
    let target_paths = targets
        .iter()
        .map(|target| target.path.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    items
        .iter()
        .filter(|item| {
            std::iter::successors(Some(item.path.as_str()), |path| {
                path.rsplit_once('/').map(|(parent, _)| parent)
            })
            .any(|path| target_paths.contains(path))
        })
        .flat_map(|item| {
            [
                Some(item.path.clone()),
                if include_old {
                    item.old_path.clone()
                } else {
                    None
                },
            ]
        })
        .flatten()
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub fn is_patch(state: &AppContext) -> bool {
    state.ui.details.patch.active
        && state.repo.details.patch.as_ref().is_some_and(|patch| {
            matches!(
                patch.request,
                PatchRequest::Commit { .. } | PatchRequest::CommitFiles { .. }
            )
        })
}

pub(super) fn open(state: &mut AppContext, feature: Feature) -> Vec<Command> {
    let Some((oid, _, _)) = selection(state) else {
        return Vec::new();
    };
    let oid = oid.to_string();
    let paths = selected_paths(state, feature != Feature::CommitFileCopy);
    if paths.is_empty() {
        return Vec::new();
    }
    match feature {
        Feature::CommitFileCheckout => request(state, Operation::CommitFileCheckout { oid, paths }),
        Feature::CommitFileDiscard => request(state, Operation::CommitFileDiscard { oid, paths }),
        Feature::CommitFilePatch | Feature::CommitFilePatchAll => crate::custom_patch::update(
            state,
            crate::custom_patch::CustomPatchAction::ToggleFiles {
                all: feature == Feature::CommitFilePatchAll,
            },
        ),
        Feature::CommitFileCopy => {
            menu(
                state,
                "Copy selected paths",
                vec![Choice {
                    label: paths.join(", "),
                    action: ChoiceAction::Copy(paths.join("\n")),
                }],
            );
            Vec::new()
        }
        _ => Vec::new(),
    }
}

pub(crate) fn enter(state: &mut AppContext) -> Vec<Command> {
    if state.work.mutation.operation_pending.is_some() {
        return Vec::new();
    }
    let Some((oid, items, ui)) = selection(state) else {
        return Vec::new();
    };
    let oid = oid.to_string();
    let paths: Vec<String> = {
        let Some(file) = crate::selected_commit_file(items, ui) else {
            let action = match state.details_commit_files_target() {
                Some(CommitFilesTarget::Branch) => {
                    crate::UiAction::ToggleBranchCommitFilesDirectory
                }
                _ => crate::UiAction::ToggleCommitFilesDirectory,
            };
            return crate::update(state, crate::Action::Ui(action));
        };
        [Some(file.path), file.old_path]
            .into_iter()
            .flatten()
            .collect()
    };
    if paths.is_empty() {
        return Vec::new();
    }
    let focus = match state.details_commit_files_target() {
        Some(CommitFilesTarget::Branch) => PanelFocus::Branches,
        _ => PanelFocus::Commits,
    };
    state.ui.details.patch = crate::partial::PatchUiState {
        active: true,
        mode: crate::partial::SelectionMode::Hunk,
        ..Default::default()
    };
    state.ui.last_left_focus = focus;
    state.ui.focus = PanelFocus::Details;
    state.ui.details.scroll_offset = 0;
    state.repo.details.patch = None;
    state.repo.details.files_error = None;
    crate::details::clear_details_pending(state);
    crate::partial::load(
        state,
        PatchRequest::CommitFiles {
            oid,
            paths: paths.into_iter().map(Into::into).collect(),
        },
    )
}

pub fn validate(
    patch: &Patch,
    ids: &[crate::patch::LineId],
    action: PatchAction,
) -> Result<(), String> {
    if !matches!(
        patch.request,
        PatchRequest::Commit { .. } | PatchRequest::CommitFiles { .. }
    ) || patch
        .files
        .iter()
        .any(|file| !matches!(file.source, PatchSource::Commit(_)))
    {
        return Err("Select a historical commit patch".into());
    }
    // Reuse the native text/metadata/line guards with the appropriate base side.
    let mut reviewed = patch.clone();
    for file in &mut reviewed.files {
        file.source = PatchSource::Worktree;
    }
    reviewed.validate_lines(
        ids,
        if action == PatchAction::Apply {
            PatchDirection::Stage
        } else {
            PatchDirection::Discard
        },
    )
}

pub(crate) fn discard_hunks(state: &mut AppContext) {
    if state.work.details.patch_request.is_some() || state.work.mutation.operation_pending.is_some()
    {
        return;
    }
    let Some(patch) = state.repo.details.patch.clone() else {
        return;
    };
    let ids = crate::partial::selected_ids(state);
    if let Err(error) = validate(&patch, &ids, PatchAction::Discard) {
        crate::push_notice(state, &error);
        return;
    }
    state.ui.workflow.dialog = Some(Dialog::Confirm(Operation::CommitPatch {
        patch,
        ids,
        action: PatchAction::Discard,
    }));
}

pub(crate) fn next_file(state: &mut AppContext, up: bool) -> Vec<Command> {
    crate::partial::close(state);
    let branch = state.details_commit_files_target() == Some(CommitFilesTarget::Branch);
    let (items, ui) = if branch {
        (
            &state.repo.branches.commit_files.items,
            &mut state.ui.branches.commit_files,
        )
    } else {
        (&state.repo.commits.files.items, &mut state.ui.commits.files)
    };
    let original = ui.selected;
    loop {
        let before = ui.selected;
        crate::move_commit_file_selected(ui, up);
        if ui.selected == before {
            ui.selected = original;
            break;
        }
        if crate::selected_commit_file(items, ui).is_some() {
            break;
        }
    }
    enter(state)
}
