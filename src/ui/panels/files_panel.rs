use crate::app::{App, SidePanel};
use crate::ui::theme::UiTheme;
use crate::ui::widgets::file_tree::{FileTree, FileTreeState};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem},
    Frame,
};

pub fn render_files_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let theme = UiTheme::default();
    let mut title = if app.files_visual_mode {
        "Files [VISUAL]"
    } else {
        "Files"
    }
    .to_string();
    if let Some(search) = app.search_match_summary_for(SidePanel::Files, false, false) {
        title = format!("{} [{}]", title, search);
    }
    let block = theme.panel_block(&title, is_active);

    if app.file_tree_nodes.is_empty() {
        let items = vec![ListItem::new("No changes").style(Style::default().fg(theme.text_muted))];
        let list = List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };
    let widget = FileTree::new(app.file_tree_nodes.clone())
        .block(block)
        .highlight_style(highlight)
        .search_query(app.search_query_for_scope(SidePanel::Files, false, false))
        .selected_indices(app.visual_selected_indices());

    let mut state = FileTreeState {
        list_state: app.files_panel.list_state,
        expanded_dirs: std::collections::HashSet::new(),
    };

    frame.render_stateful_widget(widget, area, &mut state);
}
