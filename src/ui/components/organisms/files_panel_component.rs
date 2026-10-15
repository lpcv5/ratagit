use super::props::FilesPanelProps;
use crate::ui::components::molecules::{card, EmptyState};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use crate::ui::widgets::file_tree::FileTree;
use ratatui::{layout::Rect, Frame};

pub fn draw_files_panel(
    frame: &mut Frame,
    area: Rect,
    props: FilesPanelProps<'_>,
    viewport: &mut ratatui::widgets::ListState,
) {
    let theme = UiTheme::default();
    let header = props.header;
    let block = card(&theme, &header.title, header.is_active);
    crate::ui::panels::selection::prepare(
        viewport,
        header.selected,
        props.nodes.len(),
        usize::from(block.inner(area).height),
    );
    if props.nodes.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(EmptyState::new("No changes", &theme), inner);
        return;
    }
    let widget = FileTree::new(props.nodes)
        .block(block)
        .highlight_style(theme.highlight_for(header.is_active))
        .search_query(header.search_query)
        .selected_indices(props.selected_indices);
    frame.render_stateful_widget(widget, area, viewport);
}

impl DynamicPanel for FilesPanelProps<'_> {
    fn default_height_percent(&self) -> u16 {
        25
    }
    fn focused_height_percent(&self) -> u16 {
        40
    }
    fn expand_threshold(&self) -> usize {
        10
    }
    fn min_height(&self) -> u16 {
        3
    }
}
