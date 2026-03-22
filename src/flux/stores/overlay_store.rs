use crate::app::Command;
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
                ReduceOutput::from_command(Command::Effect(EffectRequest::StartCommitEditorGuarded))
            }
            DomainAction::CommitAllConfirm(confirmed) => {
                if *confirmed {
                    ReduceOutput::from_command(Command::Effect(
                        EffectRequest::StageAllAndStartCommitEditor,
                    ))
                } else {
                    ctx.app.cancel_input();
                    ctx.app.push_log("commit all cancelled", false);
                    ReduceOutput::none().with_invalidation(UiInvalidation::all())
                }
            }
            DomainAction::StartCommandPalette => {
                ctx.app.start_command_palette();
                ctx.app
                    .push_log("command palette: type command and press Enter", true);
                ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay())
            }
            DomainAction::StartBranchCreateInput => {
                ctx.app.start_branch_create_input();
                ctx.app
                    .push_log("branch create: enter name and press Enter", true);
                ReduceOutput::none().with_invalidation(UiInvalidation::all())
            }
            DomainAction::StartStashInput => {
                let targets = ctx.app.prepare_stash_targets_from_selection();
                if targets.is_empty() {
                    ctx.app.push_log("stash blocked: no selected items", false);
                    ReduceOutput::none()
                } else {
                    ctx.app.start_stash_editor(targets);
                    ctx.app.push_log("stash: enter title and press Enter", true);
                    ReduceOutput::none().with_invalidation(UiInvalidation::all())
                }
            }
            DomainAction::StartSearchInput => {
                ctx.app.start_search_input();
                ctx.app
                    .push_log("search: type query, Enter confirm, Esc cancel", true);
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
    use crate::flux::stores::test_support::{make_envelope, mock_app};
    use pretty_assertions::assert_eq;

    fn reduce(store: &mut OverlayStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_start_commit_input_emits_guarded_effect() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartCommitInput),
        );
        assert!(!output.commands.is_empty());
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
        assert_eq!(app.input_mode, Some(InputMode::CommandPalette));
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
        assert_eq!(app.input_mode, Some(InputMode::CreateBranch));
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
        assert_eq!(app.input_mode, Some(InputMode::Search));
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
        assert_eq!(app.input_mode, None);
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::app::{InputMode, SidePanel};
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};
    use crate::git::FileStatus;
    use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use pretty_assertions::assert_eq;

    fn reduce(store: &mut OverlayStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_commit_all_confirm_true_stages_all_and_commits() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitAllConfirm);
        // Set up unstaged files to stage
        app.status.unstaged = vec![crate::git::FileEntry {
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
        app.input_mode = Some(InputMode::CommitAllConfirm);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::CommitAllConfirm(false)),
        );
        assert!(app.input_mode.is_none());
    }

    #[test]
    fn test_start_stash_input_with_selected_file() {
        let mut store = OverlayStore::new();
        let mut app = mock_app();
        app.active_panel = SidePanel::Files;
        app.files.tree_nodes = vec![FileTreeNode {
            path: "foo.txt".into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }];
        app.files.panel.list_state.select(Some(0));
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StartStashInput),
        );
        assert_eq!(app.input_mode, Some(InputMode::StashEditor));
    }
}
