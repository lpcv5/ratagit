use super::*;
use crate::patch::*;
use crate::workflow::{ChoiceAction, Dialog, WorkflowAction};
use crate::{Action, AppContext, Command, UiAction, update as reduce};

fn patch(oid: &str, path: &str) -> Box<Patch> {
    let side = PatchSide {
        path: Some(path.into()),
        oid: None,
        mode: 0o100644,
    };
    let header = format!("diff --git a/{path} b/{path}\n--- a/{path}\n+++ b/{path}\n");
    let hunks = [1, 20]
        .into_iter()
        .map(|n| PatchHunk {
            old_start: n,
            old_lines: 1,
            new_start: n,
            new_lines: 1,
            header: format!("@@ -{n} +{n} @@\n").into_bytes(),
            lines: vec![
                PatchLine {
                    kind: PatchLineKind::Removed,
                    old_line: Some(n),
                    new_line: None,
                    content: b"old\n".to_vec(),
                },
                PatchLine {
                    kind: PatchLineKind::Added,
                    old_line: None,
                    new_line: Some(n),
                    content: b"new\n".to_vec(),
                },
            ],
        })
        .collect();
    Box::new(Patch {
        request: PatchRequest::CommitFiles {
            oid: oid.into(),
            paths: vec![path.into()],
        },
        files: vec![PatchFile {
            header: header.clone().into_bytes(),
            source: PatchSource::Commit(oid.into()),
            change: PatchChange::Modified,
            old: side.clone(),
            new: side,
            binary: false,
            hunks,
            encoded: Some(
                format!("{header}@@ -1 +1 @@\n-old\n+new\n@@ -20 +20 @@\n-old\n+new\n")
                    .into_bytes(),
            ),
        }],
        truncated: false,
        preview: None,
    })
}
#[test]
fn pending_options_can_finish_after_switching_tabs_and_remain_dismissable() {
    let mut state = state();
    action(&mut state, CustomPatchAction::ToggleLines);
    let commands = action(&mut state, CustomPatchAction::Options);
    let Command::RenderCustomPatch { request_id, patch } = commands[0].clone() else {
        panic!()
    };
    reduce(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
    );
    reduce(
        &mut state,
        Action::Ui(UiAction::CyclePanelTab { previous: false }),
    );
    assert_eq!(state.ui.tabs.commits, Some(crate::RepositoryTab::Reflog));
    reduce(
        &mut state,
        Action::GitResult(crate::GitResult::RenderedCustomPatch {
            request_id,
            patch,
            result: Ok(b"patch".to_vec()),
        }),
    );
    assert!(state.ui.workflow.dialog.is_some());
    reduce(
        &mut state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Down)),
    );
    reduce(
        &mut state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Cancel)),
    );
    assert!(state.ui.workflow.dialog.is_none());
}

fn state() -> AppContext {
    let mut state = AppContext::default();
    state.ui.focus = PanelFocus::Details;
    state.ui.last_left_focus = PanelFocus::Commits;
    state.ui.details.patch.active = true;
    state.ui.details.patch.mode = crate::partial::SelectionMode::Hunk;
    state.repo.details.patch = Some(patch("source", "a.txt"));
    state
}
fn action(state: &mut AppContext, action: CustomPatchAction) -> Vec<Command> {
    reduce(state, Action::Ui(UiAction::CustomPatch(action)))
}
fn files_state() -> AppContext {
    let mut state = state();
    state.ui.focus = PanelFocus::Commits;
    state.repo.commits.files.items = ["a.txt", "b.txt"]
        .into_iter()
        .map(|p| crate::CommitFileEntry {
            path: p.into(),
            old_path: None,
            status: crate::CommitFileStatus::Modified,
        })
        .collect();
    state.ui.commits.files.active = true;
    state.ui.commits.files.commit_id = Some("source".into());
    crate::initialize_commit_files_tree(
        &state.repo.commits.files.items,
        &mut state.ui.commits.files,
    );
    state
}
fn complete(state: &mut AppContext, commands: &[Command]) -> Vec<Command> {
    let [
        Command::LoadCustomPatch {
            request_id,
            request,
        },
    ] = commands
    else {
        panic!("missing custom read: {commands:?}");
    };
    let PatchRequest::CommitFiles { oid, paths } = request else {
        panic!();
    };
    let mut loaded_patch = patch(oid, paths[0].to_str().unwrap());
    loaded_patch.request = request.clone();
    loaded_patch.files = paths
        .iter()
        .map(|p| patch(oid, p.to_str().unwrap()).files[0].clone())
        .collect();
    loaded(state, *request_id, request.clone(), Ok(loaded_patch))
}

