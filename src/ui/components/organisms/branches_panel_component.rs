use crate::app::{BranchesPanelState, SidePanel};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
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
                .map(|name| format!("Branch Commits: {} [Esc Back]", name))
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
            vec![ListItem::new("No branches").style(Style::default().fg(theme.text_muted))]
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

        let highlight = if is_active {
            theme.active_highlight()
        } else {
            theme.inactive_highlight()
        };
        let mut title = "Local Branches".to_string();
        if let Some(search) = &ctx.search_summary {
            title = format!("{} [{}]", title, search);
        }

        let list = List::new(items)
            .block(theme.panel_block(&title, is_active))
            .scroll_padding(LIST_SCROLL_PADDING)
            .highlight_style(highlight);

        let mut state = self.panel.list_state;
        frame.render_stateful_widget(list, area, &mut state);
    }
}
