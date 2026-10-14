//! File-tree ownership, selection and worktree-operation decisions.
//! Production feature code uses only data models and explicit context inputs.

mod selectors;
mod state;
#[cfg(test)]
mod tests;
mod update;

pub use selectors::{FileSelection, StageTarget};
pub use state::{FilesPanelState, FilesState};
pub use update::{FileWrite, FilesAction, FilesUpdate};
