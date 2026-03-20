use super::app::PanelState;
use crate::app::{App, SidePanel};

impl App {
    pub fn active_panel_state_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            SidePanel::Files => &mut self.files_panel,
            SidePanel::LocalBranches => &mut self.branches_panel,
            SidePanel::Commits => &mut self.commits_panel,
            SidePanel::Stash => &mut self.stash_panel,
        }
    }

    pub fn list_down(&mut self) {
        let count = self.active_panel_count();
        if count == 0 {
            return;
        }
        let state = self.active_panel_state_mut();
        let next = state
            .list_state
            .selected()
            .map(|i| (i + 1).min(count - 1))
            .unwrap_or(0);
        state.list_state.select(Some(next));
    }

    pub fn list_up(&mut self) {
        let count = self.active_panel_count();
        if count == 0 {
            return;
        }
        let state = self.active_panel_state_mut();
        let prev = state
            .list_state
            .selected()
            .map(|i| i.saturating_sub(1))
            .unwrap_or(0);
        state.list_state.select(Some(prev));
    }

    pub(super) fn active_panel_count(&self) -> usize {
        match self.active_panel {
            SidePanel::Files => self.file_tree_nodes.len(),
            SidePanel::LocalBranches => self.branches.len(),
            SidePanel::Commits => {
                if self.commit_tree_mode {
                    self.commit_tree_nodes.len()
                } else {
                    self.commits.len()
                }
            }
            SidePanel::Stash => {
                if self.stash_tree_mode {
                    self.stash_tree_nodes.len()
                } else {
                    self.stashes.len()
                }
            }
        }
    }

    pub(super) fn active_panel_name(&self) -> &'static str {
        match self.active_panel {
            SidePanel::Files => "files",
            SidePanel::LocalBranches => "branches",
            SidePanel::Commits => "commits",
            SidePanel::Stash => "stash",
        }
    }
}
