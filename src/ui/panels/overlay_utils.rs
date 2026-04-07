use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    widgets::Clear,
    Frame,
};

/// Renders the chrome (background clear + outer border block) for a modal overlay
/// and returns the inner content rect (1-cell inset from the border).
///
/// Call this after `centered_rect` to avoid repeating the same 3-line boilerplate
/// in every overlay panel.
pub fn render_overlay_chrome(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    theme: &UiTheme,
) -> Rect {
    frame.render_widget(Clear, area);
    frame.render_widget(theme.panel_block(title, true), area);
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

/// Creates a centered rectangle within the given area.
///
/// # Arguments
/// * `area` - The parent area to center within
/// * `percent_x` - Horizontal size as percentage (0-100)
/// * `percent_y` - Vertical size as percentage (0-100)
pub fn centered_rect(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
