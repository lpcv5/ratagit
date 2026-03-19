use crate::app::{App, SidePanel};
use crate::ui::panels::{
    render_branches_panel, render_command_log, render_commits_panel, render_diff_panel,
    render_files_panel, render_shortcut_bar, render_stash_panel,
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
        .constraints([
            Constraint::Min(0),
            Constraint::Length(SHORTCUT_BAR_HEIGHT),
        ])
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

    render_files_panel(frame, left_panels[0], app, app.active_panel == SidePanel::Files);
    render_branches_panel(frame, left_panels[1], app, app.active_panel == SidePanel::LocalBranches);
    render_commits_panel(frame, left_panels[2], app, app.active_panel == SidePanel::Commits);
    render_stash_panel(frame, stash_area, app, stash_focused);

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

    render_diff_panel(frame, diff_area, app);
    render_command_log(frame, log_area, app);
    render_shortcut_bar(frame, vertical[1], app);
}
