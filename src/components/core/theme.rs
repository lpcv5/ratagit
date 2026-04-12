use ratatui::{
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders},
};

pub const LIST_HIGHLIGHT_SYMBOL: &str = "▸ ";

pub fn panel_border_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default().fg(Color::Rgb(174, 129, 255))
    } else {
        Style::default().fg(Color::Rgb(98, 82, 138))
    }
}

pub fn panel_title_style(is_focused: bool) -> Style {
    if is_focused {
        Style::default()
            .fg(Color::Rgb(232, 222, 255))
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Rgb(162, 143, 205))
    }
}

pub fn selected_row_style() -> Style {
    Style::default()
        .fg(Color::Black)
        .bg(Color::Rgb(208, 183, 255))
        .add_modifier(Modifier::BOLD)
}

pub fn muted_text_style() -> Style {
    Style::default().fg(Color::Rgb(141, 123, 182))
}

pub fn accent_primary_color() -> Color {
    Color::Rgb(194, 170, 249)
}

pub fn accent_secondary_color() -> Color {
    Color::Rgb(167, 142, 230)
}

pub fn panel_block(title: impl Into<String>, is_focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(panel_border_style(is_focused))
        .title(Line::styled(title.into(), panel_title_style(is_focused)))
}

#[cfg(test)]
mod tests {
    use super::panel_block;
    use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget};

    #[test]
    fn panel_block_renders_rounded_corners() {
        let area = Rect::new(0, 0, 12, 4);
        let mut buffer = Buffer::empty(area);

        panel_block("Panel", true).render(area, &mut buffer);

        assert_eq!(buffer[(0, 0)].symbol(), "╭");
        assert_eq!(buffer[(11, 0)].symbol(), "╮");
        assert_eq!(buffer[(0, 3)].symbol(), "╰");
        assert_eq!(buffer[(11, 3)].symbol(), "╯");
    }
}
