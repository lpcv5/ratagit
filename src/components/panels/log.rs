use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, Frame};

use crate::app::CachedData;

use crate::components::core::ScrollableText;
use crate::components::Component;
use crate::components::Intent;
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

    pub fn scroll_by(&mut self, delta: i16) {
        if delta.is_negative() {
            self.scroll = self.scroll.saturating_sub(delta.unsigned_abs());
        } else {
            self.scroll = self.scroll.saturating_add(delta as u16);
        }
    }
}

impl Default for LogPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for LogPanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    return Intent::ScrollLog(-5)
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    return Intent::ScrollLog(5)
                }
                _ => {}
            }
        }

        Intent::None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        let content = if data.log_entries.is_empty() {
            "No log messages yet.".to_string()
        } else {
            data.log_entries.join("\n")
        };

        let text = ScrollableText::new(&content, "Log", is_focused, self.scroll);
        text.render(frame, area);
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
    use crate::components::test_utils::*;

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

    #[test]
    fn test_log_panel_empty_state() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = LogPanel::new();
        let data = CachedData::default();

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let line = get_buffer_line(buffer, 1);
        assert!(
            line.contains("No log messages"),
            "Expected 'No log messages' for empty log, got: {}",
            line
        );
    }

    #[test]
    fn test_log_panel_renders_entries() {
        let mut terminal = create_test_terminal(60, 10);
        let mut panel = LogPanel::new();
        let data = CachedData {
            log_entries: vec![
                "Log entry 1".to_string(),
                "Log entry 2".to_string(),
                "Log entry 3".to_string(),
            ],
            ..Default::default()
        };

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        let mut all_content = String::new();
        for row in 0..10 {
            let line = get_buffer_line(buffer, row);
            all_content.push_str(&line);
            all_content.push('\n');
        }

        assert!(
            all_content.contains("Log entry 1"),
            "Expected log entries in buffer, got:\n{}",
            all_content
        );
    }

    #[test]
    fn test_log_panel_scrolling() {
        let mut terminal = create_test_terminal(60, 8);
        let mut panel = LogPanel::new();
        let data = CachedData {
            log_entries: vec![
                "Line 1".to_string(),
                "Line 2".to_string(),
                "Line 3".to_string(),
                "Line 4".to_string(),
                "Line 5".to_string(),
                "Line 6".to_string(),
                "Line 7".to_string(),
                "Line 8".to_string(),
            ],
            ..Default::default()
        };

        // Scroll down by 3 lines
        panel.scroll_by(3);

        terminal
            .draw(|frame| {
                let area = frame.area();
                Component::render(&mut panel, frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        let mut all_content = String::new();
        for row in 0..8 {
            let line = get_buffer_line(buffer, row);
            all_content.push_str(&line);
            all_content.push('\n');
        }

        // After scrolling down, later lines should be visible
        assert!(
            all_content.contains("Line 4") || all_content.contains("Line 5"),
            "Expected scrolled content, got:\n{}",
            all_content
        );
    }

    #[test]
    fn test_log_panel_scroll_by_negative() {
        let mut panel = LogPanel::new();
        panel.scroll = 10;
        panel.scroll_by(-3);
        assert_eq!(panel.scroll, 7, "Expected scroll to decrease by 3");
    }

    #[test]
    fn test_log_panel_scroll_by_positive() {
        let mut panel = LogPanel::new();
        panel.scroll = 5;
        panel.scroll_by(3);
        assert_eq!(panel.scroll, 8, "Expected scroll to increase by 3");
    }
}
