use super::card;
use crate::ui::theme::UiTheme;
use ratatui::{layout::Rect, style::Style, text::Line, widgets::Paragraph, Frame};

/// End-editing field shared by single-line inputs and the multiline editor.
/// Scroll by terminal cells, so the visible text and cursor agree for Unicode.
pub struct InputField<'a> {
    pub title: &'a str,
    pub value: &'a str,
    pub placeholder: &'a str,
    pub focused: bool,
}

pub fn render_input(frame: &mut Frame, area: Rect, field: InputField<'_>, theme: &UiTheme) {
    let InputField {
        title,
        value,
        placeholder,
        focused,
    } = field;
    let block = card(theme, title, focused);
    let inner = block.inner(area);
    let row = value
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        .min(u16::MAX as usize) as u16;
    let col = Line::from(value.rsplit('\n').next().unwrap_or(""))
        .width()
        .min(u16::MAX as usize) as u16;
    let scroll_y = row.saturating_sub(inner.height.saturating_sub(1));
    let scroll_x = col.saturating_sub(inner.width.saturating_sub(1));
    let (text, color) = if value.is_empty() {
        (placeholder, theme.text_muted)
    } else {
        (value, theme.text_primary)
    };
    frame.render_widget(
        Paragraph::new(text)
            .style(Style::default().fg(color))
            .scroll((scroll_y, scroll_x))
            .block(block),
        area,
    );
    if focused && !inner.is_empty() {
        frame.set_cursor_position((inner.x + (col - scroll_x), inner.y + (row - scroll_y)));
    }
}
