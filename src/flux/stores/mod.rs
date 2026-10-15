use crate::flux::action::RuntimeEvent;
mod branch_store;
#[cfg(test)]
mod capability_tests;
mod runtime_store;
pub use runtime_store::RuntimeStore;
mod commit_store;
mod diff_store;
mod file_tools_store;
mod files_output;
mod files_store;
pub use file_tools_store::FileToolsStore;
mod help_store;
mod input_store;
mod navigation_store;
mod ops_store;
mod overlay_store;
mod quit_store;
mod revision_store;
mod search_store;
mod selection_store;
mod stash_store;
pub mod state_access;

use crate::app::Command;
use crate::flux::action::ActionEnvelope;
use crate::flux::command_output::CommandOutput;
pub use state_access::DirtyHint;
pub use state_access::StateAccess;
use state_access::{InvalidationAccess, LogAccess};

pub use branch_store::BranchStore;
pub use commit_store::CommitStore;
pub use diff_store::DiffStore;
pub use files_store::FilesStore;
pub use help_store::HelpStore;
pub use input_store::InputStore;
pub use navigation_store::NavigationStore;
pub use ops_store::OpsStore;
pub use overlay_store::OverlayStore;
pub use quit_store::QuitStore;
pub use revision_store::RevisionStore;
pub use search_store::SearchStore;
pub use selection_store::SelectionStore;
pub use stash_store::StashStore;

/// Reducers declare their required capabilities through `Store<S>` bounds.
/// Only the dispatcher erases the concrete state to `dyn StateAccess`.
pub struct ReduceCtx<'a, S: ?Sized> {
    pub state: &'a mut S,
    pub commands: &'a mut CommandOutput,
}

