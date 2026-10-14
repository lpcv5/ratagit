#[cfg(test)]
use crate::app::App;
use crate::app::{AppState, SearchScopeKey, SidePanel, ViewContext};
use crate::file_tree::FileTreeNode;

impl AppState {
    pub fn start_search_input(&mut self) {
        self.input.mode = Some(crate::app::InputMode::Search);
        self.input.buffer = self.input.search_query.clone();
        self.capture_search_scope();
    }

    pub fn apply_search_query(&mut self, query: String) -> usize {
        self.input.search_query = query;
        self.capture_search_scope();
        let scope = self.current_search_scope_key();
        if self.input.search_query.is_empty() {
            self.input.search_matches.clear();
            self.input.search_queries.remove(&scope);
            return 0;
        }
        self.input
            .search_queries
            .insert(scope, self.input.search_query.clone());
        self.input.search_matches = self
            .searchable_items_for_scope()
            .into_iter()
            .enumerate()
            .filter_map(|(idx, text)| {
                if text
                    .to_lowercase()
                    .contains(&self.input.search_query.to_lowercase())
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        self.input.search_matches.len()
    }

    pub fn confirm_search_input(&mut self) {
        self.input.mode = None;
        self.input.buffer.clear();
    }

    pub fn clear_search(&mut self) {
        self.capture_search_scope();
        let scope = self.current_search_scope_key();
        self.input.search_query.clear();
        self.input.buffer.clear();
        self.input.search_matches.clear();
        self.input.search_queries.remove(&scope);
    }

    pub fn restore_search_for_active_scope(&mut self) {
        self.capture_search_scope();
        let scope = self.current_search_scope_key();
        let query = self
            .input
            .search_queries
            .get(&scope)
            .cloned()
            .unwrap_or_default();
        self.input.search_query = query;
        if self.input.search_query.is_empty() {
            self.input.search_matches.clear();
            return;
        }
        self.input.search_matches = self
            .searchable_items_for_scope()
            .into_iter()
            .enumerate()
            .filter_map(|(idx, text)| {
                if text
                    .to_lowercase()
                    .contains(&self.input.search_query.to_lowercase())
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
    }

    pub fn has_search_query_for_active_scope(&self) -> bool {
        if self.input.search_query.is_empty() {
            return false;
        }
        self.input.search_scope == self.current_search_scope_key()
    }

    pub fn has_search_for_active_scope(&self) -> bool {
        if self.input.search_query.is_empty() {
            return false;
        }
        self.search_scope_matches_active()
    }

    pub fn search_match_summary_for(
        &self,
        panel: SidePanel,
        commit_tree_mode: bool,
        stash_tree_mode: bool,
    ) -> Option<String> {
        if self.input.search_query.is_empty() {
            return None;
        }
        let scope = self.scope_for(panel, commit_tree_mode, stash_tree_mode);
        if scope != self.input.search_scope {
            return None;
        }
        if self.input.search_matches.is_empty() {
            return Some(format!("/{query} 0/0", query = self.input.search_query));
        }
        let selected = self.active_panel_state_selected();
        let current = selected
            .and_then(|idx| self.input.search_matches.iter().position(|m| *m == idx))
            .map(|pos| pos + 1)
            .unwrap_or(1);
        Some(format!(
            "/{query} {current}/{total}",
            query = self.input.search_query,
            total = self.input.search_matches.len()
        ))
    }

    pub fn search_query_for_scope(
        &self,
        panel: SidePanel,
        commit_tree_mode: bool,
        stash_tree_mode: bool,
    ) -> Option<&str> {
        if self.input.search_query.is_empty() {
            return None;
        }
        let scope = self.scope_for(panel, commit_tree_mode, stash_tree_mode);
        if scope != self.input.search_scope {
            return None;
        }
        Some(self.input.search_query.as_str())
    }

    pub fn search_jump_next(&mut self) -> bool {
        self.search_jump(true)
    }

    pub fn search_jump_prev(&mut self) -> bool {
        self.search_jump(false)
    }

    pub fn search_select_initial_match(&mut self) -> bool {
        if !self.search_scope_matches_active() {
            return false;
        }
        let selected = self.active_panel_state_selected().unwrap_or(0);
        let next = self
            .input
            .search_matches
            .iter()
            .copied()
            .find(|idx| *idx >= selected)
            .or_else(|| self.input.search_matches.first().copied());
        let Some(target) = next else {
            return false;
        };
        self.select_active_panel_index(target);
        true
    }

    fn search_jump(&mut self, forward: bool) -> bool {
        if !self.search_scope_matches_active() {
            return false;
        }
        let selected = self.active_panel_state_selected().unwrap_or(0);
        let next = if forward {
            self.input
                .search_matches
                .iter()
                .copied()
                .find(|idx| *idx > selected)
                .or_else(|| self.input.search_matches.first().copied())
        } else {
            self.input
                .search_matches
                .iter()
                .rev()
                .copied()
                .find(|idx| *idx < selected)
                .or_else(|| self.input.search_matches.last().copied())
        };
        let Some(target) = next else {
            return false;
        };
        self.select_active_panel_index(target);
        true
    }

    fn capture_search_scope(&mut self) {
        self.input.search_scope = self.current_search_scope_key();
    }

    fn current_search_scope_key(&self) -> SearchScopeKey {
        self.view_context()
    }

    fn scope_for(&self, panel: SidePanel, commit_tree: bool, stash_tree: bool) -> ViewContext {
        match panel {
            SidePanel::Commits if commit_tree => ViewContext::CommitFiles {
                oid: self
                    .ui
                    .commits
                    .tree_mode
                    .selected_source
                    .clone()
                    .unwrap_or_default(),
            },
            SidePanel::Commits => ViewContext::Commits,
            SidePanel::Stash if stash_tree => ViewContext::StashFiles {
                index: self.ui.stash.tree_mode.selected_source,
            },
            SidePanel::Stash => ViewContext::Stashes,
            _ => {
                ViewContext::from_panels(panel, &self.ui.branches, &self.ui.commits, &self.ui.stash)
            }
        }
    }

    fn search_scope_matches_active(&self) -> bool {
        self.input.search_scope == self.current_search_scope_key()
            && !self.input.search_matches.is_empty()
    }

    fn searchable_items_for_scope(&self) -> Vec<String> {
        let commits = |items: &[crate::git::CommitInfo]| {
            items
                .iter()
                .map(|c| {
                    format!(
                        "{} {} {}",
                        &c.oid[..7.min(c.oid.len())],
                        c.author,
                        c.message
                    )
                })
                .collect()
        };
        match self.view_context() {
            ViewContext::Files => self
                .files
                .view()
                .tree_nodes
                .iter()
                .map(Self::tree_node_display_name)
                .collect(),
            ViewContext::Branches => self
                .ui
                .branches
                .items
                .iter()
                .map(|b| b.name.clone())
                .collect(),
            ViewContext::BranchCommits { .. } => commits(&self.ui.branches.commits_subview.items),
            ViewContext::Commits => commits(&self.ui.commits.items),
            ViewContext::CommitFiles { .. } => self
                .ui
                .commits
                .tree_mode
                .nodes
                .iter()
                .map(Self::tree_node_display_name)
                .collect(),
            ViewContext::Stashes => self
                .ui
                .stash
                .items
                .iter()
                .map(|s| format!("stash@{{{}}} {}", s.index, s.message))
                .collect(),
            ViewContext::StashFiles { .. } => self
                .ui
                .stash
                .tree_mode
                .nodes
                .iter()
                .map(Self::tree_node_display_name)
                .collect(),
        }
    }

    fn active_panel_state_selected(&self) -> Option<usize> {
        self.selection().selected_index()
    }

    pub(super) fn select_active_panel_index(&mut self, idx: usize) {
        match self.view_context() {
            ViewContext::Files => self.files.select_index(idx),
            ViewContext::Branches => self.ui.branches.panel.list_state.select(Some(idx)),
            ViewContext::BranchCommits { .. } => self
                .ui
                .branches
                .commits_subview
                .panel
                .list_state
                .select(Some(idx)),
            ViewContext::Commits | ViewContext::CommitFiles { .. } => {
                self.ui.commits.panel.list_state.select(Some(idx))
            }
            ViewContext::Stashes | ViewContext::StashFiles { .. } => {
                self.ui.stash.panel.list_state.select(Some(idx))
            }
        }
    }

    fn tree_node_display_name(node: &FileTreeNode) -> String {
        let name = node
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| node.path.display().to_string());
        if node.is_dir {
            format!("{}/", name)
        } else {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::stores::test_support::MockRepo;
    use pretty_assertions::assert_eq;

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("app")
    }

    #[test]
    fn start_search_input_sets_search_input_mode() {
        let mut app = mock_app();

        app.start_search_input();

        assert_eq!(app.input.mode, Some(crate::app::InputMode::Search));
    }

    #[test]
    fn apply_search_query_non_empty_updates_query_and_scope_presence() {
        let mut app = mock_app();

        app.apply_search_query("foo".to_string());

        assert_eq!(app.input.search_query, "foo");
        assert_eq!(app.has_search_query_for_active_scope(), true);
    }

    #[test]
    fn apply_search_query_empty_clears_query_matches_and_scope_presence() {
        let mut app = mock_app();
        app.apply_search_query("foo".to_string());

        app.apply_search_query(String::new());

        assert_eq!(app.input.search_query, "");
        assert!(app.input.search_matches.is_empty());
        assert_eq!(app.has_search_query_for_active_scope(), false);
    }

    #[test]
    fn clear_search_resets_query_matches_and_scope_presence() {
        let mut app = mock_app();
        app.apply_search_query("foo".to_string());

        app.clear_search();

        assert_eq!(app.input.search_query, "");
        assert!(app.input.search_matches.is_empty());
        assert_eq!(app.has_search_query_for_active_scope(), false);
    }

    #[test]
    fn has_search_query_for_active_scope_returns_false_when_no_query_is_applied() {
        let app = mock_app();

        assert_eq!(app.has_search_query_for_active_scope(), false);
    }

    #[test]
    fn has_search_query_for_active_scope_returns_true_when_query_is_applied() {
        let mut app = mock_app();
        app.apply_search_query("test".to_string());

        assert_eq!(app.has_search_query_for_active_scope(), true);
    }

    #[test]
    fn search_jump_next_without_matches_returns_false() {
        let mut app = mock_app();

        let jumped = app.search_jump_next();

        assert_eq!(jumped, false);
    }

    #[test]
    fn search_jump_prev_without_matches_returns_false() {
        let mut app = mock_app();

        let jumped = app.search_jump_prev();

        assert_eq!(jumped, false);
    }

    #[test]
    fn restore_search_for_active_scope_without_saved_query_clears_stale_matches() {
        let mut app = mock_app();
        app.input.search_query = "stale".to_string();
        app.input.search_matches = vec![3, 5];

        app.restore_search_for_active_scope();

        assert_eq!(app.input.search_query, "");
        assert!(app.input.search_matches.is_empty());
    }

    #[test]
    fn confirm_search_input_clears_input_mode_and_input_buffer() {
        let mut app = mock_app();
        app.input.mode = Some(crate::app::InputMode::Search);
        app.input.buffer = "test".to_string();

        app.confirm_search_input();

        assert_eq!(app.input.mode, None);
        assert_eq!(app.input.buffer, "");
    }
}
