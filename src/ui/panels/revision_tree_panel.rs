use crate::ui::components::molecules::card;
use crate::ui::components::organisms::empty_list_item;
use crate::ui::components::organisms::props::{PanelHeader, RevisionContent};
use crate::ui::theme::UiTheme;
use crate::ui::widgets::file_tree::FileTree;
use crate::ui::LIST_SCROLL_PADDING;
use ratatui::{
    layout::Rect,
    widgets::{List, ListItem},
    Frame,
};

pub struct RevisionTreePanelProps<'a> {
    pub header: PanelHeader<'a>,
    pub content: RevisionContent<'a, Vec<ListItem<'static>>>,
}

pub fn render_revision_tree_panel(
    frame: &mut Frame,
    area: Rect,
    props: RevisionTreePanelProps<'_>,
    viewport: &mut super::selection::RevisionViewport,
) {
    let header = props.header;
    let theme = UiTheme::default();
    let block = card(&theme, &header.title, header.is_active);
    let highlight = theme.highlight_for(header.is_active);
    let height = usize::from(area.height.saturating_sub(2));
    match props.content {
        RevisionContent::Tree { nodes, view_id } => {
            let list_state = viewport.state(true, view_id);
            super::selection::prepare(list_state, header.selected, nodes.len(), height);
            if !nodes.is_empty() {
                let widget = FileTree::new(nodes)
                    .block(block)
                    .search_query(header.search_query)
                    .highlight_style(highlight);
                frame.render_stateful_widget(widget, area, list_state);
            } else {
                let list = List::new(empty_list_item("No files"))
                    .block(block)
                    .scroll_padding(LIST_SCROLL_PADDING)
                    .highlight_style(highlight);
                frame.render_stateful_widget(list, area, list_state);
            }
        }
        RevisionContent::List(items) => {
            let list_state = viewport.state(false, None);
            super::selection::prepare(list_state, header.selected, items.len(), height);
            let list = List::new(items)
                .block(block)
                .scroll_padding(LIST_SCROLL_PADDING)
                .highlight_style(highlight);
            frame.render_stateful_widget(list, area, list_state);
        }
    }
}
