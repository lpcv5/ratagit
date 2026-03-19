use crate::app::{App, SidePanel};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_diff_panel(frame: &mut Frame, area: Rect, app: &App) {
    let hint = match app.active_panel {
        SidePanel::Files => "Select a file to view diff",
        SidePanel::LocalBranches => "Select a branch to view details",
        SidePanel::Commits => "Select a commit to view diff",
        SidePanel::Stash => "Select a stash entry to view diff",
    };

    let paragraph = Paragraph::new(hint)
        .style(Style::default().fg(Color::DarkGray))
        .block(Block::default().borders(Borders::ALL).title("Diff"));

    frame.render_widget(paragraph, area);
}
