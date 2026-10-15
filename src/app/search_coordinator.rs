//! Provide panel data and apply navigation/input changes around the pure search feature.
use super::search::{SearchAction, SearchContext, SearchUpdate};
#[cfg(test)]
use crate::app::App;
use crate::app::{AppState, SearchScopeKey, SidePanel, ViewContext};
use crate::file_tree::FileTreeNode;

impl AppState {
    pub fn start_search_input(&mut self) {
        self.input.mode = Some(crate::app::InputMode::Search);
        self.input.buffer = self.search.view().query.to_owned();
        self.reduce_search(SearchAction::CaptureScope);
    }

    pub fn apply_search_query(&mut self, query: String) -> usize {
        let items = if query.is_empty() {
            Vec::new()
        } else {
            self.searchable_items_for_scope()
        };
        self.reduce_search(SearchAction::SetQuery {
            query,
            items: &items,
        })
        .match_count
    }

    pub fn confirm_search_input(&mut self) {
        self.input.mode = None;
        self.input.buffer.clear();
    }

    pub fn clear_search(&mut self) {
        self.reduce_search(SearchAction::Clear);
        self.input.buffer.clear();
    }

    pub fn restore_search_for_active_scope(&mut self) {
        let scope = self.view_context();
        // Preserve the empty-query fast path: adapting history can allocate proportionally to its size.
        let items = if self
            .search
            .view()
            .saved_query(&scope)
            .is_some_and(|query| !query.is_empty())
        {
            self.searchable_items_for_scope()
        } else {
            Vec::new()
        };
        self.reduce_search(SearchAction::Restore { items: &items });
    }

    pub fn has_search_query_for_active_scope(&self) -> bool {
        if self.search.view().query.is_empty() {
            return false;
        }
        self.search.view().has_query(&self.view_context())
    }

    pub fn has_search_for_active_scope(&self) -> bool {
        if self.search.view().query.is_empty() {
            return false;
        }
        self.search.view().has_search(&self.view_context())
    }

    pub fn search_match_summary_for(
        &self,
        panel: SidePanel,
        commit_tree_mode: bool,
        stash_tree_mode: bool,
    ) -> Option<String> {
        if self.search.view().query.is_empty() {
            return None;
        }
        let scope = self.scope_for(panel, commit_tree_mode, stash_tree_mode);
        self.search.view().query_for_scope(&scope)?;
        self.search
            .view()
            .summary(&scope, self.active_panel_state_selected())
    }

    pub fn search_query_for_scope(
        &self,
        panel: SidePanel,
        commit_tree_mode: bool,
        stash_tree_mode: bool,
    ) -> Option<&str> {
        if self.search.view().query.is_empty() {
            return None;
        }
        self.search.view().query_for_scope(&self.scope_for(
            panel,
            commit_tree_mode,
            stash_tree_mode,
        ))
    }

    pub fn search_jump_next(&mut self) -> bool {
        self.apply_search_navigation(SearchAction::Next)
    }
    pub fn search_jump_prev(&mut self) -> bool {
        self.apply_search_navigation(SearchAction::Previous)
    }
    pub fn search_select_initial_match(&mut self) -> bool {
        self.apply_search_navigation(SearchAction::SelectInitial)
    }

    fn reduce_search(&mut self, action: SearchAction<'_>) -> SearchUpdate {
        let scope = self.view_context();
        let selected = if matches!(
            action,
            SearchAction::SelectInitial | SearchAction::Next | SearchAction::Previous
        ) {
            self.active_panel_state_selected()
        } else {
            None
        };
        self.search.update(
            action,
            SearchContext {
                scope: &scope,
                selected,
            },
        )
    }

    fn apply_search_navigation(&mut self, action: SearchAction<'_>) -> bool {
        let update = self.reduce_search(action);
        let Some(target) = update.navigate_to else {
            return false;
        };
        self.select_active_panel_index(target);
        true
    }

    fn scope_for(&self, panel: SidePanel, commit_tree: bool, stash_tree: bool) -> SearchScopeKey {
        match panel {
            SidePanel::Commits if commit_tree => ViewContext::CommitFiles {
                oid: self
                    .revisions
                    .view()
                    .commits
                    .tree_mode
                    .selected_source
                    .clone()
                    .unwrap_or_default(),
            },
            SidePanel::Commits => ViewContext::Commits,
            SidePanel::Stash if stash_tree => ViewContext::StashFiles {
                index: self.revisions.view().stash.tree_mode.selected_source,
            },
            SidePanel::Stash => ViewContext::Stashes,
            _ => ViewContext::from_panels(
                panel,
                &self.revisions.view().branches,
                &self.revisions.view().commits,
                &self.revisions.view().stash,
            ),
        }
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
                .revisions
                .view()
                .branches
                .items
                .iter()
                .map(|b| b.name.clone())
                .collect(),
            ViewContext::BranchCommits { .. } => {
                commits(&self.revisions.view().branches.commits_subview.items)
            }
            ViewContext::Commits => commits(&self.revisions.view().commits.items),
            ViewContext::CommitFiles { .. } => self
                .revisions
                .view()
                .commits
                .tree_mode
                .nodes
                .iter()
                .map(Self::tree_node_display_name)
                .collect(),
            ViewContext::Stashes => self
                .revisions
                .view()
                .stash
                .items
                .iter()
                .map(|s| format!("stash@{{{}}} {}", s.index, s.message))
                .collect(),
            ViewContext::StashFiles { .. } => self
                .revisions
                .view()
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
        if self.view_context() == ViewContext::Files {
            self.files.select_index(idx);
        } else {
            self.reduce_revisions(super::revisions::RevisionAction::SelectIndex(idx));
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

        assert_eq!(app.search.view().query, "foo");
        assert_eq!(app.has_search_query_for_active_scope(), true);
    }

    #[test]
    fn apply_search_query_empty_clears_query_matches_and_scope_presence() {
        let mut app = mock_app();
        app.apply_search_query("foo".to_string());

        app.apply_search_query(String::new());

        assert_eq!(app.search.view().query, "");
        assert!(app.search.view().matches.is_empty());
        assert_eq!(app.has_search_query_for_active_scope(), false);
    }

    #[test]
    fn clear_search_resets_query_matches_and_scope_presence() {
        let mut app = mock_app();
        app.apply_search_query("foo".to_string());

        app.clear_search();

        assert_eq!(app.search.view().query, "");
        assert!(app.search.view().matches.is_empty());
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
        app.search.set_test_current("stale".into(), vec![3, 5]);

        app.restore_search_for_active_scope();

        assert_eq!(app.search.view().query, "");
        assert!(app.search.view().matches.is_empty());
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

#[cfg(test)]
mod regression_tests;
