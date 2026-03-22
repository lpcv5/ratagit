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
