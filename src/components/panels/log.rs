use ratatui::layout::Rect;

use crate::app::CachedData;

use crate::components::component_v2::ComponentV2;
use crate::app::events::AppEvent;
use crate::app::AppState;
use ratatui::buffer::Buffer;

/// 日志面板组件（持有自身滚动状态）
pub struct LogPanel {
    scroll: u16,
}

impl LogPanel {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    #[allow(dead_code)] // Reserved for future scroll functionality
    pub fn scroll_by(&mut self, delta: i16) {
        if delta.is_negative() {
            self.scroll = self.scroll.saturating_sub(delta.unsigned_abs());
        } else {
            self.scroll = self.scroll.saturating_add(delta as u16);
        }
    }

    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(&mut self, frame: &mut ratatui::Frame, area: ratatui::layout::Rect, is_focused: bool, data: &CachedData) {
        use crate::components::core::ScrollableText;

        let content = if data.log_entries.is_empty() {
            "No log messages yet.".to_string()
        } else {
            data.log_entries.join("\n")
        };

        let text = ScrollableText::new(&content, "Log", is_focused, self.scroll);
        text.render(frame, area);
    }
}

impl Default for LogPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentV2 for LogPanel {
    fn handle_key_event(&mut self, _key: crossterm::event::KeyEvent, _state: &AppState) -> AppEvent {
        // Display-only component - no keyboard input handling
        AppEvent::None
    }

    fn render(&self, _area: Rect, _buf: &mut Buffer, _state: &AppState) {
        // Delegate to existing Component::render
        // This is a placeholder - actual rendering happens via Component trait
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;
    use crossterm::event::KeyCode;

    #[test]
    fn test_log_panel_component_v2() {
        let mut panel = LogPanel::new();
        let state = mock_state();
        let key = crossterm::event::KeyEvent::new(
            KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        );

        // Display-only component should always return AppEvent::None
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::None);
    }

    fn mock_state() -> AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        AppState::new(cmd_tx, event_rx)
    }

}
