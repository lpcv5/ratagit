use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{layout::Rect, Frame};

use crate::app::CachedData;

use crate::components::core::ScrollableText;
use crate::components::Component;
use crate::components::Intent;

/// 主视面板组件（持有自身滚动状态）
pub struct MainViewPanel {
    scroll: u16,
}

impl MainViewPanel {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn scroll_to(&mut self, offset: u16) {
        self.scroll = offset;
    }

    pub fn scroll_by(&mut self, delta: i16) {
        if delta.is_negative() {
            self.scroll = self.scroll.saturating_sub(delta.unsigned_abs());
        } else {
            self.scroll = self.scroll.saturating_add(delta as u16);
        }
    }
}

impl Default for MainViewPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for MainViewPanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    return Intent::ScrollMainView(-5)
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    return Intent::ScrollMainView(5)
                }
                _ => {}
            }
        }

        Intent::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        let title = if let Some((path, _)) = &data.current_diff {
            format!("Main View · Diff · {path}")
        } else {
            "Main View".to_string()
        };

        let content = data
            .current_diff
            .as_ref()
            .map(|(_, diff)| diff.as_str())
            .unwrap_or("No file diff loaded. Select a file to see its diff here.");

        let text = ScrollableText::new(content, &title, is_focused, self.scroll);
        text.render(frame, area);
    }
}
