use super::*;
use crate::workflow::{Choice, ChoiceAction, Dialog, Operation, commit_files::PatchAction};
use crate::{AppContext, Command, push_notice};

pub fn update(state: &mut AppContext, action: CustomPatchAction) -> Vec<Command> {
    use CustomPatchAction as A;
    if state.work.mutation.operation_pending.is_some() {
        return Vec::new();
    }
    let intent = match action {
        A::ToggleFiles { all } => {
            let Some((oid, items, _)) = crate::workflow::commit_files::selection(state) else {
                return Vec::new();
            };
            let oid = oid.to_string();
            let paths = if all {
                items
                    .iter()
                    .flat_map(|file| {
                        [
                            Some(file.path.clone()),
                            file.renamed_from().map(str::to_owned),
                        ]
                    })
                    .flatten()
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect()
            } else {
                crate::workflow::commit_files::selected_paths(state, true)
            };
            if paths.is_empty() {
                return Vec::new();
            }
            Intent::Files {
                oid,
                paths: paths.into_iter().map(Into::into).collect(),
            }
        }
        A::ToggleLines => {
            if !crate::workflow::commit_files::is_patch(state)
                || state.work.details.patch_request.is_some()
            {
                return Vec::new();
            }
            Intent::Lines {
                patch: state.repo.details.patch.as_ref().unwrap().clone(),
                ids: crate::partial::selected_ids(state),
            }
        }
        A::Options => Intent::Options,
        A::Preview => Intent::Preview,
        A::ClosePreview => {
            close_preview(state);
            return Vec::new();
        }
        A::Scroll { up, lines } => {
            let max = state
                .repo
                .custom_patch
                .preview
                .split(|b| *b == b'\n')
                .take(PREVIEW_LINES)
                .count()
                .saturating_sub(state.ui.details.viewport_lines.max(1));
            state.ui.custom_patch.scroll = if up {
                state.ui.custom_patch.scroll.saturating_sub(lines)
            } else {
                state.ui.custom_patch.scroll.saturating_add(lines).min(max)
            };
            return Vec::new();
        }
        A::Reset => {
            reset(state);
            return Vec::new();
        }
        A::Keep => return Vec::new(),
        A::Replace(intent) => {
            reset(state);
            return enqueue(state, *intent);
        }
        A::MenuKey(key) => {
            let Some(Dialog::Menu {
                title,
                selected,
                choices,
            }) = state.ui.workflow.dialog.as_mut()
            else {
                return Vec::new();
            };
            if *title != OPTIONS_TITLE {
                return Vec::new();
            }
            let Some(index) = ['c', 'a', 'r', 'd', 'p', 'y']
                .iter()
                .position(|ch| *ch == key)
            else {
                return Vec::new();
            };
            if index >= choices.len() {
                return Vec::new();
            }
            *selected = index;
            return crate::workflow::update(state, crate::workflow::WorkflowAction::Confirm);
        }
    };
    enqueue(state, intent)
}

fn enqueue(state: &mut AppContext, intent: Intent) -> Vec<Command> {
    if matches!(intent, Intent::Options | Intent::Preview) {
        let requested = if intent == Intent::Options {
            Output::Options
        } else {
            Output::Preview
        };
        if let Some(Pending::Render { output, .. }) = state.work.custom_patch.pending.as_mut() {
            *output = requested;
            return Vec::new();
        }
        state
            .work
            .custom_patch
            .queue
            .retain(|queued| !matches!(queued, Intent::Options | Intent::Preview));
    }
    let oid = match &intent {
        Intent::Files { oid, .. } => Some(oid.as_str()),
        Intent::Lines { patch, .. } => source(&patch.request),
        _ => None,
    };
    if let Some(oid) = oid {
        if state
            .ui
            .custom_patch
            .source
            .as_deref()
            .is_some_and(|old| old != oid)
        {
            state.ui.workflow.dialog = Some(Dialog::Menu {
                title: "Replace custom patch from another commit?",
                choices: vec![
                    Choice {
                        label: "Keep existing patch (Esc)".into(),
                        action: ChoiceAction::CustomPatch(CustomPatchAction::Keep),
                    },
                    Choice {
                        label: format!("Discard existing patch and select from {oid}"),
                        action: ChoiceAction::CustomPatch(CustomPatchAction::Replace(Box::new(
                            intent,
                        ))),
                    },
                ],
                selected: 0,
            });
            return Vec::new();
        }
        state.ui.custom_patch.source = Some(oid.to_string());
    }
    if state.work.custom_patch.queue.len() >= 64 {
        push_notice(
            state,
            "Custom patch selection queue is full; wait for loading to finish",
        );
        return Vec::new();
    }
    state.work.custom_patch.queue.push_back(intent);
    drive(state)
}

