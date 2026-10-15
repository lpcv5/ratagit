//! Diff content, hunk interaction and loading decisions.
//!
//! This feature owns its state and exposes immutable views. It has no AppState,
//! dispatcher, terminal or task-runtime dependency; the application coordinator
//! translates its output into effects and cross-feature refreshes.

mod cache;
#[cfg(test)]
mod loader;
mod memory;
mod selectors;
mod state;
mod target;
#[cfg(test)]
mod tests;
mod update;

pub use cache::{DiffCacheIdentity, DiffCacheKey, DiffCacheStats, FileDiffSource};
pub use selectors::{DiffView, HunkView};
pub(super) use state::DiffLoad;
pub use state::{DiffInteraction, DiffSide, DiffState};
pub(crate) use target::BRANCH_DIFF_LIMIT;
pub use target::{DiffRefreshSource, DiffTarget};
pub use update::{DiffAction, DiffReload, DiffUpdate, HunkWrite};
