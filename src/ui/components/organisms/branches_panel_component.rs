use crate::app::{BranchesPanelState, SidePanel};
use crate::ui::components::organisms::{
    empty_list_item, title_with_search, PanelComponent, PanelRenderContext,
};
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use crate::ui::LIST_SCROLL_PADDING;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{List, ListItem},
    Frame,
};

impl PanelComponent for BranchesPanelState {
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext<'_>) {
        if self.commits_subview_active {
            let title = self
                .commits_subview_source
                .as_ref()
                .map(|name| {
                    if self.commits_subview_loading {
                        format!("Branch Commits: {} [Loading...] [Esc Back]", name)
                    } else {
                        format!("Branch Commits: {} [Esc Back]", name)
                    }
                })
                .unwrap_or_else(|| "Branch Commits [Esc Back]".to_string());
            let commits_ctx = PanelRenderContext {
                active_panel: SidePanel::Commits,
                panel_title_override: Some(title.as_str()),
                search_query: ctx.search_query,
                search_summary: ctx.search_summary,
                visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
                highlighted_oids: &self.commits_subview.highlighted_oids,
            };
            self.commits_subview.draw(frame, area, &commits_ctx);
            return;
        }

        let theme = UiTheme::default();
        let is_active = ctx.active_panel == SidePanel::LocalBranches;

        let items: Vec<ListItem> = if self.items.is_empty() {
            empty_list_item("No branches")
        } else {
            self.items
                .iter()
                .map(|b| {
                    let (prefix, color) = if b.is_current {
                        ("* ", theme.accent)
                    } else {
                        ("  ", theme.text_primary)
                    };
                    let text = format!("{}{}", prefix, b.name);
                    let spans =
                        highlighted_spans(&text, ctx.search_query, Style::default().fg(color));
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        let highlight = theme.highlight_for(is_active);
        let title = title_with_search("Local Branches", ctx.search_summary);

        let list = List::new(items)
            .block(theme.panel_block(&title, is_active))
            .scroll_padding(LIST_SCROLL_PADDING)
            .highlight_style(highlight);

        let mut state = self.panel.list_state;
        frame.render_stateful_widget(list, area, &mut state);
    }
}

impl DynamicPanel for BranchesPanelState {
    fn default_height_percent(&self) -> u16 {
        25
    }
    /// Branches panel does not expand on focus; content rarely overflows.
    fn focused_height_percent(&self) -> u16 {
        25
    }
    /// Use usize::MAX so should_expand always returns false.
    fn expand_threshold(&self) -> usize {
        usize::MAX
    }
    fn min_height(&self) -> u16 {
        3
    }
}
