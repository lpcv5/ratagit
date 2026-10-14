//! Diff content, hunk interaction and loading decisions.
//!
//! This feature owns its state and exposes immutable views. It has no AppState,
//! dispatcher, terminal or task-runtime dependency; the application coordinator
//! translates its output into effects and cross-feature refreshes.

mod cache;
#[cfg(test)]
mod loader;
mod selectors;
mod state;
mod target;
#[cfg(test)]
mod tests;
mod update;

pub use cache::DiffCacheKey;
pub use selectors::{DiffView, HunkView};
pub(super) use state::DiffLoad;
pub use state::{DiffInteraction, DiffSide, DiffState};
pub use target::{DiffRefreshSource, DiffTarget};
pub use update::{DiffAction, DiffReload, DiffUpdate, HunkWrite};
