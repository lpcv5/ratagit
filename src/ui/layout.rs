use crate::app::{App, SidePanel};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::panels::{
    render_command_log, render_commit_editor, render_diff_panel, render_shortcut_bar,
    render_stash_editor, DiffViewProps,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

/// Collapsed height for stash/log panels when not focused (in lines)
const COLLAPSED_HEIGHT: u16 = 3;
const SHORTCUT_BAR_HEIGHT: u16 = 1;

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
    // Split top into files (40%) + branches (30%) + commits (30%)
    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ])
        .split(ratatui::layout::Rect {
            height: top_h,
            ..horizontal[0]
        });

    let stash_area = ratatui::layout::Rect {
        y: horizontal[0].y + top_h,
        height: stash_h,
        ..horizontal[0]
    };

    let files_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(SidePanel::Files, false, false),
        search_summary: app.search_match_summary_for(SidePanel::Files, false, false),
        visual_selected_indices: app.visual_selected_indices(),
    };
    app.files.draw(frame, left_panels[0], &files_ctx);
    let branches_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(SidePanel::LocalBranches, false, false),
        search_summary: app.search_match_summary_for(SidePanel::LocalBranches, false, false),
        visual_selected_indices: std::collections::HashSet::new(),
    };
    app.branches.draw(frame, left_panels[1], &branches_ctx);

    let commits_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(
            SidePanel::Commits,
            app.commits.tree_mode.active,
            false,
        ),
        search_summary: app.search_match_summary_for(
            SidePanel::Commits,
            app.commits.tree_mode.active,
            false,
        ),
        visual_selected_indices: std::collections::HashSet::new(),
    };
    app.commits.draw(frame, left_panels[2], &commits_ctx);

    let stash_ctx = PanelRenderContext {
        active_panel: app.active_panel,
        search_query: app.search_query_for_scope(
            SidePanel::Stash,
            false,
            app.stash.tree_mode.active,
        ),
        search_summary: app.search_match_summary_for(
            SidePanel::Stash,
            false,
            app.stash.tree_mode.active,
        ),
        visual_selected_indices: std::collections::HashSet::new(),
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
}
