use ratagit_core::custom_patch::CustomPatchAction;
use ratagit_core::patch::{Patch, PatchRequest};
use ratagit_core::workflow::{Dialog, Feature, Operation, WorkflowAction};
use ratagit_core::{
    Action, AppContext, BranchesSubview, Command, CommitEntry, CommitFileEntry, CommitFileStatus,
    CommitFilesUiState, CommitHashStatus, GitResult, PanelFocus, UiAction,
    initialize_commit_files_tree, select_commit_file_tree_path, update,
};

fn file(path: &str, status: CommitFileStatus, old: Option<&str>) -> CommitFileEntry {
    CommitFileEntry {
        path: path.into(),
        old_path: old.map(Into::into),
        status,
    }
}

fn context(branch: bool, files: Vec<CommitFileEntry>) -> AppContext {
    let mut state = AppContext::default();
    let mut ui = CommitFilesUiState {
        active: true,
        commit_id: Some("captured".into()),
        ..Default::default()
    };
    initialize_commit_files_tree(&files, &mut ui);
    if branch {
        state.ui.focus = PanelFocus::Branches;
        state.ui.last_left_focus = PanelFocus::Branches;
        state.ui.branches.subview = BranchesSubview::CommitFiles;
        state.ui.branches.subview_branch = Some("topic".into());
        state.ui.branches.commit_files = ui;
        state.repo.branches.commit_files.items = files;
    } else {
        state.ui.focus = PanelFocus::Commits;
        state.ui.last_left_focus = PanelFocus::Commits;
        state.ui.commits.files = ui;
        state.repo.commits.files.items = files;
    }
    state
}

fn ui(state: &mut AppContext, action: UiAction) -> Vec<Command> {
    update(state, Action::Ui(action))
}

#[test]
fn opening_another_commit_clears_rows_before_the_new_result_arrives() {
    let mut state = context(
        false,
        vec![file("stale.txt", CommitFileStatus::Modified, None)],
    );
    state.repo.commits.items = vec![CommitEntry {
        id: "new".into(),
        full_id: "new-full".into(),
        summary: "new".into(),
        message: "new".into(),
        author_name: "test".into(),
        graph: "*".into(),
        hash_status: CommitHashStatus::Unpushed,
        is_merge: false,
    }];
    assert_eq!(
        ui(&mut state, UiAction::OpenCommitFilesPanel),
        vec![Command::RefreshCommitFiles {
            commit_id: "new-full".into(),
        }]
    );
    assert!(state.repo.commits.files.items.is_empty());
    assert!(state.ui.commits.files.tree_rows.is_empty());
    update(
        &mut state,
        Action::GitResult(GitResult::CommitFiles {
            commit_id: "captured".into(),
            result: Ok(vec![file("late.txt", CommitFileStatus::Added, None)]),
        }),
    );
    assert!(state.repo.commits.files.items.is_empty());
    assert!(state.work.commit_files.commit_files_loading);
}

#[test]
fn pending_file_lists_block_historical_actions_and_custom_patch_for_both_owners() {
    for branch in [false, true] {
        let mut state = context(
            branch,
            vec![file("stale.txt", CommitFileStatus::Modified, None)],
        );
        state.work.commit_files.commit_files_loading = true;
        for feature in [
            Feature::CommitFileCheckout,
            Feature::CommitFileDiscard,
            Feature::CommitFileCopy,
            Feature::CommitFilePatch,
            Feature::CommitFilePatchAll,
        ] {
            assert!(
                ui(
                    &mut state,
                    UiAction::Workflow(WorkflowAction::Open(feature))
                )
                .is_empty()
            );
            assert!(state.ui.workflow.dialog.is_none());
        }
        assert!(
            ui(
                &mut state,
                UiAction::CustomPatch(CustomPatchAction::ToggleFiles { all: true })
            )
            .is_empty()
        );
        assert!(ui(&mut state, UiAction::OpenCommitFilePatch).is_empty());
        assert!(!state.ui.details.patch.active);
    }
}

#[test]
fn contextual_refresh_retains_commit_selection_collapsed_dirs_and_batch_selection() {
    for branch in [false, true] {
        let files = vec![
            file("a/one", CommitFileStatus::Added, None),
            file("b/two", CommitFileStatus::Added, None),
        ];
        let mut state = context(branch, files.clone());
        let selected = if branch {
            &mut state.ui.branches.commit_files
        } else {
            &mut state.ui.commits.files
        };
        selected.expanded_dirs.remove("a");
        ratagit_core::refresh_commit_files_tree_projection(&files, selected);
        assert!(select_commit_file_tree_path(&files, selected, "b/two"));
        selected.scroll_offset = 1;
        selected.selected_rows.insert("b/two".into());
        let before = selected.clone();
        let commands = ui(&mut state, UiAction::RefreshCommitFiles);
        assert_eq!(
            commands,
            vec![if branch {
                Command::RefreshBranchCommitFiles {
                    branch: "topic".into(),
                    commit_id: "captured".into(),
                }
            } else {
                Command::RefreshCommitFiles {
                    commit_id: "captured".into(),
                }
            }]
        );
        let result = if branch {
            GitResult::BranchCommitFiles {
                branch: "topic".into(),
                commit_id: "captured".into(),
                result: Ok(files),
            }
        } else {
            GitResult::CommitFiles {
                commit_id: "captured".into(),
                result: Ok(files),
            }
        };
        update(&mut state, Action::GitResult(result));
        let after = if branch {
            &state.ui.branches.commit_files
        } else {
            &state.ui.commits.files
        };
        assert_eq!(after.commit_id, before.commit_id);
        assert_eq!(after.selected, before.selected);
        assert_eq!(after.scroll_offset, before.scroll_offset);
        assert_eq!(after.expanded_dirs, before.expanded_dirs);
        assert_eq!(after.selected_rows, before.selected_rows);
        assert!(after.active);
    }
}

