use crate::app::{Command, CommitFieldFocus, InputMode};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct InputStore;

impl InputStore {
    pub fn new() -> Self {
        Self
    }

    fn handle_esc(ctx: &mut ReduceCtx<'_>, mode: InputMode) -> ReduceOutput {
        if mode == InputMode::Search {
            ctx.app.clear_search();
        }
        ctx.app.cancel_input();
        ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay())
    }

    fn handle_tab(ctx: &mut ReduceCtx<'_>, mode: InputMode) -> ReduceOutput {
        match mode {
            InputMode::CommitEditor => {
                ctx.app.commit_focus = match ctx.app.commit_focus {
                    CommitFieldFocus::Message => CommitFieldFocus::Description,
                    CommitFieldFocus::Description => CommitFieldFocus::Message,
                };
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::CreateBranch
            | InputMode::StashEditor
            | InputMode::CommandPalette
            | InputMode::Search
            | InputMode::BranchSwitchConfirm
            | InputMode::CommitAllConfirm => ReduceOutput::none(),
        }
    }

    fn handle_enter(ctx: &mut ReduceCtx<'_>, mode: InputMode) -> ReduceOutput {
        match mode {
            InputMode::CommitEditor => match ctx.app.commit_focus {
                CommitFieldFocus::Message => {
                    let title = ctx.app.commit_message_buffer.trim().to_string();
                    if title.is_empty() {
                        ctx.app.push_log("Empty commit message ignored", false);
                        return ReduceOutput::none();
                    }
                    let description = ctx.app.commit_description_buffer.trim_end();
                    let value = if description.is_empty() {
                        title
                    } else {
                        format!("{}\n\n{}", title, description)
                    };
                    ctx.app.input_mode = None;
                    ctx.app.commit_message_buffer.clear();
                    ctx.app.commit_description_buffer.clear();
                    ctx.app.commit_focus = CommitFieldFocus::Message;
                    ReduceOutput::from_command(Command::Sync(DomainAction::Commit(value)))
                }
                CommitFieldFocus::Description => {
                    ctx.app.commit_description_buffer.push('\n');
                    ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
                }
            },
            InputMode::CreateBranch => {
                let value = ctx.app.input_buffer.trim().to_string();
                ctx.app.input_mode = None;
                ctx.app.input_buffer.clear();
                if value.is_empty() {
                    ctx.app.push_log("Empty input ignored", false);
                    return ReduceOutput::none()
                        .with_invalidation(UiInvalidation::log_and_overlay());
                }
                ReduceOutput::from_command(Command::Sync(DomainAction::CreateBranch(value)))
                    .with_invalidation(UiInvalidation::overlay())
            }
            InputMode::CommandPalette => {
                let value = ctx.app.input_buffer.trim().to_string();
                ctx.app.input_mode = None;
                ctx.app.input_buffer.clear();
                if value.is_empty() {
                    ctx.app.push_log("command palette: empty command", false);
                    return ReduceOutput::none()
                        .with_invalidation(UiInvalidation::log_and_overlay());
                }
                match ctx.app.resolve_command_palette_command(&value) {
                    Some(action) => ReduceOutput::from_command(Command::Sync(action))
                        .with_invalidation(UiInvalidation::overlay()),
                    None => {
                        ctx.app
                            .push_log(format!("unknown command: {}", value), false);
                        ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay())
                    }
                }
            }
            InputMode::StashEditor => {
                let value = ctx.app.stash_message_buffer.trim().to_string();
                let paths = ctx.app.stash_targets.clone();
                ctx.app.input_mode = None;
                ctx.app.stash_message_buffer.clear();
                ctx.app.stash_targets.clear();
                if value.is_empty() {
                    ctx.app.push_log("Empty stash title ignored", false);
                    return ReduceOutput::none()
                        .with_invalidation(UiInvalidation::log_and_overlay());
                }
                if paths.is_empty() {
                    ctx.app.push_log("stash blocked: no selected items", false);
                    return ReduceOutput::none()
                        .with_invalidation(UiInvalidation::log_and_overlay());
                }
                ReduceOutput::from_command(Command::Sync(DomainAction::StashPush {
                    message: value,
                    paths,
                }))
                .with_invalidation(UiInvalidation::overlay())
            }
            InputMode::Search => {
                ctx.app.confirm_search_input();
                ReduceOutput::from_command(Command::Sync(DomainAction::SearchConfirm))
                    .with_invalidation(UiInvalidation::overlay())
            }
            InputMode::BranchSwitchConfirm => ReduceOutput::none(),
            InputMode::CommitAllConfirm => ReduceOutput::none(),
        }
    }

    fn handle_backspace(ctx: &mut ReduceCtx<'_>, mode: InputMode) -> ReduceOutput {
        match mode {
            InputMode::CommitEditor => {
                match ctx.app.commit_focus {
                    CommitFieldFocus::Message => {
                        ctx.app.commit_message_buffer.pop();
                    }
                    CommitFieldFocus::Description => {
                        ctx.app.commit_description_buffer.pop();
                    }
                }
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::CreateBranch | InputMode::CommandPalette => {
                ctx.app.input_buffer.pop();
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::StashEditor => {
                ctx.app.stash_message_buffer.pop();
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::Search => {
                ctx.app.input_buffer.pop();
                ReduceOutput::from_command(Command::Sync(DomainAction::SearchSetQuery(
                    ctx.app.input_buffer.clone(),
                )))
            }
            InputMode::BranchSwitchConfirm => ReduceOutput::none(),
            InputMode::CommitAllConfirm => ReduceOutput::none(),
        }
    }

    fn handle_char(ctx: &mut ReduceCtx<'_>, mode: InputMode, c: char) -> ReduceOutput {
        match mode {
            InputMode::CommitEditor => {
                match ctx.app.commit_focus {
                    CommitFieldFocus::Message => ctx.app.commit_message_buffer.push(c),
                    CommitFieldFocus::Description => ctx.app.commit_description_buffer.push(c),
                }
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::CreateBranch | InputMode::CommandPalette => {
                ctx.app.input_buffer.push(c);
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::StashEditor => {
                ctx.app.stash_message_buffer.push(c);
                ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
            }
            InputMode::Search => {
                ctx.app.input_buffer.push(c);
                ReduceOutput::from_command(Command::Sync(DomainAction::SearchSetQuery(
                    ctx.app.input_buffer.clone(),
                )))
            }
            InputMode::BranchSwitchConfirm => ReduceOutput::none(),
            InputMode::CommitAllConfirm => ReduceOutput::none(),
        }
    }
}

impl Store for InputStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };

        match domain {
            DomainAction::InputEsc
            | DomainAction::InputTab
            | DomainAction::InputEnter
            | DomainAction::InputBackspace
            | DomainAction::InputChar(_) => {}
            _ => return ReduceOutput::none(),
        }

        let mode = match ctx.app.input_mode {
            Some(mode) => mode,
            None => return ReduceOutput::none(),
        };

        match domain {
            DomainAction::InputEsc => Self::handle_esc(ctx, mode),
            DomainAction::InputTab => Self::handle_tab(ctx, mode),
            DomainAction::InputEnter => Self::handle_enter(ctx, mode),
            DomainAction::InputBackspace => Self::handle_backspace(ctx, mode),
            DomainAction::InputChar(c) => Self::handle_char(ctx, mode, *c),
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
    fn test_non_input_action_ignored() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
    }

    #[test]
    fn test_no_input_mode_ignored() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = None;
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputEnter),
        );
        assert!(output.commands.is_empty());
    }

    #[test]
    fn test_input_char_in_create_branch_mode_updates_buffer() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CreateBranch);
        app.input_buffer.clear();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputChar('f')),
        );
        assert_eq!(app.input_buffer, "f");
    }

    #[test]
    fn test_input_backspace_in_create_branch_removes_char() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CreateBranch);
        app.input_buffer = "feature".to_string();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputBackspace),
        );
        assert_eq!(app.input_buffer, "featur");
    }

    #[test]
    fn test_input_esc_in_search_mode_clears_and_exits() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::Search);
        app.search_query = "foo".to_string();
        reduce(&mut store, &mut app, Action::Domain(DomainAction::InputEsc));
        assert!(app.input_mode.is_none());
        assert!(app.search_query.is_empty());
    }

    #[test]
    fn test_input_esc_in_commit_mode_cancels() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.commit_message_buffer = "wip".to_string();
        reduce(&mut store, &mut app, Action::Domain(DomainAction::InputEsc));
        assert!(app.input_mode.is_none());
        assert!(app.commit_message_buffer.is_empty());
    }

    #[test]
    fn test_input_enter_create_branch_emits_create_branch_command() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CreateBranch);
        app.input_buffer = "feature".to_string();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputEnter),
        );
        assert!(!output.commands.is_empty());
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::app::{CommitFieldFocus, InputMode};
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use pretty_assertions::assert_eq;

    #[test]
    fn test_input_char_in_commit_editor_message_focus() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.commit_focus = CommitFieldFocus::Message;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputChar('x')),
        );
        assert_eq!(app.commit_message_buffer, "x");
    }

    #[test]
    fn test_input_char_in_commit_editor_description_focus() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.commit_focus = CommitFieldFocus::Description;
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputChar('y')),
        );
        assert_eq!(app.commit_description_buffer, "y");
    }

    #[test]
    fn test_input_tab_in_commit_editor_switches_focus() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.commit_focus = CommitFieldFocus::Message;
        reduce(&mut store, &mut app, Action::Domain(DomainAction::InputTab));
        assert_eq!(app.commit_focus, CommitFieldFocus::Description);
    }

    #[test]
    fn test_input_enter_in_commit_editor_message_with_content_emits_commit_command() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.commit_focus = CommitFieldFocus::Message;
        app.commit_message_buffer = "fix: test".to_string();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputEnter),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_input_backspace_in_commit_editor_message() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommitEditor);
        app.commit_focus = CommitFieldFocus::Message;
        app.commit_message_buffer = "fix".to_string();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputBackspace),
        );
        assert_eq!(app.commit_message_buffer, "fi");
    }

    #[test]
    fn test_input_char_in_stash_editor() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::StashEditor);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputChar('w')),
        );
        assert_eq!(app.stash_message_buffer, "w");
    }

    #[test]
    fn test_input_enter_in_stash_editor_emits_stash_push_command() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::StashEditor);
        app.stash_message_buffer = "wip".to_string();
        app.stash_targets = vec!["foo.txt".into()];
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputEnter),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_input_char_in_command_palette() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommandPalette);
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputChar('q')),
        );
        assert_eq!(app.input_buffer, "q");
    }

    #[test]
    fn test_input_enter_in_command_palette_with_known_command() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::CommandPalette);
        app.input_buffer = "quit".to_string();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::InputEnter),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_input_esc_in_branch_switch_confirm_cancels() {
        let mut store = InputStore::new();
        let mut app = mock_app();
        app.input_mode = Some(InputMode::BranchSwitchConfirm);
        reduce(&mut store, &mut app, Action::Domain(DomainAction::InputEsc));
        assert!(app.input_mode.is_none());
    }
}
