use super::files_output::finish_files_update;
use super::state_access::{FilesAccess, LogAccess};
use crate::app::files::FilesAction;
use crate::flux::action::{Action, ActionEnvelope, DomainAction};
use crate::flux::stores::{ReduceCtx, ReduceOutput, Store};

pub struct FilesStore;

impl FilesStore {
    pub fn new() -> Self {
        Self
    }
}

impl<S: FilesAccess + LogAccess + ?Sized> Store<S> for FilesStore {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput {
        let Action::Domain(domain) = &action.action else {
            return ReduceOutput::none();
        };
        let action = match domain {
            DomainAction::StageFile(path) => FilesAction::StagePath(path),
            DomainAction::UnstageFile(path) => FilesAction::UnstagePath(path),
            DomainAction::DiscardPaths(paths) => FilesAction::DiscardPaths(paths),
            DomainAction::StageFileFinished { path, result } => {
                FilesAction::StageFinished { path, result }
            }
            DomainAction::UnstageFileFinished { path, result } => {
                FilesAction::UnstageFinished { path, result }
            }
            DomainAction::DiscardPathsFinished { paths, result } => {
                FilesAction::DiscardPathsFinished { paths, result }
            }
            _ => return ReduceOutput::none(),
        };
        let update = ctx.state.reduce_files(action);
        finish_files_update(ctx, update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use std::path::PathBuf;

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
    use crate::flux::stores::test_support::{mock_app, reduce_action as reduce};
    use std::path::PathBuf;

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
