use super::*;

pub(super) fn open(state: &mut AppContext, feature: Feature) -> Vec<Command> {
    use Feature as F;
    if feature == F::CustomPatchPreview {
        return crate::custom_patch::update(
            state,
            if state.ui.custom_patch.preview_return_focus.is_some() {
                crate::custom_patch::CustomPatchAction::ClosePreview
            } else {
                crate::custom_patch::CustomPatchAction::Preview
            },
        );
    }
    if feature == F::CustomPatchOptions {
        return crate::custom_patch::update(state, crate::custom_patch::CustomPatchAction::Options);
    }
    if feature == F::Sequence {
        menu(
            state,
            "Merge / rebase / cherry-pick / revert",
            [
                ("Continue", SequenceAction::Continue),
                ("Abort", SequenceAction::Abort),
                ("Skip", SequenceAction::Skip),
            ]
            .into_iter()
            .map(|(label, action)| run_choice(label, Operation::Sequence(action)))
            .collect(),
        );
        return Vec::new();
    }
    if matches!(
        state.active_left_view(),
        Some(crate::ActiveLeftView::CommitFiles | crate::ActiveLeftView::BranchCommitFiles)
    ) {
        return commit_files::open(state, feature);
    }
    if state.active_left_view() == Some(crate::ActiveLeftView::Files) {
        return files::open(state, feature);
    }
    if state.active_left_view() == Some(crate::ActiveLeftView::BranchesList) {
        return branches(state, feature);
    }
    let ids = selected_commit_ids(state);
    if feature == F::ClearCopied {
        state.ui.workflow.copied_commits.clear();
        return Vec::new();
    }
    if feature == F::PasteCommits {
        if state.ui.workflow.copied_commits.is_empty() {
            crate::push_notice(state, "Copy commits with C first");
            return Vec::new();
        }
        return request(
            state,
            Operation::CherryPick(state.ui.workflow.copied_commits.clone()),
        );
    }
    let Some(id) = ids.first().cloned() else {
        return Vec::new();
    };
    if feature == F::CopyCommits {
        // Lists are newest first; Git must apply oldest first.
        state.ui.workflow.copied_commits = ids.into_iter().rev().collect();
        crate::push_notice(state, "Commits copied; switch branch then V to cherry-pick");
        return Vec::new();
    }
    if ids.len() != 1 {
        crate::push_notice(state, "Select one commit for this operation");
        return Vec::new();
    }
    match feature {
        F::CopyAttributes => {
            let commit = match state.active_commit_list_target() {
                Some(crate::CommitListTarget::Branch) => state
                    .repo
                    .branches
                    .commits
                    .get(state.ui.branches.commits.selected),
                _ => state.repo.commits.items.get(state.ui.commits.selected),
            };
            if let Some(commit) = commit {
                let choices = [
                    ("Hash", crate::commit_key(commit)),
                    ("Short hash", commit.id.clone()),
                    ("Message", commit.message.clone()),
                    ("Author", commit.author_name.clone()),
                ]
                .into_iter()
                .map(|(label, value)| Choice {
                    label: label.into(),
                    action: ChoiceAction::Copy(value),
                })
                .collect();
                menu(state, "Copy commit attribute", choices);
            }
        }
        F::Reset => reset_menu(state, id),
        F::BranchFromCommit => prompt::open(state, PromptKind::Branch, id),
        F::Tag => prompt::open(state, PromptKind::Tag, id),
        F::Revert => return request(state, Operation::Revert(id)),
        F::MoveUp | F::MoveDown => {
            return request(
                state,
                Operation::MoveCommit {
                    id,
                    up: feature == F::MoveUp,
                },
            );
        }
        F::Fixup => return request(state, Operation::Fixup(id)),
        F::Autosquash => return request(state, Operation::Autosquash(id)),
        _ => {}
    }
    Vec::new()
}

fn branches(state: &mut AppContext, feature: Feature) -> Vec<Command> {
    use Feature as F;
    if feature == F::RemoteBranches {
        state.ui.workflow.next_request = state.ui.workflow.next_request.wrapping_add(1);
        let request_id = state.ui.workflow.next_request;
        state.ui.workflow.remote_request = Some(request_id);
        return vec![Command::RemoteBranches { request_id }];
    }
    if feature == F::CheckoutByName {
        prompt::open(state, PromptKind::Checkout, String::new());
        return Vec::new();
    }
    if feature == F::CheckoutPrevious {
        return checkout(state, "-".into());
    }
    let Some(branch) = crate::selected_branch_name(state) else {
        return Vec::new();
    };
    match feature {
        F::Merge => menu(
            state,
            "Merge selected branch into current",
            vec![
                run_choice(
                    "Merge",
                    Operation::Merge {
                        target: branch.clone(),
                        squash: false,
                    },
                ),
                run_choice(
                    "Squash merge (stage changes without committing)",
                    Operation::Merge {
                        target: branch,
                        squash: true,
                    },
                ),
            ],
        ),
        F::Rename => prompt::open(state, PromptKind::Rename, branch),
        F::Upstream => menu(
            state,
            "Upstream options",
            vec![
                Choice {
                    label: "Set upstream (remote/branch)".into(),
                    action: ChoiceAction::Prompt {
                        kind: PromptKind::Upstream,
                        target: branch.clone(),
                    },
                },
                run_choice(
                    "Unset upstream",
                    Operation::Upstream {
                        branch: branch.clone(),
                        target: None,
                    },
                ),
                run_choice(
                    "Fast-forward from upstream",
                    Operation::FastForward { branch },
                ),
                Choice {
                    label: "Reset current branch to its upstream".into(),
                    action: ChoiceAction::Run(Operation::Reset {
                        target: "@{upstream}".into(),
                        mode: ResetMode::Hard,
                    }),
                },
            ],
        ),
        F::FastForward => return request(state, Operation::FastForward { branch }),
        _ => {}
    }
    Vec::new()
}

pub(super) fn checkout(state: &mut AppContext, name: String) -> Vec<Command> {
    if crate::selectors::repository_has_uncommitted_changes(state) {
        crate::branches::open_auto_stash_confirm(
            state,
            crate::AutoStashOperation::Checkout { branch: name },
        );
        Vec::new()
    } else {
        request(state, Operation::Checkout { name })
    }
}

pub(super) fn reset_menu(state: &mut AppContext, target: String) {
    menu(
        state,
        "Reset current branch to selected target",
        [
            ("Soft (keep staged changes)", ResetMode::Soft),
            ("Mixed (keep worktree changes)", ResetMode::Mixed),
            ("Hard (discard tracked changes)", ResetMode::Hard),
        ]
        .into_iter()
        .map(|(label, mode)| {
            run_choice(
                format!("{label}: {target}"),
                Operation::Reset {
                    target: target.clone(),
                    mode,
                },
            )
        })
        .collect(),
    );
}
