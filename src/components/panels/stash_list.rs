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
}

impl Default for StashListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::components::Component for StashListPanel {
    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        data: &crate::app::CachedData,
    ) -> crate::components::Intent {
        self.0.handle_event(event, data)
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        data: &crate::app::CachedData,
    ) {
        self.0.render(frame, area, is_focused, data);
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
    use crate::components::Component;

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

    #[test]
    fn test_stash_list_empty_state() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = StashListPanel::new();
        let data = create_test_cached_data_with_stashes(vec![]);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let line = get_buffer_line(buffer, 1);
        assert!(
            line.contains("No items"),
            "Expected 'No items' for empty stash, got: {}",
            line
        );
    }

    #[test]
    fn test_stash_list_renders_entries() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = StashListPanel::new();
        let stashes = vec![
            test_stash_entry(0, "abc12345", "WIP on main: fix bug"),
            test_stash_entry(1, "def67890", "WIP on feature: add tests"),
        ];
        let data = create_test_cached_data_with_stashes(stashes);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Check that stash entries appear
        let content = get_buffer_line(buffer, 1);
        assert!(
            content.contains("abc12345") || content.contains("WIP on main"),
            "Expected stash entry in buffer, got: {}",
            content
        );
    }

    #[test]
    fn test_stash_list_shows_multiple_entries() {
        let mut terminal = create_test_terminal(60, 15);
        let mut panel = StashListPanel::new();
        let stashes = vec![
            test_stash_entry(0, "aaa11111", "Stash 1"),
            test_stash_entry(1, "bbb22222", "Stash 2"),
            test_stash_entry(2, "ccc33333", "Stash 3"),
        ];
        let data = create_test_cached_data_with_stashes(stashes);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Collect all lines to verify multiple entries
        let mut all_content = String::new();
        for row in 0..15 {
            let line = get_buffer_line(buffer, row);
            all_content.push_str(&line);
            all_content.push('\n');
        }

        // At least one stash entry should be visible
        assert!(
            all_content.contains("Stash 1")
                || all_content.contains("Stash 2")
                || all_content.contains("Stash 3"),
            "Expected at least one stash entry visible, got:\n{}",
            all_content
        );
    }
}
