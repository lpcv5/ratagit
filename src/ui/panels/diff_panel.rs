use crate::app::{App, SidePanel};
use crate::git::DiffLineKind;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::List,
    widgets::ListItem,
    Frame,
};

pub fn render_diff_panel(frame: &mut Frame, area: Rect, app: &App) {
    let theme = UiTheme::default();

    if app.current_diff.is_empty() {
        let hint = match app.active_panel {
            SidePanel::Files => "Select a file to view diff",
            SidePanel::LocalBranches => "Select a branch to view details",
            SidePanel::Commits => "Select a commit/file to view diff",
            SidePanel::Stash => "Select a stash entry/file to view diff",
        };
        let paragraph = ratatui::widgets::Paragraph::new(hint)
            .style(Style::default().fg(theme.text_muted))
            .block(theme.panel_block("Diff", true));
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
        .block(theme.panel_block("Diff", true).title(title));

    frame.render_widget(list, area);
}
