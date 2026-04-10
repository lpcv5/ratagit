use crate::app::SidePanel;
use crate::flux::files_backend::{FilesPanelNodeStatus, FilesPanelViewState};
use crate::ui::components::organisms::{empty_list_item, title_with_search, PanelRenderContext};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode, FileTreeNodeStatus, FileTreeState};
use ratatui::{layout::Rect, Frame};

pub fn draw_files_panel(
    frame: &mut Frame,
    area: Rect,
    state: &FilesPanelViewState,
    ctx: &PanelRenderContext<'_>,
) {
    let theme = UiTheme::default();
    let is_active = ctx.active_panel == SidePanel::Files;

    let base = if state.selection.visual_mode {
        "Files [VISUAL]"
    } else {
        "Files"
    };
    let title = title_with_search(base, ctx.search_summary);
    let block = theme.panel_block(&title, is_active);

    if state.nodes.is_empty() {
        let items = empty_list_item("No changes");
        let list = ratatui::widgets::List::new(items).block(block);
        frame.render_widget(list, area);
        return;
    }

    let highlight = theme.highlight_for(is_active);
    let nodes: Vec<FileTreeNode> = state
        .nodes
        .iter()
        .map(|node| FileTreeNode {
            path: node.path.clone(),
            status: match &node.status {
                FilesPanelNodeStatus::Unstaged(status) => {
                    FileTreeNodeStatus::Unstaged(status.clone())
                }
                FilesPanelNodeStatus::Staged(status) => FileTreeNodeStatus::Staged(status.clone()),
                FilesPanelNodeStatus::Untracked => FileTreeNodeStatus::Untracked,
                FilesPanelNodeStatus::Directory => FileTreeNodeStatus::Directory,
            },
            depth: node.depth,
            is_dir: node.is_dir,
            is_expanded: node.is_expanded,
        })
        .collect();

    let widget = FileTree::new(&nodes)
        .block(block)
        .highlight_style(highlight)
        .search_query(ctx.search_query)
        .selected_indices(ctx.visual_selected_indices);

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(state.selection.selected_index);
    let mut tree_state = FileTreeState {
        list_state,
        expanded_dirs: std::collections::HashSet::new(),
    };

    frame.render_stateful_widget(widget, area, &mut tree_state);
}

impl DynamicPanel for FilesPanelViewState {
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
