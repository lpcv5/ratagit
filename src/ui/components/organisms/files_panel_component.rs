use crate::app::{FilesPanelState, SidePanel};
use crate::ui::components::organisms::{empty_list_item, title_with_search, PanelRenderContext};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use crate::ui::widgets::file_tree::{FileTree, FileTreeState};
use ratatui::{layout::Rect, Frame};

pub fn draw_files_panel(
    frame: &mut Frame,
    area: Rect,
    state: &FilesPanelState,
    ctx: &PanelRenderContext<'_>,
) {
    let theme = UiTheme::default();
    let is_active = ctx.active_panel == SidePanel::Files;

    let base = if state.visual_mode {
        "Files [VISUAL]"
    } else {
        "Files"
    };
    let title = title_with_search(base, ctx.search_summary);
    let block = theme.panel_block(&title, is_active);

    if state.tree_nodes.is_empty() {
        let items = empty_list_item("No changes");
        let list = ratatui::widgets::List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let highlight = theme.highlight_for(is_active);
    let widget = FileTree::new(&state.tree_nodes)
        .block(block)
        .highlight_style(highlight)
        .search_query(ctx.search_query)
        .selected_indices(ctx.visual_selected_indices);

    let mut tree_state = FileTreeState {
        list_state: state.panel.list_state,
        expanded_dirs: std::collections::HashSet::new(),
    };

    frame.render_stateful_widget(widget, area, &mut tree_state);
}

impl DynamicPanel for FilesPanelState {
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