#[test]
fn hunks_accumulate_across_files_and_repeated_space_removes_without_git_writes() {
    let mut state = state();
    assert!(action(&mut state, CustomPatchAction::ToggleLines).is_empty());
    assert!(
        matches!(&state.ui.custom_patch.included["a.txt"], Inclusion::Lines(lines) if lines.len() == 2)
    );
    state.repo.details.patch = Some(patch("source", "b.txt"));
    action(&mut state, CustomPatchAction::ToggleLines);
    assert_eq!(capture(&state).unwrap().files.len(), 2);
    reduce(&mut state, Action::Ui(UiAction::CloseFilePatch));
    assert_eq!(state.ui.custom_patch.included.len(), 2);
    state.repo.details.patch = Some(patch("source", "a.txt"));
    state.ui.details.patch.active = true;
    action(&mut state, CustomPatchAction::ToggleLines);
    assert!(!state.ui.custom_patch.included.contains_key("a.txt"));
    assert_eq!(state.ui.custom_patch.included.len(), 1);
    assert!(state.work.mutation.operation_pending.is_none());
}

#[test]
fn file_space_promotes_partial_selection_and_a_toggles_all_files() {
    let mut state = files_state();
    action(&mut state, CustomPatchAction::ToggleLines);
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: false });
    complete(&mut state, &commands);
    assert_eq!(state.ui.custom_patch.included["a.txt"], Inclusion::Whole);
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    complete(&mut state, &commands);
    assert_eq!(state.ui.custom_patch.included.len(), 2);
    assert!(
        state
            .ui
            .custom_patch
            .included
            .values()
            .all(|v| *v == Inclusion::Whole)
    );
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    complete(&mut state, &commands);
    assert!(state.ui.custom_patch.included.is_empty());
    assert!(state.ui.custom_patch.source.is_none());
}

#[test]
fn removing_a_hunk_demotes_whole_file_and_line_mode_promotes_it_again() {
    let mut state = files_state();
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: false });
    complete(&mut state, &commands);
    action(&mut state, CustomPatchAction::ToggleLines);
    assert!(
        matches!(&state.ui.custom_patch.included["a.txt"], Inclusion::Lines(lines) if lines.len() == 2 && lines.contains(&(1,0)))
    );
    action(&mut state, CustomPatchAction::ToggleLines);
    assert_eq!(state.ui.custom_patch.included["a.txt"], Inclusion::Whole);
    state.ui.details.patch.mode = crate::partial::SelectionMode::Line;
    action(&mut state, CustomPatchAction::ToggleLines);
    assert!(
        matches!(&state.ui.custom_patch.included["a.txt"], Inclusion::Lines(lines) if lines.len() == 3)
    );
}

#[test]
fn rapid_file_toggles_are_serialized_and_details_reads_do_not_steal_ownership() {
    let mut state = files_state();
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: false });
    crate::select_commit_file_tree_path(
        &state.repo.commits.files.items,
        &mut state.ui.commits.files,
        "b.txt",
    );
    assert!(action(&mut state, CustomPatchAction::ToggleFiles { all: false }).is_empty());
    let details = reduce(&mut state, Action::Ui(UiAction::OpenCommitFilePatch));
    assert!(matches!(details.as_slice(), [Command::LoadPatch { .. }]));
    let queued = complete(&mut state, &commands);
    assert!(matches!(
        queued.as_slice(),
        [Command::LoadCustomPatch { .. }]
    ));
    assert!(state.work.details.patch_request.is_some());
    complete(&mut state, &queued);
    assert_eq!(state.ui.custom_patch.included.len(), 2);
}

#[test]
fn reset_and_mutation_cancel_pending_reads_without_accepting_stale_results() {
    let mut state = files_state();
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: false });
    action(&mut state, CustomPatchAction::Reset);
    complete(&mut state, &commands);
    assert!(state.ui.custom_patch.included.is_empty());
    let current = action(&mut state, CustomPatchAction::ToggleFiles { all: false });
    complete(&mut state, &commands);
    assert!(state.ui.custom_patch.included.is_empty());
    complete(&mut state, &current);
    let before = state.ui.custom_patch.included.clone();
    let pending = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    reduce(&mut state, Action::Ui(UiAction::Pull));
    complete(&mut state, &pending);
    assert_eq!(state.ui.custom_patch.included, before);
    assert!(state.work.custom_patch.pending.is_none());
}

#[test]
fn source_replacement_requires_confirmation_and_keep_preserves_existing_selection() {
    let mut state = state();
    action(&mut state, CustomPatchAction::ToggleLines);
    let before = capture(&state).unwrap();
    state.repo.details.patch = Some(patch("other", "b.txt"));
    action(&mut state, CustomPatchAction::ToggleLines);
    assert!(matches!(
        state.ui.workflow.dialog,
        Some(Dialog::Menu {
            title: "Replace custom patch from another commit?",
            ..
        })
    ));
    reduce(
        &mut state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Confirm)),
    );
    assert_eq!(capture(&state).unwrap(), before);
    action(&mut state, CustomPatchAction::ToggleLines);
    reduce(
        &mut state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Down)),
    );
    reduce(
        &mut state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Confirm)),
    );
    assert_eq!(state.ui.custom_patch.source.as_deref(), Some("other"));
    assert!(!state.ui.custom_patch.included.contains_key("a.txt"));
    assert!(state.ui.custom_patch.included.contains_key("b.txt"));
}

