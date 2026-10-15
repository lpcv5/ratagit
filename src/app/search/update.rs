use super::{SearchScopeKey, SearchState};

/// Explicit view identity and cursor input; this feature never touches panel state.
pub struct SearchContext<'a> {
    pub scope: &'a SearchScopeKey,
    pub selected: Option<usize>,
}

pub enum SearchAction<'a> {
    CaptureScope,
    SetQuery { query: String, items: &'a [String] },
    Restore { items: &'a [String] },
    Clear,
    SelectInitial,
    Next,
    Previous,
}

#[derive(Default, Debug, PartialEq, Eq)]
pub struct SearchUpdate {
    pub match_count: usize,
    pub navigate_to: Option<usize>,
}

impl SearchState {
    pub fn update(&mut self, action: SearchAction<'_>, context: SearchContext<'_>) -> SearchUpdate {
        let mut navigate_to = None;
        match action {
            SearchAction::CaptureScope => self.scope = context.scope.clone(),
            SearchAction::SetQuery { query, items } => {
                self.query = query;
                self.scope = context.scope.clone();
                if self.query.is_empty() {
                    self.queries.remove(&self.scope);
                } else {
                    self.queries.insert(self.scope.clone(), self.query.clone());
                }
                self.matches = match_indices(&self.query, items);
            }
            SearchAction::Restore { items } => {
                self.scope = context.scope.clone();
                self.query = self.queries.get(&self.scope).cloned().unwrap_or_default();
                self.matches = match_indices(&self.query, items);
            }
            SearchAction::Clear => {
                self.scope = context.scope.clone();
                self.query.clear();
                self.matches.clear();
                self.queries.remove(&self.scope);
            }
            action
            @ (SearchAction::SelectInitial | SearchAction::Next | SearchAction::Previous) => {
                if self.view().has_matches(context.scope) {
                    let selected = context.selected.unwrap_or(0);
                    navigate_to = match action {
                        SearchAction::SelectInitial => self
                            .matches
                            .iter()
                            .copied()
                            .find(|index| *index >= selected)
                            .or_else(|| self.matches.first().copied()),
                        SearchAction::Next => self
                            .matches
                            .iter()
                            .copied()
                            .find(|index| *index > selected)
                            .or_else(|| self.matches.first().copied()),
                        SearchAction::Previous => self
                            .matches
                            .iter()
                            .rev()
                            .copied()
                            .find(|index| *index < selected)
                            .or_else(|| self.matches.last().copied()),
                        _ => unreachable!("only navigation actions enter this branch"),
                    };
                }
            }
        }
        SearchUpdate {
            match_count: self.matches.len(),
            navigate_to,
        }
    }
}

fn match_indices(query: &str, items: &[String]) -> Vec<usize> {
    if query.is_empty() {
        return Vec::new();
    }
    let query = query.to_lowercase();
    items
        .iter()
        .enumerate()
        .filter_map(|(index, text)| text.to_lowercase().contains(&query).then_some(index))
        .collect()
}
