use crate::app::SidePanel;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::components::organisms::{
    draw_branches_panel, draw_commits_panel_view, draw_files_panel, draw_stash_panel,
    PanelRenderContext,
};
use crate::ui::panels::{
    render_branch_switch_confirm, render_command_log, render_command_palette,
    render_commit_all_confirm, render_commit_editor, render_diff_panel, render_shortcut_bar,
    render_stash_editor,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

/// Collapsed height for stash/log panels when not focused (in lines)
const COLLAPSED_HEIGHT: u16 = 3;
const SHORTCUT_BAR_HEIGHT: u16 = 1;
const PANEL_BORDER_ROWS: u16 = 2;
const LEFT_PANEL_DEFAULT_SPLIT: [u16; 3] = [40, 30, 30];
const LEFT_PANEL_FOCUS_SPLIT: [u16; 3] = [60, 20, 20];

pub fn render_layout(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    let size = frame.area();
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(SHORTCUT_BAR_HEIGHT)])
        .split(size);

    // Horizontal split: Left (30%) | Right (70%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(vertical[0]);

    let left_h = horizontal[0].height;
    let stash_focused = snapshot.active_panel == SidePanel::Stash;

    // Stash panel: COLLAPSED_HEIGHT when not focused, else share remaining space
    let stash_h = if stash_focused {
        // Give stash ~25% of left height
        (left_h / 4).max(COLLAPSED_HEIGHT)
    } else {
        COLLAPSED_HEIGHT
    };

    // Files and branches/commits share the remaining height
    let top_h = left_h.saturating_sub(stash_h);
    let top_rect = ratatui::layout::Rect {
        height: top_h,
        ..horizontal[0]
    };

    // Default split: files (40%) + branches (30%) + commits (30%)
    let default_left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[0]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[1]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[2]),
        ])
        .split(top_rect);

    let files_view = snapshot.files_view_state();
    let branches_view = snapshot.branches_view_state();
    let commits_view = snapshot.commits_view_state();
    let stash_view = snapshot.stash_view_state();

    let focus_index = match snapshot.active_panel {
        SidePanel::Files => Some(0usize),
        SidePanel::LocalBranches => Some(1usize),
        SidePanel::Commits => Some(2usize),
        SidePanel::Stash => None,
    };
    let focus_item_count = match snapshot.active_panel {
        SidePanel::Files => files_view.nodes.len(),
        SidePanel::LocalBranches => {
            if branches_view.commits_subview.active {
                branches_view.commits_subview.items.len()
            } else {
                branches_view.items.len()
            }
        }
        // Keep commit panel sizing stable when toggling between commit list and tree mode.
        // Use the parent commit list length as the single source for overflow checks.
        SidePanel::Commits => commits_view.items.len(),
        SidePanel::Stash => 0,
    };

    let should_expand_focused_left_panel = focus_index
        .map(|idx| {
            let visible_rows = usize::from(
                default_left_panels[idx]
                    .height
                    .saturating_sub(PANEL_BORDER_ROWS),
            )
            .max(1);
            focus_item_count > visible_rows
        })
        .unwrap_or(false);

    let left_constraints = if should_expand_focused_left_panel {
        let split = match focus_index {
            Some(0) => [
                LEFT_PANEL_FOCUS_SPLIT[0],
                LEFT_PANEL_FOCUS_SPLIT[1],
                LEFT_PANEL_FOCUS_SPLIT[2],
            ],
            Some(1) => [
                LEFT_PANEL_FOCUS_SPLIT[1],
                LEFT_PANEL_FOCUS_SPLIT[0],
                LEFT_PANEL_FOCUS_SPLIT[2],
            ],
            Some(2) => [
                LEFT_PANEL_FOCUS_SPLIT[1],
                LEFT_PANEL_FOCUS_SPLIT[2],
                LEFT_PANEL_FOCUS_SPLIT[0],
            ],
            _ => LEFT_PANEL_DEFAULT_SPLIT,
        };
        vec![
            Constraint::Percentage(split[0]),
            Constraint::Percentage(split[1]),
            Constraint::Percentage(split[2]),
        ]
    } else {
        vec![
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[0]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[1]),
            Constraint::Percentage(LEFT_PANEL_DEFAULT_SPLIT[2]),
        ]
    };

    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints(left_constraints)
        .split(top_rect);

    let stash_area = ratatui::layout::Rect {
        y: horizontal[0].y + top_h,
        height: stash_h,
        ..horizontal[0]
    };

    let files_ctx = PanelRenderContext {
        active_panel: snapshot.active_panel,
        panel_title_override: None,
        search_query: snapshot.files_search_query,
        search_summary: snapshot.render_cache.files_search_summary.as_deref(),
        visual_selected_indices: &snapshot.render_cache.files_visual_selected_indices,
        highlighted_oids: PanelRenderContext::empty_highlighted_oids(),
    };
    draw_files_panel(frame, left_panels[0], &files_view, &files_ctx);
    let branches_ctx = PanelRenderContext {
        active_panel: snapshot.active_panel,
        panel_title_override: None,
        search_query: snapshot.branches_search_query,
        search_summary: snapshot.render_cache.branches_search_summary.as_deref(),
        visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
        highlighted_oids: PanelRenderContext::empty_highlighted_oids(),
    };
    draw_branches_panel(frame, left_panels[1], &branches_view, &branches_ctx);

    let commits_ctx = PanelRenderContext {
        active_panel: snapshot.active_panel,
        panel_title_override: None,
        search_query: snapshot.commits_search_query,
        search_summary: snapshot.render_cache.commits_search_summary.as_deref(),
        visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
        highlighted_oids: &commits_view.highlighted_oids,
    };
    draw_commits_panel_view(frame, left_panels[2], &commits_view, &commits_ctx);

    let stash_ctx = PanelRenderContext {
        active_panel: snapshot.active_panel,
        panel_title_override: None,
        search_query: snapshot.stash_search_query,
        search_summary: snapshot.render_cache.stash_search_summary.as_deref(),
        visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
        highlighted_oids: PanelRenderContext::empty_highlighted_oids(),
    };
    draw_stash_panel(frame, stash_area, &stash_view, &stash_ctx);

    // Right side: diff + command log
    // Command log collapses to COLLAPSED_HEIGHT when stash is not the concern
    let right_h = horizontal[1].height;
    let log_h = COLLAPSED_HEIGHT;
    let diff_h = right_h.saturating_sub(log_h);

    let diff_area = ratatui::layout::Rect {
        height: diff_h,
        ..horizontal[1]
    };
    let log_area = ratatui::layout::Rect {
        y: horizontal[1].y + diff_h,
        height: log_h,
        ..horizontal[1]
    };

    let detail_view = snapshot.detail_view_state();
    render_diff_panel(frame, diff_area, &detail_view);
    let command_log_view = snapshot.command_log_view_state();
    render_command_log(frame, log_area, &command_log_view);
    let shortcut_bar_view = snapshot.shortcut_bar_view_state();
    render_shortcut_bar(frame, vertical[1], &shortcut_bar_view);
    let commit_editor_view = snapshot.commit_editor_view_state();
    render_commit_editor(frame, &commit_editor_view);
    let stash_editor_view = snapshot.stash_editor_view_state();
    render_stash_editor(frame, &stash_editor_view);
    let branch_switch_confirm_view = snapshot.branch_switch_confirm_view_state();
    render_branch_switch_confirm(frame, &branch_switch_confirm_view);
    let commit_all_confirm_view = snapshot.commit_all_confirm_view_state();
    render_commit_all_confirm(frame, &commit_all_confirm_view);
    let command_palette_view = snapshot.command_palette_view_state();
    render_command_palette(frame, &command_palette_view);
}
