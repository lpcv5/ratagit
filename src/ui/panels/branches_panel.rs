use crate::app::App;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn render_branches_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
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

    let items: Vec<ListItem> = if app.branches.is_empty() {
        vec![ListItem::new("No branches").style(Style::default().fg(Color::DarkGray))]
    } else {
        app.branches
            .iter()
            .map(|b| {
                let (prefix, color) = if b.is_current {
                    ("* ", Color::Green)
                } else {
                    ("  ", Color::Cyan)
                };
                ListItem::new(format!("{}{}", prefix, b.name))
                    .style(Style::default().fg(color))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Local Branches")
                .border_style(border_style)
                .title_style(title_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = app.branches_panel.list_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}
