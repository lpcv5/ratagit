use crate::app::{App, SidePanel};
use crate::git::DiffLineKind;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render_diff_panel(frame: &mut Frame, area: Rect, app: &App) {
    if app.current_diff.is_empty() {
        let hint = match app.active_panel {
            SidePanel::Files => "Select a file to view diff",
            SidePanel::LocalBranches => "Select a branch to view details",
            SidePanel::Commits => "Select a commit to view diff",
            SidePanel::Stash => "Select a stash entry to view diff",
        };
        let paragraph = ratatui::widgets::Paragraph::new(hint)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Diff"));
        frame.render_widget(paragraph, area);
        return;
    }

    let scroll = app.diff_scroll;
    let items: Vec<ListItem> = app.current_diff.iter().skip(scroll).map(|line| {
        let (style, prefix) = match line.kind {
            DiffLineKind::Added   => (Style::default().fg(Color::Green),  "+"),
            DiffLineKind::Removed => (Style::default().fg(Color::Red),    "-"),
            DiffLineKind::Header  => (Style::default().fg(Color::Cyan),   ""),
            DiffLineKind::Context => (Style::default().fg(Color::Gray),   " "),
        };
        let text = Line::from(vec![
            Span::styled(format!("{}{}", prefix, line.content), style),
        ]);
        ListItem::new(text)
    }).collect();

    let total = app.current_diff.len();
    let title = format!("Diff [{}/{}]", scroll + 1, total);

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title));

    frame.render_widget(list, area);
}
