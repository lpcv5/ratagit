use crate::app::App;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::ListItem,
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

    let title = if app.stash_tree_mode {
        if let Some(index) = app.stash_tree_stash_index {
            format!("Stash Files stash@{{{}}} [Esc Back]", index)
        } else {
            "Stash Files [Esc Back]".to_string()
        }
    } else {
        "Stash".to_string()
    };
    render_revision_tree_panel(
        frame,
        area,
        RevisionTreePanelProps {
            title: &title,
            is_active,
            tree_mode: app.stash_tree_mode,
            tree_nodes: &app.stash_tree_nodes,
            list_items: items,
            list_state: app.stash_panel.list_state,
        },
    );
}
