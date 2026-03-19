use crate::app::{App, SidePanel};
use crate::ui::panels::{
    render_branches_panel, render_command_log, render_commits_panel, render_diff_panel,
    render_files_panel, render_stash_panel,
};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs},
    Frame,
};

pub fn render_layout(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // 垂直分割: Tab Bar | Main Body | Command Log
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(size);

    // 渲染 Tab Bar
    render_tabs(frame, app, vertical[0]);

    // 水平分割: Left Panels (30%) | Right Diff (70%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(vertical[1]);

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

    // 右侧 Diff 面板
    render_diff_panel(frame, horizontal[1], app);

    // 底部命令日志
    render_command_log(frame, vertical[2], app);
}

fn render_tabs(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let titles = vec!["1 Status", "2 Commits", "3 Branches", "4 Stash"];
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("ratagit"))
        .select(app.current_tab as usize)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(tabs, area);
}
