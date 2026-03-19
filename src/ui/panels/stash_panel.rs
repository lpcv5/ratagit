use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render_stash_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
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

    let items: Vec<ListItem> = if app.stashes.is_empty() {
        vec![ListItem::new("No stashes").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.stashes
            .iter()
            .map(|s| ListItem::new(s.as_str()).style(Style::default().fg(Color::Yellow)))
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Stash")
                .border_style(border_style)
                .title_style(title_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = app.stash_panel.list_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}
