use super::card;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    widgets::Clear,
    Frame,
};

/// Clear the covered content and return geometry from the same card that is drawn.
pub fn render_modal(frame: &mut Frame, area: Rect, title: &str, theme: &UiTheme) -> Rect {
    let block = card(theme, title, false).title_style(
        Style::default()
            .fg(theme.text_primary)
            .add_modifier(Modifier::BOLD),
    );
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    inner
}
