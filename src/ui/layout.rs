use crate::app::{App, SidePanel};
use crate::ui::panels::{
    render_branches_panel, render_command_log, render_commits_panel, render_diff_panel,
    render_files_panel, render_stash_panel,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

pub fn render_layout(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // 水平分割: Left (30%) | Right (70%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(size);

    // 左侧 4 个面板垂直分割
    let left_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .split(horizontal[0]);

    render_files_panel(frame, left_panels[0], app, app.active_panel == SidePanel::Files);
    render_branches_panel(frame, left_panels[1], app, app.active_panel == SidePanel::LocalBranches);
    render_commits_panel(frame, left_panels[2], app, app.active_panel == SidePanel::Commits);
    render_stash_panel(frame, left_panels[3], app, app.active_panel == SidePanel::Stash);

    // 右侧: 上 Diff | 下 Git Log，各占 50%
    let right_panels = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(horizontal[1]);

    render_diff_panel(frame, right_panels[0], app);
    render_command_log(frame, right_panels[1], app);
}