fn drive(state: &mut AppContext) -> Vec<Command> {
    if state.work.custom_patch.pending.is_some() {
        return Vec::new();
    }
    while let Some(intent) = state.work.custom_patch.queue.pop_front() {
        match intent {
            Intent::Files { oid, paths } => {
                state.ui.custom_patch.source = Some(oid.clone());
                let request = PatchRequest::CommitFiles { oid, paths };
                let id = next_id(state);
                state.work.custom_patch.pending = Some(Pending::Load {
                    id,
                    request: request.clone(),
                });
                return vec![Command::LoadCustomPatch {
                    request_id: id,
                    request,
                }];
            }
            Intent::Lines { patch, ids } => {
                if state.ui.custom_patch.source.as_deref() != source(&patch.request) {
                    push_notice(state, "Custom patch source mismatch");
                    continue;
                }
                if let Err(error) = selection::lines(state, &patch, &ids) {
                    push_notice(state, &error);
                }
            }
            Intent::Options | Intent::Preview => match capture(state) {
                Ok(patch) => {
                    let id = next_id(state);
                    let output = if intent == Intent::Options {
                        Output::Options
                    } else {
                        Output::Preview
                    };
                    state.work.custom_patch.pending = Some(Pending::Render {
                        id,
                        patch: patch.clone(),
                        output,
                    });
                    return vec![Command::RenderCustomPatch {
                        request_id: id,
                        patch,
                    }];
                }
                Err(error) => push_notice(state, &error),
            },
        }
    }
    if state.ui.custom_patch.included.is_empty() {
        state.ui.custom_patch.source = None;
        state.repo.custom_patch.files.clear();
    }
    Vec::new()
}

fn next_id(state: &mut AppContext) -> u64 {
    let id = state.work.custom_patch.next_id;
    state.work.custom_patch.next_id = id.wrapping_add(1);
    id
}

pub fn loaded(
    state: &mut AppContext,
    id: u64,
    request: PatchRequest,
    result: Result<Box<Patch>, String>,
) -> Vec<Command> {
    if state.work.custom_patch.pending.as_ref()
        != Some(&Pending::Load {
            id,
            request: request.clone(),
        })
    {
        return Vec::new();
    }
    state.work.custom_patch.pending = None;
    match result {
        Ok(patch) if patch.request == request && !patch.files.is_empty() => {
            if let Err(error) = selection::files(state, &patch) {
                push_notice(state, &error);
            }
        }
        Ok(_) => push_notice(
            state,
            "Custom patch response is empty or has a mismatched target",
        ),
        Err(error) => push_notice(state, &error),
    }
    drive(state)
}

pub fn rendered(
    state: &mut AppContext,
    id: u64,
    patch: Box<CustomPatch>,
    result: Result<Vec<u8>, String>,
) -> Vec<Command> {
    let Some(Pending::Render {
        id: pending,
        patch: target,
        output,
    }) = state.work.custom_patch.pending.as_ref()
    else {
        return Vec::new();
    };
    if *pending != id || target != &patch {
        return Vec::new();
    }
    let output = *output;
    state.work.custom_patch.pending = None;
    match result {
        Ok(bytes) if bytes.len() <= MAX_BYTES => {
            if !state.work.custom_patch.queue.is_empty() {
                state
                    .work
                    .custom_patch
                    .queue
                    .push_back(if output == Output::Options {
                        Intent::Options
                    } else {
                        Intent::Preview
                    });
                return drive(state);
            }
            state.repo.custom_patch.preview = bytes;
            if output == Output::Preview {
                state
                    .ui
                    .custom_patch
                    .preview_return_focus
                    .get_or_insert(state.ui.focus);
                state.ui.custom_patch.scroll = 0;
                state.ui.focus = PanelFocus::Details;
            } else {
                options(state, patch);
            }
        }
        Ok(_) => push_notice(state, "Custom patch output exceeds 16 MiB"),
        Err(error) => push_notice(state, &error),
    }
    drive(state)
}

fn options(state: &mut AppContext, patch: Box<CustomPatch>) {
    let mut choices = vec![Choice {
        label: "c  Clear custom patch".into(),
        action: ChoiceAction::CustomPatch(CustomPatchAction::Reset),
    }];
    for (label, action) in [
        ("a  Apply patch to working tree", PatchAction::Apply),
        ("r  Apply patch in reverse", PatchAction::Reverse),
        ("d  Remove patch from original commit", PatchAction::Discard),
    ] {
        choices.push(Choice {
            label: label.into(),
            action: ChoiceAction::Run(Operation::CustomPatch {
                patch: patch.clone(),
                action,
            }),
        });
    }
    choices.push(Choice {
        label: "p  Preview accumulated patch (Tab returns)".into(),
        action: ChoiceAction::CustomPatch(CustomPatchAction::Preview),
    });
    if let Ok(text) = std::str::from_utf8(&state.repo.custom_patch.preview) {
        choices.push(Choice {
            label: "y  Copy patch to clipboard".into(),
            action: ChoiceAction::Copy(text.into()),
        });
    }
    state.ui.workflow.dialog = Some(Dialog::Menu {
        title: OPTIONS_TITLE,
        choices,
        selected: 0,
    });
}

fn close_preview(state: &mut AppContext) {
    if let Some(focus) = state.ui.custom_patch.preview_return_focus.take() {
        state.ui.focus = focus;
    }
}

pub(super) fn reset(state: &mut AppContext) {
    close_preview(state);
    invalidate_reads(state);
    state.repo.custom_patch = Data::default();
    state.ui.custom_patch = Ui::default();
}

pub fn invalidate_reads(state: &mut AppContext) {
    state.work.custom_patch.pending = None;
    state.work.custom_patch.queue.clear();
    if state.ui.custom_patch.included.is_empty() {
        state.ui.custom_patch.source = None;
        state.repo.custom_patch.files.clear();
    }
}
