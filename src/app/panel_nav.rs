use super::app::PanelState;
use crate::app::{App, SidePanel};

impl App {
    pub fn active_panel_state_mut(&mut self) -> &mut PanelState {
        match self.active_panel {
            SidePanel::Files => &mut self.files.panel,
            SidePanel::LocalBranches => &mut self.branches.panel,
            SidePanel::Commits => &mut self.commits.panel,
            SidePanel::Stash => &mut self.stash.panel,
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
            SidePanel::Files => self.files.tree_nodes.len(),
            SidePanel::LocalBranches => self.branches.items.len(),
            SidePanel::Commits => {
                if self.commits.tree_mode.active {
                    self.commits.tree_mode.nodes.len()
                } else {
                    self.commits.items.len()
                }
            }
            SidePanel::Stash => {
                if self.stash.tree_mode.active {
                    self.stash.tree_mode.nodes.len()
                } else {
                    self.stash.items.len()
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

