use ratatui::layout::{Constraint, Direction, Layout, Rect};

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
