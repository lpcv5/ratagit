use crate::app::{App, SidePanel};
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
    Frame,
};

pub fn render_stash_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();
    let items: Vec<ListItem> = if app.stashes.is_empty() {
        vec![ListItem::new("No stashes").style(Style::default().fg(theme.text_muted))]
    } else {
        let query = app.search_query_for_scope(SidePanel::Stash, false, false);
        app.stashes
            .iter()
            .map(|s| {
                let text = format!("stash@{{{}}} {}", s.index, s.message);
                let spans = highlighted_spans(&text, query, Style::default().fg(Color::Yellow));
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let mut title = if app.stash_tree_mode {
        if let Some(index) = app.stash_tree_stash_index {
            format!("Stash Files stash@{{{}}} [Esc Back]", index)
        } else {
            "Stash Files [Esc Back]".to_string()
        }
    } else {
        "Stash".to_string()
    };
    if let Some(search) = app.search_match_summary_for(SidePanel::Stash, false, app.stash_tree_mode)
    {
        title = format!("{} [{}]", title, search);
    }
    render_revision_tree_panel(
        frame,
        area,
        RevisionTreePanelProps {
            title: &title,
            is_active,
            tree_mode: app.stash_tree_mode,
            tree_nodes: &app.stash_tree_nodes,
            tree_search_query: app.search_query_for_scope(SidePanel::Stash, false, true),
            list_items: items,
            list_state: app.stash_panel.list_state,
        },
    );
}
