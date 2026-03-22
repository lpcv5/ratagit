mod branch_store;
mod commit_store;
mod diff_store;
mod files_store;
mod input_store;
mod navigation_store;
mod ops_store;
mod overlay_store;
mod quit_store;
mod revision_store;
mod search_store;
mod selection_store;
mod stash_store;

use crate::app::{App, Command};
use crate::flux::action::ActionEnvelope;

pub use branch_store::BranchStore;
pub use commit_store::CommitStore;
pub use diff_store::DiffStore;
pub use files_store::FilesStore;
pub use input_store::InputStore;
pub use navigation_store::NavigationStore;
pub use ops_store::OpsStore;
pub use overlay_store::OverlayStore;
pub use quit_store::QuitStore;
pub use revision_store::RevisionStore;
pub use search_store::SearchStore;
pub use selection_store::SelectionStore;
pub use stash_store::StashStore;

pub struct ReduceCtx<'a> {
    pub app: &'a mut App,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiInvalidation(u8);

impl UiInvalidation {
    const MAIN_CONTENT: u8 = 0b0000_0001;
    const DIFF: u8 = 0b0000_0010;
    const COMMAND_LOG: u8 = 0b0000_0100;
    const SHORTCUT_BAR: u8 = 0b0000_1000;
    const OVERLAY: u8 = 0b0001_0000;

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

    pub fn apply(self, app: &mut App) {
        if self.0 == 0 {
            return;
        }
        if self.0 == Self::all().0 {
            app.dirty.mark_all();
            return;
        }
        if (self.0 & (Self::MAIN_CONTENT | Self::DIFF)) == (Self::MAIN_CONTENT | Self::DIFF) {
            app.dirty.mark_main_content();
        } else {
            if (self.0 & Self::MAIN_CONTENT) != 0 {
                app.dirty.left_panels = true;
            }
            if (self.0 & Self::DIFF) != 0 {
                app.dirty.mark_diff();
            }
        }
        if (self.0 & Self::COMMAND_LOG) != 0 {
            app.dirty.mark_command_log();
        }
        if (self.0 & Self::SHORTCUT_BAR) != 0 {
            app.dirty.shortcut_bar = true;
        }
        if (self.0 & Self::OVERLAY) != 0 {
            app.dirty.mark_overlay();
        }
    }
}

#[derive(Default)]
pub struct ReduceOutput {
    pub commands: Vec<Command>,
    pub invalidation: UiInvalidation,
}

impl ReduceOutput {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn from_command(command: Command) -> Self {
        Self {
            commands: vec![command],
            invalidation: UiInvalidation::none(),
        }
    }

    pub fn with_invalidation(mut self, invalidation: UiInvalidation) -> Self {
        self.invalidation.merge(invalidation);
        self
    }
}

pub trait Store {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput;
}

/// Helper to flush pending refresh without logging success.
pub(super) fn flush_refresh() -> Command {
    Command::Effect(crate::flux::effects::EffectRequest::ProcessBackgroundLoads)
}

/// Shared test utilities for all store unit tests.
#[cfg(test)]
pub mod test_support {
    use crate::app::App;
    use crate::flux::action::{Action, ActionEnvelope};
    use crate::git::{
        BranchInfo, CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, GitError,
        GitRepository, GitStatus, StashInfo,
    };
    use std::path::{Path, PathBuf};
    use std::sync::mpsc::{self, Receiver};

    pub struct MockRepo;

    impl GitRepository for MockRepo {
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
        fn diff_unstaged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }
        fn diff_staged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }
        fn diff_untracked(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![])
        }
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
        fn commits(&self, _: usize) -> Result<Vec<CommitInfo>, GitError> {
            Ok(vec![CommitInfo {
                short_hash: "abc1234".to_string(),
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
        fn commit_diff_scoped(&self, _: &str, _: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            Ok(vec![DiffLine {
                kind: DiffLineKind::Header,
                content: "diff".to_string(),
            }])
        }
        fn commit(&self, _: &str) -> Result<String, GitError> {
            Ok("oid".to_string())
        }
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
        fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError> {
            let (tx, rx) = mpsc::channel();
            drop(tx);
            Ok(rx)
        }
    }

    pub fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("mock app")
    }

    pub fn make_envelope(action: Action) -> ActionEnvelope {
        ActionEnvelope {
            sequence: 1,
            action,
        }
    }
}
