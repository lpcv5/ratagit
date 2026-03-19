use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render_commits_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let border_style = if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title_style = if is_active {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let items: Vec<ListItem> = if app.commits.is_empty() {
        vec![ListItem::new("No commits").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.commits
            .iter()
            .map(|c| {
                let graph = if c.parent_count > 1 { "⑂ " } else { "● " };
                let text = format!("{}{} {} {}", graph, c.short_hash, c.message, c.author);
                ListItem::new(text).style(Style::default().fg(Color::White))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Commits")
                .border_style(border_style)
                .title_style(title_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = app.commits_panel.list_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}