impl<S: ?Sized> ReduceCtx<'_, S> {
    /// Emit to the dispatcher-owned output; return invalidation separately.
    pub fn command(&mut self, command: Command) -> ReduceOutput {
        self.commands.command(command);
        ReduceOutput::none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiInvalidation(u8);

impl UiInvalidation {
    const MAIN_CONTENT: u8 = DirtyHint::MAIN_CONTENT;
    const DIFF: u8 = DirtyHint::DIFF;
    const COMMAND_LOG: u8 = DirtyHint::COMMAND_LOG;
    const SHORTCUT_BAR: u8 = DirtyHint::SHORTCUT_BAR;
    const OVERLAY: u8 = DirtyHint::OVERLAY;

    pub const fn none() -> Self {
        Self(0)
    }

    pub const fn all() -> Self {
        Self(
            Self::MAIN_CONTENT
                | Self::DIFF
                | Self::COMMAND_LOG
                | Self::SHORTCUT_BAR
                | Self::OVERLAY,
        )
    }

    pub const fn overlay() -> Self {
        Self(Self::OVERLAY)
    }

    pub const fn diff() -> Self {
        Self(Self::DIFF)
    }

    pub const fn log_and_overlay() -> Self {
        Self(Self::COMMAND_LOG | Self::OVERLAY)
    }

    pub fn merge(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn apply(self, state: &mut (impl InvalidationAccess + ?Sized)) {
        if self.0 == 0 {
            return;
        }
        if self.0 == Self::all().0 {
            state.mark_all_dirty();
            return;
        }
        state.mark_dirty(DirtyHint(self.0));
    }
}

#[derive(Default)]
pub struct ReduceOutput {
    pub invalidation: UiInvalidation,
}

impl ReduceOutput {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn with_invalidation(mut self, invalidation: UiInvalidation) -> Self {
        self.invalidation.merge(invalidation);
        self
    }
}

pub trait Store<S: ?Sized> {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_, S>) -> ReduceOutput;
}

/// Helper command to trigger a background task processing tick.
pub(super) fn tick_background_loads() -> Command {
    Command::runtime(RuntimeEvent::ProcessBackgroundLoads)
}

/// Log a completed operation's result and return the appropriate ReduceOutput.
/// On success: logs success_msg, returns full-UI invalidation.
/// On failure: logs the failure message, returns no-op output.
pub(super) fn log_result<S: LogAccess + ?Sized>(
    ctx: &mut ReduceCtx<'_, S>,
    result: &Result<(), crate::git::GitError>,
    success_msg: impl Into<String>,
    fail_msg: impl FnOnce(&str) -> String,
) -> ReduceOutput {
    match result {
        Ok(()) => {
            ctx.state.push_log(success_msg.into(), true);
            ReduceOutput::none().with_invalidation(UiInvalidation::all())
        }
        Err(e) => {
            ctx.state
                .push_log(fail_msg(&crate::app::git_error::message(e)), false);
            ReduceOutput::none()
        }
    }
}

/// Shared test utilities for all store unit tests.
#[cfg(test)]
pub mod test_support {
    use crate::app::App;
    use crate::flux::action::{Action, ActionEnvelope};
    use crate::git::{
        BranchInfo, CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, GitError,
        GitStatus, StashInfo,
    };
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{self, Receiver};

    pub struct MockRepo;

    impl crate::git::WorktreeRepository for MockRepo {
        fn status(&self) -> Result<GitStatus, GitError> {
            Ok(GitStatus::default())
        }

        fn stage(&self, _: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn stage_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
            Ok(())
        }

        fn unstage(&self, _: &Path) -> Result<(), GitError> {
            Ok(())
        }

        fn unstage_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
            Ok(())
        }

        fn discard_paths(&self, _: &[PathBuf]) -> Result<(), GitError> {
            Ok(())
        }
    }

    impl crate::git::PatchRepository for MockRepo {
        fn diff_unstaged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }

        fn diff_staged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }

        fn diff_untracked(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }

        fn commit_diff_scoped(&self, _: &str, _: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![DiffLine {
                kind: DiffLineKind::Header,
                content: "diff".to_string(),
            }])
        }
    }

    impl crate::git::HistoryRepository for MockRepo {
        fn commits(&self, _: usize) -> Result<Vec<CommitInfo>, GitError> {
            Ok(vec![CommitInfo {
                oid: "abc1234567890".to_string(),
                message: "test commit".to_string(),
                author: "tester".to_string(),
                graph: vec![crate::git::GraphCell {
                    text: "●".to_string(),
                    lane: 0,
                    pipe_oid: None,
                    pipe_oids: vec![],
                }],
                time: "2026-03-20 00:00".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::DefaultBranch,
                parent_oids: vec![],
            }])
        }

        fn commit_files(&self, _: &str) -> Result<Vec<FileEntry>, GitError> {
            Ok(vec![])
        }
    }

    impl crate::git::ReferenceRepository for MockRepo {
        fn branches(&self) -> Result<Vec<BranchInfo>, GitError> {
            Ok(vec![])
        }

        fn create_branch(&self, _: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn checkout_branch(&self, _: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn delete_branch(&self, _: &str) -> Result<(), GitError> {
            Ok(())
        }

        fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
            let (tx, rx) = mpsc::channel();
            drop(tx);
            Ok(rx)
        }
    }

    impl crate::git::StashRepository for MockRepo {
        fn stashes(&self) -> Result<Vec<StashInfo>, GitError> {
            Ok(vec![])
        }

        fn stash_files(&self, _: usize) -> Result<Vec<FileEntry>, GitError> {
            Ok(vec![])
        }

        fn stash_diff(&self, _: usize, _: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }

        fn stash_push_paths(&self, _: &[PathBuf], _: &str) -> Result<usize, GitError> {
            Ok(0)
        }

        fn stash_apply(&self, _: usize) -> Result<(), GitError> {
            Ok(())
        }

        fn stash_pop(&self, _: usize) -> Result<(), GitError> {
            Ok(())
        }

        fn stash_drop(&self, _: usize) -> Result<(), GitError> {
            Ok(())
        }
    }

    impl crate::git::CommitRepository for MockRepo {
        fn commit(&self, _: &str) -> Result<String, GitError> {
            Ok("oid".to_string())
        }
    }
    impl crate::git::OperationRepository for MockRepo {}

    pub fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("mock app")
    }

    pub fn make_envelope(action: Action) -> ActionEnvelope {
        ActionEnvelope {
            sequence: 1,
            action,
        }
    }

    pub struct TestReduceOutput {
        pub commands: Vec<crate::app::Command>,
        pub invalidation: super::UiInvalidation,
    }

    pub fn reduce_state<S: ?Sized>(
        store: &mut impl super::Store<S>,
        state: &mut S,
        action: Action,
    ) -> TestReduceOutput {
        let mut commands = crate::flux::command_output::CommandOutput::default();
        let output = store.reduce(
            &make_envelope(action),
            &mut super::ReduceCtx {
                state,
                commands: &mut commands,
            },
        );
        TestReduceOutput {
            commands: commands.drain(),
            invalidation: output.invalidation,
        }
    }

    pub fn reduce_action<S: super::Store<crate::app::AppState>>(
        store: &mut S,
        app: &mut App,
        action: Action,
    ) -> TestReduceOutput {
        reduce_state(store, &mut app.state, action)
    }
}
