use crate::app::App;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{List, ListItem},
    Frame,
};

pub fn render_stash_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();

    let items: Vec<ListItem> = if app.stashes.is_empty() {
        vec![ListItem::new("No stashes").style(Style::default().fg(theme.text_muted))]
    } else {
        app.stashes
            .iter()
            .map(|s| {
                let text = format!("stash@{{{}}} {}", s.index, s.message);
                ListItem::new(text).style(Style::default().fg(Color::Yellow))
            })
            .collect()
    };

    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };
    let list = List::new(items)
        .block(theme.panel_block("Stash", is_active))
        .highlight_style(highlight);

    let mut state = app.stash_panel.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}
