use crate::app::{FilesPanelState, SidePanel};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::theme::UiTheme;
use crate::ui::widgets::file_tree::{FileTree, FileTreeState};
use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem},
    Frame,
};

impl PanelComponent for FilesPanelState {
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext<'_>) {
        let theme = UiTheme::default();
        let is_active = ctx.active_panel == SidePanel::Files;

        let mut title = if self.visual_mode {
            "Files [VISUAL]"
        } else {
            "Files"
        }
        .to_string();
        if let Some(search) = &ctx.search_summary {
            title = format!("{} [{}]", title, search);
        }
        let block = theme.panel_block(&title, is_active);

        if self.tree_nodes.is_empty() {
            let items =
                vec![ListItem::new("No changes").style(Style::default().fg(theme.text_muted))];
            let list = List::new(items).block(block);
            frame.render_widget(list, area);
            return;
        }

        let highlight = if is_active {
            theme.active_highlight()
        } else {
            theme.inactive_highlight()
        };
        let widget = FileTree::new(self.tree_nodes.clone())
            .block(block)
            .highlight_style(highlight)
            .search_query(ctx.search_query)
            .selected_indices(ctx.visual_selected_indices.clone());

        let mut state = FileTreeState {
            list_state: self.panel.list_state,
            expanded_dirs: std::collections::HashSet::new(),
        };

        frame.render_stateful_widget(widget, area, &mut state);
    }
}
