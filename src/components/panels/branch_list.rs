use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::app::CachedData;
use crate::app::AppState;
use crate::app::events::{AppEvent, GitEvent};
use crate::components::core::{render_branches, SimpleListPanel};
use crate::components::component_v2::ComponentV2;

use super::CommitPanel;

enum BranchMode {
    List,
    CommitsSub { panel: Box<CommitPanel> },
}

pub struct BranchListPanel {
    list: SimpleListPanel,
    mode: BranchMode,
}

impl BranchListPanel {
    pub fn new() -> Self {
        Self {
            list: SimpleListPanel::new("Branches", render_branches),
            mode: BranchMode::List,
        }
    }

    pub fn state_mut(&mut self) -> &mut ratatui::widgets::ListState {
        self.list.state_mut()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.list.selected_index()
    }

    pub fn show_branch_commits(&mut self) {
        self.mode = BranchMode::CommitsSub {
            panel: Box::new(CommitPanel::new()),
        };
    }

    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, is_focused: bool, data: &CachedData) {
        match &mut self.mode {
            BranchMode::List => self.list.render(frame, area, is_focused, data),
            BranchMode::CommitsSub { panel } => {
                CommitPanel::render(panel, frame, area, is_focused, data)
            }
        }
    }
}

impl Default for BranchListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentV2 for BranchListPanel {
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !state.data_cache.branches.is_empty() {
                    let current = self.list.state_mut().selected().unwrap_or(0);
                    let next = (current + 1).min(state.data_cache.branches.len() - 1);
                    self.list.state_mut().select(Some(next));
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !state.data_cache.branches.is_empty() {
                    let current = self.list.state_mut().selected().unwrap_or(0);
                    let prev = current.saturating_sub(1);
                    self.list.state_mut().select(Some(prev));
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Enter => AppEvent::ActivatePanel,
            KeyCode::Char('d') => AppEvent::Git(GitEvent::DiscardSelected),
            _ => AppEvent::None,
        }
    }

    fn render(&self, _area: Rect, _buf: &mut Buffer, _state: &AppState) {
        // Render implementation will be added when ComponentV2 is fully integrated
        // For now, this is a stub to satisfy the trait
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::git_ops::CommitEntry;
    use crate::components::core::MultiSelectableList;

    #[test]
    fn test_branch_panel_component_v2() {
        use crate::components::component_v2::ComponentV2;
        use crate::app::events::AppEvent;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = BranchListPanel::new();
        let mut state = mock_state();

        // Add a branch entry so navigation works
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            }
        ];

        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);

        // Should return SelectionChanged for 'j' key
        assert_eq!(event, AppEvent::SelectionChanged);
    }

    fn mock_state() -> crate::app::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        crate::app::AppState::new(cmd_tx, event_rx)
    }
}

