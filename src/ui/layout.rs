use crate::app::{App, SidePanel};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::panels::{
    render_branch_switch_confirm, render_command_log, render_commit_editor, render_diff_panel,
    render_shortcut_bar, render_stash_editor, DiffViewProps,
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

pub fn render_layout(frame: &mut Frame, app: &App) {
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
    let stash_focused = app.active_panel == SidePanel::Stash;

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

    let focus_index = match app.active_panel {
        SidePanel::Files => Some(0usize),
        SidePanel::LocalBranches => Some(1usize),
        SidePanel::Commits => Some(2usize),
        SidePanel::Stash => None,
    };
    let focus_item_count = match app.active_panel {
        SidePanel::Files => app.files.tree_nodes.len(),
        SidePanel::LocalBranches => app.branches.items.len(),
        // Keep commit panel sizing stable when toggling between commit list and tree mode.
        // Use the parent commit list length as the single source for overflow checks.
        SidePanel::Commits => app.commits.items.len(),
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
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(SidePanel::Files, false, false),
        search_summary: app.render_cache.files_search_summary.as_deref(),
        visual_selected_indices: &app.render_cache.files_visual_selected_indices,
        highlighted_oids: PanelRenderContext::empty_highlighted_oids(),
    };
    app.files.draw(frame, left_panels[0], &files_ctx);
    let branches_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(SidePanel::LocalBranches, false, false),
        search_summary: app.render_cache.branches_search_summary.as_deref(),
        visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
        highlighted_oids: PanelRenderContext::empty_highlighted_oids(),
    };
    app.branches.draw(frame, left_panels[1], &branches_ctx);

    let commits_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(
            SidePanel::Commits,
            app.commits.tree_mode.active,
            false,
        ),
        search_summary: app.render_cache.commits_search_summary.as_deref(),
        visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
        highlighted_oids: &app.commits.highlighted_oids,
    };
    app.commits.draw(frame, left_panels[2], &commits_ctx);

    let stash_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(
            SidePanel::Stash,
            false,
            app.stash.tree_mode.active,
        ),
        search_summary: app.render_cache.stash_search_summary.as_deref(),
        visual_selected_indices: PanelRenderContext::empty_visual_selected_indices(),
        highlighted_oids: PanelRenderContext::empty_highlighted_oids(),
    };
    app.stash.draw(frame, stash_area, &stash_ctx);

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

    render_diff_panel(
        frame,
        diff_area,
        DiffViewProps {
            lines: &app.current_diff,
            scroll: app.diff_scroll,
            active_panel: app.active_panel,
            is_loading: app.has_pending_diff_reload() && app.current_diff.is_empty(),
        },
    );
    render_command_log(frame, log_area, app);
    render_shortcut_bar(frame, vertical[1], app);
    render_commit_editor(frame, app);
    render_stash_editor(frame, app);
    render_branch_switch_confirm(frame, app);
}
