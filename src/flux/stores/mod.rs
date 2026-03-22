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

#[derive(Default)]
pub struct ReduceOutput {
    pub commands: Vec<Command>,
}

impl ReduceOutput {
    pub fn none() -> Self {
        Self::default()
    }

    pub fn from_command(command: Command) -> Self {
        Self {
            commands: vec![command],
        }
    }
}

pub trait Store {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput;
}
