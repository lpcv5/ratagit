//! Search scope/query ownership, pure matching and navigation decisions.
//! Coordinators provide searchable text and apply the returned navigation intent.
mod selectors;
mod state;
#[cfg(test)]
mod tests;
mod update;

pub use selectors::SearchView;
pub use state::{SearchScopeKey, SearchState};
pub use update::{SearchAction, SearchContext, SearchUpdate};
