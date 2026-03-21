use crate::app::{CommitsPanelState, SidePanel};
use crate::git::CommitSyncState;
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

impl PanelComponent for CommitsPanelState {
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext<'_>) {
        let theme = UiTheme::default();
        let is_active = ctx.active_panel == SidePanel::Commits;

        let items: Vec<ListItem> = if self.items.is_empty() {
            vec![ListItem::new("No commits").style(Style::default().fg(theme.text_muted))]
        } else {
            self.items
                .iter()
                .map(|c| {
                    let color = match c.sync_state {
                        CommitSyncState::DefaultBranch => Color::Green,
                        CommitSyncState::RemoteBranch => Color::Yellow,
                        CommitSyncState::LocalOnly => Color::Red,
                    };
                    let hash_spans = highlighted_spans(
                        &c.short_hash,
                        ctx.search_query,
                        Style::default().fg(color),
                    );
                    let message_spans = highlighted_spans(
                        &c.message,
                        ctx.search_query,
                        Style::default().fg(Color::White),
                    );
                    let mut spans = Vec::with_capacity(hash_spans.len() + 1 + message_spans.len());
                    spans.extend(hash_spans);
                    spans.push(ratatui::text::Span::raw(" "));
                    spans.extend(message_spans);
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        let mut title = if self.tree_mode.active {
            if let Some(ref oid) = self.tree_mode.selected_source {
                format!("Commit Files {} [Esc Back]", &oid[..oid.len().min(7)])
            } else {
                "Commit Files [Esc Back]".to_string()
            }
        } else {
            "Commits [default:green remote:yellow local:red]".to_string()
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
