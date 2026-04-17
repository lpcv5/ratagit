use crate::components::core::{render_stashes, SimpleListPanel};
use crate::components::component_v2::ComponentV2;
use crate::app::events::{AppEvent, GitEvent};
use crate::app::AppState;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

pub struct StashListPanel(pub SimpleListPanel);

impl StashListPanel {
    pub fn new() -> Self {
        Self(SimpleListPanel::new("Stash", render_stashes))
    }

    pub fn state_mut(&mut self) -> &mut ratatui::widgets::ListState {
        self.0.state_mut()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.0.selected_index()
    }

    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, is_focused: bool, data: &crate::app::CachedData) {
        self.0.render(frame, area, is_focused, data);
    }
}

impl Default for StashListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentV2 for StashListPanel {
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if !state.data_cache.stashes.is_empty() {
                    let current = self.0.state_mut().selected().unwrap_or(0);
                    let next = (current + 1).min(state.data_cache.stashes.len() - 1);
                    self.0.state_mut().select(Some(next));
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if !state.data_cache.stashes.is_empty() {
                    let current = self.0.state_mut().selected().unwrap_or(0);
                    let prev = current.saturating_sub(1);
                    self.0.state_mut().select(Some(prev));
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
            KeyCode::Char(' ') => AppEvent::Git(GitEvent::StashSelected),
            KeyCode::Char('p') => AppEvent::Git(GitEvent::StashSelected),
            KeyCode::Char('d') => AppEvent::Git(GitEvent::DiscardSelected),
            KeyCode::Enter => AppEvent::ActivatePanel,
            _ => AppEvent::None,
        }
    }

    fn render(&self, _area: Rect, _buf: &mut Buffer, _state: &AppState) {
        // Render implementation will be added when ComponentV2 is fully integrated
        // For now, this is a stub to satisfy the trait
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::components::test_utils::*;

    #[test]
    fn test_stash_panel_component_v2() {
        use crate::components::component_v2::ComponentV2;
        use crate::app::events::AppEvent;
        use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

        let mut panel = StashListPanel::new();
        let mut state = mock_state();

        // Add a stash entry so navigation works
        state.data_cache.stashes = vec![
            crate::backend::git_ops::StashEntry {
                index: 0,
                id: "abc123".to_string(),
                message: "Test stash".to_string(),
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
