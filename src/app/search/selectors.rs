use super::SearchScopeKey;
use std::collections::HashMap;

/// Borrowed search projection. It exposes no mutable application or input state.
#[derive(Clone, Copy)]
pub struct SearchView<'a> {
    pub query: &'a str,
    pub matches: &'a [usize],
    pub scope: &'a SearchScopeKey,
    pub(super) queries: &'a HashMap<SearchScopeKey, String>,
}

impl<'a> SearchView<'a> {
    pub fn saved_query(&self, scope: &SearchScopeKey) -> Option<&'a str> {
        self.queries.get(scope).map(String::as_str)
    }

    pub fn query_for_scope(&self, scope: &SearchScopeKey) -> Option<&'a str> {
        (!self.query.is_empty() && self.scope == scope).then_some(self.query)
    }

    pub fn has_query(&self, scope: &SearchScopeKey) -> bool {
        self.query_for_scope(scope).is_some()
    }

    pub fn has_matches(&self, scope: &SearchScopeKey) -> bool {
        self.scope == scope && !self.matches.is_empty()
    }

    pub fn has_search(&self, scope: &SearchScopeKey) -> bool {
        self.has_query(scope) && self.has_matches(scope)
    }

    pub fn summary(&self, scope: &SearchScopeKey, selected: Option<usize>) -> Option<String> {
        let query = self.query_for_scope(scope)?;
        if self.matches.is_empty() {
            return Some(format!("/{query} 0/0"));
        }
        let current = selected
            .and_then(|index| self.matches.iter().position(|item| *item == index))
            .map(|position| position + 1)
            .unwrap_or(1);
        Some(format!(
            "/{query} {current}/{total}",
            total = self.matches.len()
        ))
    }
}
