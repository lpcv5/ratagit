use ratagit_core::{AppState, PanelFocus};
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::{Frame, Terminal};

use crate::frame::{TerminalSize, buffer_to_text};
use crate::panels::{
    panel_title, render_branches_lines, render_commits_lines, render_details_lines,
    render_files_lines, render_log_lines, render_stash_lines, shortcuts_for_state,
};

pub fn render_terminal(frame: &mut Frame<'_>, state: &AppState) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);

    render_panel_grid(frame, state, root[0]);
    render_shortcuts(frame, state, root[1]);
}

pub fn render_terminal_text(state: &AppState, size: TerminalSize) -> String {
    let backend = TestBackend::new(size.width.max(1) as u16, size.height.max(1) as u16);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| render_terminal(frame, state))
        .expect("terminal render should succeed");
    buffer_to_text(terminal.backend().buffer())
}

fn render_panel_grid(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(area);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(28),
            Constraint::Percentage(24),
            Constraint::Percentage(28),
            Constraint::Percentage(20),
        ])
        .split(columns[0]);
    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(columns[1]);

    render_block_panel(
        frame,
        state,
        PanelFocus::Files,
        left[0],
        render_files_lines(state, left[0].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Branches,
        left[1],
        render_branches_lines(state, left[1].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Commits,
        left[2],
        render_commits_lines(state, left[2].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Stash,
        left[3],
        render_stash_lines(state, left[3].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Details,
        right[0],
        render_details_lines(state, right[0].height.saturating_sub(2) as usize),
    );
    render_block_panel(
        frame,
        state,
        PanelFocus::Log,
        right[1],
        render_log_lines(state, right[1].height.saturating_sub(2) as usize),
    );
}

fn render_block_panel(
    frame: &mut Frame<'_>,
    state: &AppState,
    panel: PanelFocus,
    area: Rect,
    lines: Vec<String>,
) {
    let focused = state.focus == panel;
    let border_style = if focused {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title = if focused {
        format!(" {} * ", panel_title(panel))
    } else {
        format!(" {} ", panel_title(panel))
    };
    let text = lines.into_iter().map(Line::from).collect::<Vec<_>>();
    let widget = Paragraph::new(text).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style),
    );
    frame.render_widget(widget, area);
}

fn render_shortcuts(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let widget = Paragraph::new(shortcuts_for_state(state));
    frame.render_widget(widget, area);
}
