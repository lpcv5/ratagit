use crate::app::{Command, RefreshKind};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct OverlayStore;

impl OverlayStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for OverlayStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::StartCommitInput => {
                if ctx.state.start_commit_editor_guarded() {
                    ctx.state.push_log(
                        "commit: edit message/description then press Enter on message".to_string(),
                        true,
                    );
                    ReduceOutput::none().with_invalidation(UiInvalidation::all())
                } else {
                    ReduceOutput::none()
                }
            }
            DomainAction::CommitAllConfirm(confirmed) => {
                if *confirmed {
                    ctx.state.cancel_input();
                    let paths = ctx.state.all_file_paths();
                    if paths.is_empty() {
                        ctx.state.push_log("nothing to stage".to_string(), false);
                        return ReduceOutput::none();
                    }
                    ReduceOutput::from_command(Command::Effect(EffectRequest::StagePaths(paths)))
                } else {
                    ctx.state.cancel_input();
                    ctx.state
                        .push_log("commit all cancelled".to_string(), false);
                    ReduceOutput::none().with_invalidation(UiInvalidation::all())
                }
            }
            DomainAction::StagePathsFinished { result } => match result {
                Ok(()) => {
                    ctx.state.request_refresh(RefreshKind::StatusOnly);
                    ctx.state.start_commit_editor();
                    ctx.state.push_log(
                        "commit: all files staged; edit message/description then press Enter"
                            .to_string(),
                        true,
                    );
                    ReduceOutput::none().with_invalidation(UiInvalidation::all())
                }
                Err(e) => {
                    ctx.state
                        .push_log(format!("stage all failed: {}", e), false);
                    ReduceOutput::none()
                }
            },
            DomainAction::StartCommandPalette => {
                ctx.state.start_command_palette();
                ctx.state.push_log(
                    "command palette: type command and press Enter".to_string(),
                    true,
                );
                ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay())
            }
            DomainAction::StartBranchCreateInput => {
                ctx.state.start_branch_create_input();
                ctx.state.push_log(
                    "branch create: enter name and press Enter".to_string(),
                    true,
                );
                ReduceOutput::none().with_invalidation(UiInvalidation::all())
            }
            DomainAction::StartStashInput => {
                let targets = ctx.state.prepare_stash_targets_from_selection();
                if targets.is_empty() {
                    ctx.state
                        .push_log("stash blocked: no selected items".to_string(), false);
                    ReduceOutput::none()
                } else {
                    ctx.state.start_stash_editor(targets);
                    ctx.state
                        .push_log("stash: enter title and press Enter".to_string(), true);
                    ReduceOutput::none().with_invalidation(UiInvalidation::all())
                }
            }
            DomainAction::StartSearchInput => {
                ctx.state.start_search_input();
                ctx.state.push_log(
                    "search: type query, Enter confirm, Esc cancel".to_string(),
                    true,
                );
                ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay())
            }
            _ => ReduceOutput::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::InputMode;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_start_commit_input_with_staged_files_opens_editor() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        app.git.status.staged.push(crate::git::FileEntry {
            path: "foo.txt".into(),
            status: crate::git::FileStatus::Modified,
        });
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartCommitInput),
        );
        assert_eq!(app.input.mode, Some(InputMode::CommitEditor));
    }

    #[test]
    fn test_start_commit_input_with_no_files_does_nothing() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartCommitInput),
        );
        assert!(output.commands.is_empty());
        assert!(app.input.mode.is_none());
    }

    #[test]
    fn test_start_command_palette_sets_mode() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartCommandPalette),
        );
        assert_eq!(app.input.mode, Some(InputMode::CommandPalette));
    }

    #[test]
    fn test_start_branch_create_sets_mode() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartBranchCreateInput),
        );
        assert_eq!(app.input.mode, Some(InputMode::CreateBranch));
    }

    #[test]
    fn test_start_search_sets_mode() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartSearchInput),
        );
        assert_eq!(app.input.mode, Some(InputMode::Search));
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
        assert_eq!(app.input.mode, None);
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::app::{InputMode, SidePanel};
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use crate::git::FileStatus;
    use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_commit_all_confirm_true_stages_all_and_commits() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        app.input.mode = Some(InputMode::CommitAllConfirm);
        // Set up unstaged files to stage
        app.git.status.unstaged = vec![crate::git::FileEntry {
            path: "foo.txt".into(),
            status: FileStatus::Modified,
        }];
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CommitAllConfirm(true)),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_commit_all_confirm_false_cancels() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        app.input.mode = Some(InputMode::CommitAllConfirm);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CommitAllConfirm(false)),
        );
        assert!(app.input.mode.is_none());
    }

    #[test]
    fn test_start_stash_input_with_selected_file() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        app.ui.active_panel = SidePanel::Files;
        app.ui.files.tree_nodes = vec![FileTreeNode {
            path: "foo.txt".into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }];
        app.ui.files.panel.list_state.select(Some(0));
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartStashInput),
        );
        assert_eq!(app.input.mode, Some(InputMode::StashEditor));
    }
}