#[test]
fn complete_binary_file_is_supported_but_incomplete_or_truncated_ranges_are_atomic() {
    let mut state = files_state();
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    let [
        Command::LoadCustomPatch {
            request_id,
            request,
        },
    ] = commands.as_slice()
    else {
        panic!();
    };
    let mut data = patch("source", "a.txt");
    let mut binary = patch("source", "b.txt").files.remove(0);
    binary.binary = true;
    binary.hunks.clear();
    binary.encoded = None;
    data.request = request.clone();
    data.files.push(binary);
    loaded(&mut state, *request_id, request.clone(), Ok(data.clone()));
    assert!(state.ui.custom_patch.included.is_empty());
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    let [
        Command::LoadCustomPatch {
            request_id,
            request,
        },
    ] = commands.as_slice()
    else {
        panic!();
    };
    data.files[1].encoded = Some(b"native binary data".to_vec());
    data.truncated = true;
    loaded(&mut state, *request_id, request.clone(), Ok(data.clone()));
    assert!(state.ui.custom_patch.included.is_empty());
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    let [
        Command::LoadCustomPatch {
            request_id,
            request,
        },
    ] = commands.as_slice()
    else {
        panic!();
    };
    data.truncated = false;
    loaded(&mut state, *request_id, request.clone(), Ok(data));
    assert_eq!(capture(&state).unwrap().files.len(), 2);
}

#[test]
fn options_capture_all_inclusion_preview_and_copy_and_ignore_wrong_render_target() {
    let mut state = state();
    action(&mut state, CustomPatchAction::ToggleLines);
    let commands = action(&mut state, CustomPatchAction::Options);
    let [Command::RenderCustomPatch { request_id, patch }] = commands.as_slice() else {
        panic!();
    };
    let mut wrong = patch.clone();
    wrong.oid = "other".into();
    rendered(&mut state, *request_id, wrong, Ok(b"wrong".to_vec()));
    assert!(state.ui.workflow.dialog.is_none());
    rendered(
        &mut state,
        *request_id,
        patch.clone(),
        Ok(b"native patch\n".to_vec()),
    );
    assert!(
        matches!(&state.ui.workflow.dialog,Some(Dialog::Menu { choices, .. }) if matches!(&choices[5].action, ChoiceAction::Copy(text) if text == "native patch\n"))
    );
    action(&mut state, CustomPatchAction::MenuKey('r'));
    assert!(
        matches!(&state.ui.workflow.dialog,Some(Dialog::Confirm(crate::workflow::Operation::CustomPatch { patch: target, action: crate::workflow::commit_files::PatchAction::Reverse })) if target == patch)
    );
}

#[test]
fn custom_read_metadata_uses_independent_ownership_without_debounce_or_mutation() {
    let commands = [
        Command::LoadCustomPatch {
            request_id: 0,
            request: patch("source", "a.txt").request.clone(),
        },
        Command::RenderCustomPatch {
            request_id: 1,
            patch: Box::new(CustomPatch {
                oid: "source".into(),
                files: Vec::new(),
            }),
        },
    ];
    for (command, label) in commands
        .iter()
        .zip(["load_custom_patch", "render_custom_patch"])
    {
        assert_eq!(command.log_label(), label);
        assert!(!command.is_mutating());
        assert_eq!(command.debounce_key(), None);
        assert_eq!(command.refresh_coalescing_key(), None);
        assert_eq!(command.pending_operation_label(), None);
    }
}

#[test]
fn options_wait_for_later_queued_toggles_and_do_not_show_stale_selection() {
    let mut state = files_state();
    let commands = action(&mut state, CustomPatchAction::ToggleFiles { all: false });
    complete(&mut state, &commands);
    let rendering = action(&mut state, CustomPatchAction::Options);
    let [Command::RenderCustomPatch { request_id, patch }] = rendering.as_slice() else {
        panic!();
    };
    let next = action(&mut state, CustomPatchAction::ToggleFiles { all: true });
    assert!(next.is_empty());
    let loading = rendered(
        &mut state,
        *request_id,
        patch.clone(),
        Ok(b"old partial selection".to_vec()),
    );
    assert!(state.ui.workflow.dialog.is_none());
    let latest = complete(&mut state, &loading);
    let [Command::RenderCustomPatch { patch, .. }] = latest.as_slice() else {
        panic!("missing aggregate output: {latest:?}");
    };
    assert_eq!(patch.files.len(), 2);
}

#[test]
fn oversized_selection_is_rejected_without_losing_existing_inclusion() {
    let mut state = state();
    action(&mut state, CustomPatchAction::ToggleLines);
    let before = capture(&state).unwrap();
    let mut large = patch("source", "b.txt");
    large.files[0].encoded = Some(vec![b'x'; MAX_BYTES + 1]);
    state.repo.details.patch = Some(large);
    action(&mut state, CustomPatchAction::ToggleLines);
    assert_eq!(capture(&state).unwrap(), before);
    assert!(state.notices.last().unwrap().contains("16 MiB"));
}
