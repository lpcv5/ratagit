use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};

use crate::app::CachedData;

use crate::components::core::panel_block;
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

        let block = panel_block(&title, is_focused);
        let paragraph = if is_diff_text(content) {
            Paragraph::new(colorize_diff_text(content))
        } else {
            Paragraph::new(content.to_string())
        }
        .block(block)
        .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }
}

fn is_diff_text(content: &str) -> bool {
    content.lines().any(|line| {
        line.starts_with("diff --git ")
            || line.starts_with("index ")
            || line.starts_with("@@ ")
            || line.starts_with("+++ ")
            || line.starts_with("--- ")
    })
}

fn colorize_diff_text(content: &str) -> Text<'static> {
    let lines: Vec<Line<'static>> = content.lines().map(colorize_diff_line).collect();
    Text::from(lines)
}

fn colorize_diff_line(line: &str) -> Line<'static> {
    let style = if line.starts_with("diff --git ") {
        Style::default().fg(Color::Rgb(116, 182, 247))
    } else if line.starts_with("index ")
        || line.starts_with("new file mode ")
        || line.starts_with("deleted file mode ")
        || line.starts_with("similarity index ")
        || line.starts_with("rename from ")
        || line.starts_with("rename to ")
    {
        Style::default().fg(Color::Rgb(245, 196, 109))
    } else if line.starts_with("@@ ") {
        Style::default().fg(Color::Rgb(191, 151, 255))
    } else if line.starts_with("+++ ") || line.starts_with("--- ") {
        Style::default().fg(Color::Rgb(142, 201, 255))
    } else if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(Color::Rgb(122, 214, 154))
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(Color::Rgb(239, 122, 122))
    } else if line.starts_with("\\ No newline at end of file") {
        Style::default().fg(Color::Rgb(164, 164, 172))
    } else {
        Style::default()
    };

    Line::from(Span::styled(line.to_string(), style))
}
