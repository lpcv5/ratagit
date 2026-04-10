use crate::app::{SidePanel, StashPanelState};
use crate::ui::components::organisms::{empty_list_item, title_with_search, PanelRenderContext};
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::traits::DynamicPanel;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::ListItem,
    Frame,
};

pub fn draw_stash_panel(
    frame: &mut Frame,
    area: Rect,
    state: &StashPanelState,
    ctx: &PanelRenderContext<'_>,
) {
    let is_active = ctx.active_panel == SidePanel::Stash;
    let items: Vec<ListItem> = if state.items.is_empty() {
        empty_list_item("No stashes")
    } else {
        state
            .items
            .iter()
            .map(|s| {
                let text = format!("stash@{{{}}} {}", s.index, s.message);
                let spans =
                    highlighted_spans(&text, ctx.search_query, Style::default().fg(Color::Yellow));
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let base = if state.tree_mode.active {
        if let Some(index) = state.tree_mode.selected_source {
            format!("Stash Files stash@{{{}}} [Esc Back]", index)
        } else {
            "Stash Files [Esc Back]".to_string()
        }
    } else {
        "Stash".to_string()
    };
    let title = title_with_search(&base, ctx.search_summary);
    render_revision_tree_panel(
        frame,
        area,
        RevisionTreePanelProps {
            title: &title,
            is_active,
            tree_mode: state.tree_mode.active,
            tree_nodes: &state.tree_mode.nodes,
            tree_search_query: if state.tree_mode.active {
                ctx.search_query
            } else {
                None
            },
            list_items: items,
            list_state: state.panel.list_state,
        },
    );
}

impl DynamicPanel for StashPanelState {
    /// Stash collapses to min_height when unfocused; return 0 so the layout
    /// falls back to min_height directly.
    fn default_height_percent(&self) -> u16 {
        0
    }
    fn focused_height_percent(&self) -> u16 {
        25
    }
    /// Expand as soon as there is more than one stash entry.
    fn expand_threshold(&self) -> usize {
        1
    }
    fn min_height(&self) -> u16 {
        3
    }
}
