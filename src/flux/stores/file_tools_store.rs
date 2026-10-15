//! Files panel tools emit owned effects and consume their completion events.
use super::state_access::LogAccess;
use super::{ReduceCtx, ReduceOutput, Store, UiInvalidation};
use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;

pub struct FileToolsStore;

impl<S: LogAccess + ?Sized> Store<S> for FileToolsStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        let Action::Domain(action) = &action.action else {
            return ReduceOutput::none();
        };
        let effect = match action {
            DomainAction::ToggleStageAll => Some(EffectRequest::ToggleStageAll),
            DomainAction::AmendHead => Some(EffectRequest::AmendHead),
            DomainAction::IgnoreFile(path) => Some(EffectRequest::IgnoreFile(path.clone())),
            DomainAction::EditFile(path) => Some(EffectRequest::PrepareEditor(path.clone())),
            DomainAction::CopyPath(path) => Some(EffectRequest::CopyPath(path.clone())),
            _ => None,
        };
        if let Some(effect) = effect {
            return ctx.command(Command::Effect(effect));
        }
        let (message, result) = match action {
            DomainAction::EditorPrepared {
                result: Ok(command),
            } => {
                return ctx.command(Command::RunEditor(command.clone()));
            }
            DomainAction::EditorPrepared { result: Err(error) } => {
                ("prepare editor".into(), Err(error))
            }
            DomainAction::ToggleStageAllFinished { result } => (
                if matches!(result, Ok(false)) {
                    "unstage all"
                } else {
                    "stage all"
                }
                .into(),
                result.as_ref().map(|_| ()),
            ),
            DomainAction::AmendHeadFinished { result } => {
                ("amend HEAD".into(), result.as_ref().map(|_| ()))
            }
            DomainAction::IgnoreFileFinished { path, result } => (
                format!("ignore {}", path.display()),
                result.as_ref().map(|_| ()),
            ),
            DomainAction::CopyPathFinished { path, result } => (
                format!("copy path {}", path.display()),
                result.as_ref().map(|_| ()),
            ),
            DomainAction::EditorFinished { result } => {
                ("edit file".into(), result.as_ref().map(|_| ()))
            }
            _ => return ReduceOutput::none(),
        };
        match result {
            Ok(()) => ctx.state.push_log(format!("{message}: done"), true),
            Err(error) => ctx.state.push_log(
                format!("{message}: {}", crate::app::git_error::message(error)),
                false,
            ),
        }
        ReduceOutput::none().with_invalidation(UiInvalidation::all())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::dispatcher::Dispatcher;
    use crate::flux::effects::{execute_with_desktop, EffectRequest};
    use crate::flux::stores::test_support::MockRepo;
    use crate::git::GitError;
    use std::sync::Mutex;

    #[derive(Default)]
    struct Clipboard {
        texts: Mutex<Vec<String>>,
        fail: bool,
    }

    impl crate::desktop::Desktop for Clipboard {
        fn copy_text(&self, text: &str) -> Result<(), GitError> {
            if self.fail {
                return Err(GitError::Worker("clipboard unavailable".into()));
            }
            self.texts.lock().unwrap().push(text.to_string());
            Ok(())
        }
    }

    #[test]
    fn test_copy_path_effect_and_completion_use_injected_clipboard_without_host_access() {
        for fail in [false, true] {
            let clipboard = Clipboard {
                fail,
                ..Default::default()
            };
            let path = std::path::PathBuf::from("目录/a file.txt");
            let events =
                execute_with_desktop(EffectRequest::CopyPath(path.clone()), &MockRepo, &clipboard);
            assert!(
                matches!(&events[0], Action::Domain(DomainAction::CopyPathFinished { path: copied, result })
                if copied == &path && result.is_err() == fail)
            );
            assert_eq!(clipboard.texts.lock().unwrap().len(), usize::from(!fail));
            if !fail {
                assert_eq!(clipboard.texts.lock().unwrap()[0], path.to_str().unwrap());
            }
            let mut app = crate::flux::stores::test_support::mock_app();
            let mut dispatcher = Dispatcher::with_default_stores();
            let envelope = dispatcher.next_envelope(events[0].clone());
            dispatcher.dispatch(&mut app.state, envelope);
            let log = app.command_log.last().unwrap();
            assert_eq!(log.success, !fail);
            assert!(log.command.contains("copy path"));
        }
    }

    #[test]
    fn test_editor_preparation_emits_terminal_command_and_failures_do_not_launch() {
        let mut app = crate::flux::stores::test_support::mock_app();
        let mut dispatcher = Dispatcher::with_default_stores();
        let command = crate::desktop::EditorCommand {
            program: "editor".into(),
            args: vec!["file".into()],
            cwd: ".".into(),
        };
        let envelope = dispatcher.next_envelope(Action::Domain(DomainAction::EditorPrepared {
            result: Ok(command.clone()),
        }));
        let output = dispatcher.dispatch(&mut app.state, envelope);
        assert!(
            matches!(output.commands.as_slice(), [Command::RunEditor(actual)] if actual == &command)
        );
        let envelope = dispatcher.next_envelope(Action::Domain(DomainAction::EditorPrepared {
            result: Err(GitError::InvalidState),
        }));
        let output = dispatcher.dispatch(&mut app.state, envelope);
        assert!(output.commands.is_empty());
        assert!(!app.command_log.last().unwrap().success);
    }

    #[test]
    fn test_mutation_completions_refresh_including_failure_after_partial_write() {
        for action in [
            DomainAction::ToggleStageAllFinished { result: Ok(true) },
            DomainAction::ToggleStageAllFinished { result: Ok(false) },
            DomainAction::ToggleStageAllFinished {
                result: Err(GitError::InvalidState),
            },
            DomainAction::IgnoreFileFinished {
                path: "file".into(),
                result: Ok(()),
            },
            DomainAction::EditorFinished {
                result: Err(GitError::InvalidState),
            },
            DomainAction::AmendHeadFinished {
                result: Ok("oid".into()),
            },
        ] {
            let full = matches!(action, DomainAction::AmendHeadFinished { .. });
            let mut app = crate::flux::stores::test_support::mock_app();
            let mut dispatcher = Dispatcher::with_default_stores();
            let envelope = dispatcher.next_envelope(Action::Domain(action));
            dispatcher.dispatch(&mut app.state, envelope);
            assert_eq!(
                app.pending_refresh_kind(),
                Some(if full {
                    crate::app::RefreshKind::Full
                } else {
                    crate::app::RefreshKind::StatusOnly
                })
            );
        }
    }
}
