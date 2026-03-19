use crate::app::App;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::List,
    widgets::ListItem,
    Frame,
};

pub fn render_branches_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();

    let items: Vec<ListItem> = if app.branches.is_empty() {
        vec![ListItem::new("No branches").style(Style::default().fg(theme.text_muted))]
    } else {
        app.branches
            .iter()
            .map(|b| {
                let (prefix, color) = if b.is_current {
                    ("* ", theme.accent)
                } else {
                    ("  ", theme.text_primary)
                };
                ListItem::new(format!("{}{}", prefix, b.name))
                    .style(Style::default().fg(color))
            })
            .collect()
    };

    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };
    let list = List::new(items)
        .block(theme.panel_block("Local Branches", is_active))
        .highlight_style(highlight);

    let mut state = app.branches_panel.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}
