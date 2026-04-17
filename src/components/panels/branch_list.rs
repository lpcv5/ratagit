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
    use crossterm::event::KeyModifiers;

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

    #[test]
    fn test_branch_list_navigation_wraparound() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        // Start at first branch
        panel.state_mut().select(Some(0));

        // Navigate down twice (should stop at last item, not wrap)
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        panel.handle_key_event(key_j, &state);
        panel.handle_key_event(key_j, &state);

        assert_eq!(panel.selected_index(), Some(1)); // Should be at last item
    }

    #[test]
    fn test_branch_list_checkout_event() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        panel.state_mut().select(Some(1)); // Select feature branch

        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);

        // Should return ActivatePanel event (which triggers branch checkout or commit view)
        assert_eq!(event, AppEvent::ActivatePanel);
    }

    #[test]
    fn test_branch_list_delete_non_current() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        panel.state_mut().select(Some(1)); // Select non-current branch

        let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_d, &state);

        // Should return Git event for discard (branch delete uses 'd' key)
        assert!(matches!(event, AppEvent::Git(GitEvent::DiscardSelected)));
    }

    #[test]
    fn test_branch_list_delete_current_branch() {
        let mut panel = BranchListPanel::new();
        let mut state = mock_state();
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
        ];

        panel.state_mut().select(Some(0)); // Select current branch

        let key_d = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_d, &state);

        // 'd' key always returns DiscardSelected event, regardless of branch
        // The actual logic to prevent deleting current branch is in the processor/handler
        assert!(matches!(event, AppEvent::Git(GitEvent::DiscardSelected)));
    }

    #[test]
    fn test_branch_list_empty_state() {
        let mut panel = BranchListPanel::new();
        let state = mock_state();
        // No branches in state

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);

        // Should return None when no branches
        assert_eq!(event, AppEvent::None);
    }


    fn mock_state() -> crate::app::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        crate::app::AppState::new(cmd_tx, event_rx)
    }
}