#[test]
fn copied_sources_are_excluded_from_checkout_patch_builder_and_passive_details() {
    for status in [CommitFileStatus::Copied, CommitFileStatus::Renamed] {
        for branch in [false, true] {
            let mut state = context(branch, vec![file("destination", status, Some("source"))]);
            ui(
                &mut state,
                UiAction::Workflow(WorkflowAction::Open(Feature::CommitFileCheckout)),
            );
            let Some(Dialog::Confirm(Operation::CommitFileCheckout { paths, .. })) =
                state.ui.workflow.dialog.take()
            else {
                panic!("expected captured checkout")
            };
            let expected: Vec<String> = if status == CommitFileStatus::Renamed {
                vec!["destination".into(), "source".into()]
            } else {
                vec!["destination".into()]
            };
            assert_eq!(paths, expected);
            let commands = ui(
                &mut state,
                UiAction::CustomPatch(CustomPatchAction::ToggleFiles { all: true }),
            );
            let Command::LoadCustomPatch {
                request: PatchRequest::CommitFiles { paths, .. },
                ..
            } = &commands[0]
            else {
                panic!("expected custom patch load")
            };
            assert_eq!(
                paths,
                &expected
                    .iter()
                    .map(Into::into)
                    .collect::<Vec<std::path::PathBuf>>()
            );
            let commands = ui(&mut state, UiAction::RefreshCommitFiles);
            assert_eq!(commands.len(), 1);
            let result = if branch {
                GitResult::BranchCommitFiles {
                    branch: "topic".into(),
                    commit_id: "captured".into(),
                    result: Ok(vec![file("destination", status, Some("source"))]),
                }
            } else {
                GitResult::CommitFiles {
                    commit_id: "captured".into(),
                    result: Ok(vec![file("destination", status, Some("source"))]),
                }
            };
            let commands = update(&mut state, Action::GitResult(result));
            let Command::RefreshCommitFileDiff { target, .. } = &commands[0] else {
                panic!("expected passive details load")
            };
            assert_eq!(
                target.paths[0].old_path.as_deref(),
                (status == CommitFileStatus::Renamed).then_some("source")
            );
        }
    }
}

#[test]
fn successful_history_rewrite_closes_context_but_failed_rewrite_retains_it() {
    for branch in [false, true] {
        for success in [false, true] {
            let mut state = context(branch, vec![file("file", CommitFileStatus::Modified, None)]);
            let operation = Operation::CommitFileDiscard {
                oid: "captured".into(),
                paths: vec!["file".into()],
            };
            update(
                &mut state,
                Action::GitResult(GitResult::RepositoryOperation {
                    operation: ratagit_core::extras::RepositoryOperation::Workflow(operation),
                    result: if success {
                        Ok(())
                    } else {
                        Err("conflict".into())
                    },
                }),
            );
            let active = if branch {
                state.ui.branches.commit_files.active
            } else {
                state.ui.commits.files.active
            };
            assert_eq!(active, !success);
        }
    }
}

#[test]
fn historical_next_previous_file_keeps_left_selection_visible_and_skips_directories() {
    for branch in [false, true] {
        let mut state = context(
            branch,
            (0..12)
                .map(|n| file(&format!("dir{n:02}/file"), CommitFileStatus::Modified, None))
                .collect(),
        );
        let (items, selected) = if branch {
            (
                &state.repo.branches.commit_files.items,
                &mut state.ui.branches.commit_files,
            )
        } else {
            (&state.repo.commits.files.items, &mut state.ui.commits.files)
        };
        assert!(select_commit_file_tree_path(items, selected, "dir04/file"));
        selected.scroll_offset = selected.selected;
        for up in [false, false, false, true, true, true, true] {
            state.ui.focus = PanelFocus::Details;
            state.ui.details.patch.active = true;
            state.repo.details.patch = Some(Box::new(Patch {
                request: PatchRequest::CommitFiles {
                    oid: "captured".into(),
                    paths: vec![],
                },
                files: vec![],
                truncated: false,
                preview: None,
            }));
            let commands = ui(
                &mut state,
                UiAction::NextPatchFileInViewport {
                    up,
                    visible_lines: 3,
                },
            );
            assert!(matches!(commands.as_slice(), [Command::LoadPatch { .. }]));
            let selected = if branch {
                &state.ui.branches.commit_files
            } else {
                &state.ui.commits.files
            };
            assert!(
                selected.tree_rows[selected.selected]
                    .path
                    .ends_with("/file")
            );
            assert!(selected.selected >= selected.scroll_offset);
            assert!(selected.selected < selected.scroll_offset + 3);
        }
    }
}
