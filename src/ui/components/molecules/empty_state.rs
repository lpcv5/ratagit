use crate::ui::theme::UiTheme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Paragraph, Widget},
};

/// An empty-state message that centers in spacious cards and stays compact in lists.
pub struct EmptyState<'a> {
    message: &'a str,
    theme: &'a UiTheme,
}

impl<'a> EmptyState<'a> {
    pub fn new(message: &'a str, theme: &'a UiTheme) -> Self {
        Self { message, theme }
    }
}

impl Widget for EmptyState<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let spacious = area.height >= 5 && area.width >= 28;
        let paragraph = Paragraph::new(self.message).style(
            Style::default()
                .fg(self.theme.text_muted)
                .bg(self.theme.surface),
        );
        if spacious {
            paragraph.centered().render(
                Rect::new(area.x, area.y + area.height / 2, area.width, 1),
                buf,
            );
        } else {
            paragraph.render(area, buf);
        }
    }
}
