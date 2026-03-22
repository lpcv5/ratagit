use crate::app::{Command, CommitFieldFocus, InputMode};
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct InputStore;

impl InputStore {
    pub fn new() -> Self {
        Self
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
            DomainAction::InputEsc => {
                if mode == InputMode::Search {
                    ctx.app.clear_search();
                }
                ctx.app.cancel_input();
                ReduceOutput::none().with_invalidation(UiInvalidation::log_and_overlay())
            }
            DomainAction::InputTab => match mode {
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
                | InputMode::BranchSwitchConfirm => ReduceOutput::none(),
            },
            DomainAction::InputEnter => match mode {
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
                            ReduceOutput::none()
                                .with_invalidation(UiInvalidation::log_and_overlay())
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
            },
            DomainAction::InputBackspace => match mode {
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
                InputMode::CreateBranch => {
                    ctx.app.input_buffer.pop();
                    ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
                }
                InputMode::CommandPalette => {
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
            },
            DomainAction::InputChar(c) => match mode {
                InputMode::CommitEditor => {
                    match ctx.app.commit_focus {
                        CommitFieldFocus::Message => ctx.app.commit_message_buffer.push(*c),
                        CommitFieldFocus::Description => ctx.app.commit_description_buffer.push(*c),
                    }
                    ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
                }
                InputMode::CreateBranch => {
                    ctx.app.input_buffer.push(*c);
                    ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
                }
                InputMode::CommandPalette => {
                    ctx.app.input_buffer.push(*c);
                    ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
                }
                InputMode::StashEditor => {
                    ctx.app.stash_message_buffer.push(*c);
                    ReduceOutput::none().with_invalidation(UiInvalidation::overlay())
                }
                InputMode::Search => {
                    ctx.app.input_buffer.push(*c);
                    ReduceOutput::from_command(Command::Sync(DomainAction::SearchSetQuery(
                        ctx.app.input_buffer.clone(),
                    )))
                }
                InputMode::BranchSwitchConfirm => ReduceOutput::none(),
            },
            _ => ReduceOutput::none(),
        }
    }
}
