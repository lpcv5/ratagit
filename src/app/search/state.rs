use crate::app::ViewContext;
use std::collections::HashMap;

pub type SearchScopeKey = ViewContext;

/// Search domain data is private to this feature; editor buffers remain in InputState.
#[derive(Default)]
pub struct SearchState {
    pub(super) query: String,
    pub(super) matches: Vec<usize>,
    pub(super) scope: SearchScopeKey,
    pub(super) queries: HashMap<SearchScopeKey, String>,
}

impl SearchState {
    pub fn view(&self) -> super::SearchView<'_> {
        super::SearchView {
            query: &self.query,
            matches: &self.matches,
            scope: &self.scope,
            queries: &self.queries,
        }
    }

    #[cfg(test)]
    pub(crate) fn set_test_current(&mut self, query: String, matches: Vec<usize>) {
        self.query = query;
        self.matches = matches;
    }
}
