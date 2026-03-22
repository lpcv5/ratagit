use crate::app::Command;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::effects::EffectRequest;
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store, UiInvalidation};

pub struct FilesStore;

impl FilesStore {
    pub fn new() -> Self {
        Self
    }
}

impl Store for FilesStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        match domain {
            DomainAction::StageFile(path) => {
                ReduceOutput::from_command(Command::Effect(EffectRequest::StageFile(path.clone())))
            }
            DomainAction::UnstageFile(path) => ReduceOutput::from_command(Command::Effect(
                EffectRequest::UnstageFile(path.clone()),
            )),
            DomainAction::DiscardPaths(paths) => {
                if paths.is_empty() {
                    ctx.app
                        .push_log("discard blocked: no discardable selected items", false);
                    return ReduceOutput::none();
                }
                ReduceOutput::from_command(Command::Effect(EffectRequest::DiscardPaths(
                    paths.clone(),
                )))
            }
            DomainAction::StageFileFinished { path, result } => {
                let display = path.display().to_string();
                match result {
                    Ok(()) => {
                        ctx.app.push_log(format!("staged {}", display), true);
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(err) => {
                        ctx.app
                            .push_log(format!("stage failed {}: {}", display, err), false);
                    }
                }
                ReduceOutput::none()
            }
            DomainAction::UnstageFileFinished { path, result } => {
                let display = path.display().to_string();
                match result {
                    Ok(()) => {
                        ctx.app.push_log(format!("unstaged {}", display), true);
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(err) => {
                        ctx.app
                            .push_log(format!("unstage failed {}: {}", display, err), false);
                    }
                }
                ReduceOutput::none()
            }
            DomainAction::DiscardPathsFinished { paths, result } => {
                match result {
                    Ok(()) => {
                        if paths.len() == 1 {
                            ctx.app
                                .push_log(format!("discarded {}", paths[0].display()), true);
                        } else {
                            ctx.app
                                .push_log(format!("discarded {} path(s)", paths.len()), true);
                        }
                        return ReduceOutput::none().with_invalidation(UiInvalidation::all());
                    }
                    Err(err) => {
                        ctx.app.push_log(format!("discard failed: {}", err), false);
                    }
                }
                ReduceOutput::none()
            }
            _ => ReduceOutput::none(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};
    use std::path::PathBuf;

    fn reduce(store: &mut FilesStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_stage_file_emits_effect() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StageFile(PathBuf::from("foo.txt"))),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_unstage_file_emits_effect() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::UnstageFile(PathBuf::from("foo.txt"))),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_stage_file_finished_ok_logs_success() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let initial_logs = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StageFileFinished {
                path: PathBuf::from("foo.txt"),
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > initial_logs);
    }

    #[test]
    fn test_unstage_file_finished_ok_logs_success() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let initial_logs = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::UnstageFileFinished {
                path: PathBuf::from("foo.txt"),
                result: Ok(()),
            }),
        );
        assert!(app.command_log.len() > initial_logs);
    }

    #[test]
    fn test_discard_paths_finished_single_path_logs_name() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPathsFinished {
                paths: vec![PathBuf::from("bar.txt")],
                result: Ok(()),
            }),
        );
        let last = app.command_log.last().unwrap();
        assert!(last.command.contains("bar.txt"));
    }

    #[test]
    fn test_unknown_action_does_nothing() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let output = reduce(&mut store, &mut app, Action::Domain(DomainAction::Quit));
        assert!(output.commands.is_empty());
    }
}

#[cfg(test)]
mod more_tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{make_envelope, mock_app};
    use std::path::PathBuf;

    fn reduce(store: &mut FilesStore, app: &mut crate::app::App, action: Action) -> ReduceOutput {
        let env = make_envelope(action);
        let mut ctx = ReduceCtx { app };
        store.reduce(&env, &mut ctx)
    }

    #[test]
    fn test_discard_paths_emits_effect() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let output = reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPaths(vec![PathBuf::from("foo.txt")])),
        );
        assert!(!output.commands.is_empty());
    }

    #[test]
    fn test_stage_file_finished_err_logs_failure() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::StageFileFinished {
                path: PathBuf::from("foo.txt"),
                result: Err("stage failed".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_unstage_file_finished_err_logs_failure() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::UnstageFileFinished {
                path: PathBuf::from("foo.txt"),
                result: Err("unstage failed".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }

    #[test]
    fn test_discard_paths_finished_multiple_paths_logs_count() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPathsFinished {
                paths: vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")],
                result: Ok(()),
            }),
        );
        let last = app.command_log.last().unwrap();
        assert!(last.command.contains("2"));
    }

    #[test]
    fn test_discard_paths_finished_err_logs_failure() {
        let mut store = FilesStore::new();
        let mut app = mock_app();
        let logs_before = app.command_log.len();
        reduce(
            &mut store,
            &mut app,
            Action::Domain(DomainAction::DiscardPathsFinished {
                paths: vec![PathBuf::from("foo.txt")],
                result: Err("discard failed".to_string()),
            }),
        );
        assert!(app.command_log.len() > logs_before);
    }
}
