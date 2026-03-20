use crate::app::{App, SearchScopeKey, SidePanel};
use crate::ui::widgets::file_tree::FileTreeNode;

impl App {
    pub fn start_search_input(&mut self) {
        self.input_mode = Some(crate::app::InputMode::Search);
        self.input_buffer = self.search_query.clone();
        self.capture_search_scope();
    }

    pub fn apply_search_query(&mut self, query: String) -> usize {
        self.search_query = query;
        self.capture_search_scope();
        let scope = self.current_search_scope_key();
        if self.search_query.is_empty() {
            self.search_matches.clear();
            self.search_queries.remove(&scope);
            return 0;
        }
        self.search_queries.insert(scope, self.search_query.clone());
        self.search_matches = self
            .searchable_items_for_scope()
            .into_iter()
            .enumerate()
            .filter_map(|(idx, text)| {
                if text
                    .to_lowercase()
                    .contains(&self.search_query.to_lowercase())
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
        self.search_matches.len()
    }

    pub fn confirm_search_input(&mut self) {
        self.input_mode = None;
        self.input_buffer.clear();
    }

    pub fn clear_search(&mut self) {
        self.capture_search_scope();
        let scope = self.current_search_scope_key();
        self.search_query.clear();
        self.input_buffer.clear();
        self.search_matches.clear();
        self.search_queries.remove(&scope);
    }

    pub fn restore_search_for_active_scope(&mut self) {
        self.capture_search_scope();
        let scope = self.current_search_scope_key();
        let query = self.search_queries.get(&scope).cloned().unwrap_or_default();
        self.search_query = query;
        if self.search_query.is_empty() {
            self.search_matches.clear();
            return;
        }
        self.search_matches = self
            .searchable_items_for_scope()
            .into_iter()
            .enumerate()
            .filter_map(|(idx, text)| {
                if text
                    .to_lowercase()
                    .contains(&self.search_query.to_lowercase())
                {
                    Some(idx)
                } else {
                    None
                }
            })
            .collect();
    }

    pub fn has_search_query_for_active_scope(&self) -> bool {
        if self.search_query.is_empty() {
            return false;
        }
        self.search_scope_panel == self.active_panel
            && self.search_scope_commit_tree_mode
                == Self::normalize_commit_scope(self.active_panel, self.commit_tree_mode)
            && self.search_scope_stash_tree_mode
                == Self::normalize_stash_scope(self.active_panel, self.stash_tree_mode)
    }

    pub fn has_search_for_active_scope(&self) -> bool {
        if self.search_query.is_empty() {
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
        if self.search_query.is_empty() {
            return None;
        }
        if panel != self.search_scope_panel
            || Self::normalize_commit_scope(panel, commit_tree_mode)
                != self.search_scope_commit_tree_mode
            || Self::normalize_stash_scope(panel, stash_tree_mode)
                != self.search_scope_stash_tree_mode
        {
            return None;
        }
        if self.search_matches.is_empty() {
            return Some(format!("/{query} 0/0", query = self.search_query));
        }
        let selected = self.active_panel_state_selected();
        let current = selected
            .and_then(|idx| self.search_matches.iter().position(|m| *m == idx))
            .map(|pos| pos + 1)
            .unwrap_or(1);
        Some(format!(
            "/{query} {current}/{total}",
            query = self.search_query,
            total = self.search_matches.len()
        ))
    }

    pub fn search_query_for_scope(
        &self,
        panel: SidePanel,
        commit_tree_mode: bool,
        stash_tree_mode: bool,
    ) -> Option<&str> {
        if self.search_query.is_empty() {
            return None;
        }
        if panel != self.search_scope_panel
            || Self::normalize_commit_scope(panel, commit_tree_mode)
                != self.search_scope_commit_tree_mode
            || Self::normalize_stash_scope(panel, stash_tree_mode)
                != self.search_scope_stash_tree_mode
        {
            return None;
        }
        Some(self.search_query.as_str())
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
            .search_matches
            .iter()
            .copied()
            .find(|idx| *idx >= selected)
            .or_else(|| self.search_matches.first().copied());
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
            self.search_matches
                .iter()
                .copied()
                .find(|idx| *idx > selected)
                .or_else(|| self.search_matches.first().copied())
        } else {
            self.search_matches
                .iter()
                .rev()
                .copied()
                .find(|idx| *idx < selected)
                .or_else(|| self.search_matches.last().copied())
        };
        let Some(target) = next else {
            return false;
        };
        self.select_active_panel_index(target);
        true
    }

    fn capture_search_scope(&mut self) {
        self.search_scope_panel = self.active_panel;
        self.search_scope_commit_tree_mode =
            Self::normalize_commit_scope(self.active_panel, self.commit_tree_mode);
        self.search_scope_stash_tree_mode =
            Self::normalize_stash_scope(self.active_panel, self.stash_tree_mode);
    }

    fn current_search_scope_key(&self) -> SearchScopeKey {
        SearchScopeKey {
            panel: self.search_scope_panel,
            commit_tree_mode: self.search_scope_commit_tree_mode,
            stash_tree_mode: self.search_scope_stash_tree_mode,
        }
    }

    fn search_scope_matches_active(&self) -> bool {
        self.search_scope_panel == self.active_panel
            && self.search_scope_commit_tree_mode
                == Self::normalize_commit_scope(self.active_panel, self.commit_tree_mode)
            && self.search_scope_stash_tree_mode
                == Self::normalize_stash_scope(self.active_panel, self.stash_tree_mode)
            && !self.search_matches.is_empty()
    }

    fn searchable_items_for_scope(&self) -> Vec<String> {
        match self.active_panel {
            SidePanel::Files => self
                .file_tree_nodes
                .iter()
                .map(Self::tree_node_display_name)
                .collect(),
            SidePanel::LocalBranches => self.branches.iter().map(|b| b.name.clone()).collect(),
            SidePanel::Commits => {
                if self.commit_tree_mode {
                    return self
                        .commit_tree_nodes
                        .iter()
                        .map(Self::tree_node_display_name)
                        .collect();
                }
                self.commits
                    .iter()
                    .map(|c| format!("{} {} {} {}", c.short_hash, c.message, c.author, c.time))
                    .collect()
            }
            SidePanel::Stash => {
                if self.stash_tree_mode {
                    return self
                        .stash_tree_nodes
                        .iter()
                        .map(Self::tree_node_display_name)
                        .collect();
                }
                self.stashes
                    .iter()
                    .map(|s| format!("stash@{{{}}} {}", s.index, s.message))
                    .collect()
            }
        }
    }

    fn active_panel_state_selected(&self) -> Option<usize> {
        match self.active_panel {
            SidePanel::Files => self.files_panel.list_state.selected(),
            SidePanel::LocalBranches => self.branches_panel.list_state.selected(),
            SidePanel::Commits => self.commits_panel.list_state.selected(),
            SidePanel::Stash => self.stash_panel.list_state.selected(),
        }
    }

    fn select_active_panel_index(&mut self, idx: usize) {
        match self.active_panel {
            SidePanel::Files => self.files_panel.list_state.select(Some(idx)),
            SidePanel::LocalBranches => self.branches_panel.list_state.select(Some(idx)),
            SidePanel::Commits => self.commits_panel.list_state.select(Some(idx)),
            SidePanel::Stash => self.stash_panel.list_state.select(Some(idx)),
        }
    }

    fn normalize_commit_scope(panel: SidePanel, value: bool) -> bool {
        if panel == SidePanel::Commits {
            value
        } else {
            false
        }
    }

    fn normalize_stash_scope(panel: SidePanel, value: bool) -> bool {
        if panel == SidePanel::Stash {
            value
        } else {
            false
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
