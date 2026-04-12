use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, Frame};

use crate::app::CachedData;

use crate::components::core::ScrollableText;
use crate::components::Component;
use crate::components::Intent;

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

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        let content = if data.log_entries.is_empty() {
            "No log messages yet.".to_string()
        } else {
            data.log_entries.join("\n")
        };

        let text = ScrollableText::new(&content, "Log", is_focused, self.scroll);
        text.render(frame, area);
    }
}
