use crate::app::files::FilesRenderView;
use crate::app::SidePanel;
use crate::ui::components::molecules::{card, EmptyState};
use crate::ui::components::organisms::{title_with_search, PanelRenderContext};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use crate::ui::widgets::file_tree::FileTree;
use ratatui::{layout::Rect, Frame};

pub fn draw_files_panel(
    frame: &mut Frame,
    area: Rect,
    state: &FilesRenderView<'_>,
    ctx: &PanelRenderContext<'_>,
    viewport: &mut ratatui::widgets::ListState,
) {
    let theme = UiTheme::default();
    let is_active = !ctx.diff_focused && ctx.active_panel == SidePanel::Files;

    let base = if state.visual_mode {
        "Files [VISUAL]"
    } else {
        "Files"
    };
    let title = title_with_search(base, ctx.search_summary);
    let block = card(&theme, &title, is_active);
    crate::ui::panels::selection::prepare(
        viewport,
        state.panel,
        state.tree_nodes.len(),
        usize::from(block.inner(area).height),
    );

    if state.tree_nodes.is_empty() {
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(EmptyState::new("No changes", &theme), inner);
        return;
    }

    let highlight = theme.highlight_for(is_active);
    let widget = FileTree::new(state.tree_nodes)
        .block(block)
        .highlight_style(highlight)
        .search_query(ctx.search_query)
        .selected_indices(ctx.visual_selected_indices);

    frame.render_stateful_widget(widget, area, viewport);
}

impl DynamicPanel for FilesRenderView<'_> {
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
