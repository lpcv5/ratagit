use crate::app::{App, SidePanel};
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{List, ListItem},
    Frame,
};

pub fn render_branches_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();

    let items: Vec<ListItem> = if app.branches.is_empty() {
        vec![ListItem::new("No branches").style(Style::default().fg(theme.text_muted))]
    } else {
        let query = app.search_query_for_scope(SidePanel::LocalBranches, false, false);
        app.branches
            .iter()
            .map(|b| {
                let (prefix, color) = if b.is_current {
                    ("* ", theme.accent)
                } else {
                    ("  ", theme.text_primary)
                };
                let text = format!("{}{}", prefix, b.name);
                let spans = highlighted_spans(&text, query, Style::default().fg(color));
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };
    let mut title = "Local Branches".to_string();
    if let Some(search) = app.search_match_summary_for(SidePanel::LocalBranches, false, false) {
        title = format!("{} [{}]", title, search);
    }

    let list = List::new(items)
        .block(theme.panel_block(&title, is_active))
        .highlight_style(highlight);

    let mut state = app.branches_panel.list_state;
    frame.render_stateful_widget(list, area, &mut state);
}
