use crate::app::{SidePanel, StashPanelState};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
    Frame,
};

impl PanelComponent for StashPanelState {
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext<'_>) {
        let theme = UiTheme::default();
        let is_active = ctx.active_panel == SidePanel::Stash;
        let items: Vec<ListItem> = if self.items.is_empty() {
            vec![ListItem::new("No stashes").style(Style::default().fg(theme.text_muted))]
        } else {
            self.items
                .iter()
                .map(|s| {
                    let text = format!("stash@{{{}}} {}", s.index, s.message);
                    let spans = highlighted_spans(
                        &text,
                        ctx.search_query,
                        Style::default().fg(Color::Yellow),
                    );
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        let mut title = if self.tree_mode.active {
            if let Some(index) = self.tree_mode.selected_source {
                format!("Stash Files stash@{{{}}} [Esc Back]", index)
            } else {
                "Stash Files [Esc Back]".to_string()
            }
        } else {
            "Stash".to_string()
        };
        if let Some(search) = &ctx.search_summary {
            title = format!("{} [{}]", title, search);
        }
        render_revision_tree_panel(
            frame,
            area,
            RevisionTreePanelProps {
                title: &title,
                is_active,
                tree_mode: self.tree_mode.active,
                tree_nodes: &self.tree_mode.nodes,
                tree_search_query: if self.tree_mode.active {
                    ctx.search_query
                } else {
                    None
                },
                list_items: items,
                list_state: self.panel.list_state,
            },
        );
    }
}
