use crate::ui::theme::UiTheme;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeState};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem, ListState},
    Frame,
};

pub fn render_revision_tree_panel(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    is_active: bool,
    tree_mode: bool,
    tree_nodes: &[FileTreeNode],
    list_items: Vec<ListItem<'static>>,
    list_state: ListState,
) {
    let theme = UiTheme::default();
    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };

    if tree_mode {
        if tree_nodes.is_empty() {
            let items = vec![ListItem::new("No files").style(Style::default().fg(theme.text_muted))];
            let list = List::new(items)
                .block(theme.panel_block(title, is_active))
                .highlight_style(highlight);
            let mut state = list_state;
            frame.render_stateful_widget(list, area, &mut state);
            return;
        }

        let widget = FileTree::new(tree_nodes.to_vec())
            .block(theme.panel_block(title, is_active))
            .highlight_style(highlight);
        let mut state = FileTreeState {
            list_state,
            expanded_dirs: std::collections::HashSet::new(),
        };
        frame.render_stateful_widget(widget, area, &mut state);
        return;
    }

    let list = List::new(list_items)
        .block(theme.panel_block(title, is_active))
        .highlight_style(highlight);
    let mut state = list_state;
    frame.render_stateful_widget(list, area, &mut state);
}
