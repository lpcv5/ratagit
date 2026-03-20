use crate::ui::theme::UiTheme;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeState};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem, ListState},
    Frame,
};

pub struct RevisionTreePanelProps<'a> {
    pub title: &'a str,
    pub is_active: bool,
    pub tree_mode: bool,
    pub tree_nodes: &'a [FileTreeNode],
    pub tree_search_query: Option<&'a str>,
    pub list_items: Vec<ListItem<'static>>,
    pub list_state: ListState,
}

pub fn render_revision_tree_panel(
    frame: &mut Frame,
    area: Rect,
    props: RevisionTreePanelProps<'_>,
) {
    let RevisionTreePanelProps {
        title,
        is_active,
        tree_mode,
        tree_nodes,
        tree_search_query,
        list_items,
        list_state,
    } = props;
    let theme = UiTheme::default();
    let highlight = if is_active {
        theme.active_highlight()
    } else {
        theme.inactive_highlight()
    };

    if tree_mode {
        if tree_nodes.is_empty() {
            let items =
                vec![ListItem::new("No files").style(Style::default().fg(theme.text_muted))];
            let list = List::new(items)
                .block(theme.panel_block(title, is_active))
                .highlight_style(highlight);
            let mut state = list_state;
            frame.render_stateful_widget(list, area, &mut state);
            return;
        }

        let widget = FileTree::new(tree_nodes.to_vec())
            .block(theme.panel_block(title, is_active))
            .search_query(tree_search_query)
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
