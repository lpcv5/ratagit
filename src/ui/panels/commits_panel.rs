use crate::app::App;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem},
    Frame,
};

pub fn render_commits_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();

    let items: Vec<ListItem> = if app.commits.is_empty() {
        vec![ListItem::new("No commits").style(Style::default().fg(theme.text_muted))]
    } else {
        app.commits
            .iter()
            .map(|c| {
                let graph = if c.parent_count > 1 { "⑂ " } else { "● " };
                let text = format!("{}{} {} {}", graph, c.short_hash, c.message, c.author);
                ListItem::new(text).style(Style::default().fg(theme.text_primary))
            })
            .collect()
    };

    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };
    let list = List::new(items)
        .block(theme.panel_block("Commits", is_active))
        .highlight_style(highlight);

    let mut state = app.commits_panel.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}
