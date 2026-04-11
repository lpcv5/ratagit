use crate::app::SidePanel;
use crate::flux::branch_backend::BranchPanelViewState;
use crate::ui::components::organisms::{
    draw_commits_panel_view, empty_list_item, title_with_search, CommitsPanelViewState,
    CommitsTreeViewState, PanelRenderContext,
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

pub fn draw_branches_panel(
    frame: &mut Frame,
    area: Rect,
    state: &BranchPanelViewState,
    ctx: &PanelRenderContext<'_>,
) {
    if state.commits_subview.active {
        let title = state
            .commits_subview
            .source_branch
            .as_ref()
            .map(|name| {
                if state.commits_subview.loading {
                    format!("Branch Commits: {} [Loading...] [Esc Back]", name)
                } else {
                    format!("Branch Commits: {} [Esc Back]", name)
                }
            })
            .unwrap_or_else(|| "Branch Commits [Esc Back]".to_string());
        let commits_view = CommitsPanelViewState {
            selected_index: state.commits_subview.selected_index,
            items: state.commits_subview.items.clone(),
            tree_mode: CommitsTreeViewState::default(),
            highlighted_oids: state.commits_subview.highlighted_oids.clone(),
        };
        let commits_ctx = PanelRenderContext {
            active_panel: SidePanel::Commits,
            panel_title_override: Some(title.as_str()),
            search_query: ctx.search_query,
            search_summary: ctx.search_summary,
            visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
            highlighted_oids: &commits_view.highlighted_oids,
        };
        draw_commits_panel_view(frame, area, &commits_view, &commits_ctx);
        return;
    }

    let theme = UiTheme::default();
    let is_active = ctx.active_panel == SidePanel::LocalBranches;

    let items: Vec<ListItem> = if state.items.is_empty() {
        empty_list_item("No branches")
    } else {
        state
            .items
            .iter()
            .map(|b| {
                let (prefix, color) = if b.is_current {
                    ("* ", theme.accent)
                } else {
                    ("  ", theme.text_primary)
                };
                let text = format!("{}{}", prefix, b.name);
                let spans = highlighted_spans(&text, ctx.search_query, Style::default().fg(color));
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

    let mut list_state = ratatui::widgets::ListState::default();
    list_state.select(state.selection.selected_index);
    frame.render_stateful_widget(list, area, &mut list_state);
}

impl DynamicPanel for BranchPanelViewState {
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
