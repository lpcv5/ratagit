use ratagit_core::{AppState, CommitField, EditorKind, PanelFocus, StashScope};
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};
use ratatui::{Frame, Terminal};

use crate::frame::{TerminalBuffer, TerminalSize, buffer_to_text};
use crate::layout::compute_left_panel_heights;
use crate::panels::{
    PanelLine, panel_title, render_branches_lines, render_commits_lines, render_details_lines,
    render_files_lines, render_log_lines, render_stash_lines, shortcuts_for_state,
};
use crate::theme::{
    RowRole, batch_selected_row_style, focused_panel_style, inactive_panel_style, row_style,
    selected_row_style,
};

pub fn render_terminal(frame: &mut Frame<'_>, state: &AppState) {
    let area = frame.area();
    let root = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(6), Constraint::Length(1)])
        .split(area);

    render_panel_grid(frame, state, root[0]);
    render_shortcuts(frame, state, root[1]);
    render_editor_modal(frame, state, root[0]);
}

pub fn render_terminal_text(state: &AppState, size: TerminalSize) -> String {
    buffer_to_text(&render_terminal_buffer(state, size))
}

pub fn render_terminal_buffer(state: &AppState, size: TerminalSize) -> TerminalBuffer {
    let backend = TestBackend::new(size.width.max(1) as u16, size.height.max(1) as u16);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| render_terminal(frame, state))
        .expect("terminal render should succeed");
    terminal.backend().buffer().clone()
}

fn render_panel_grid(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(34), Constraint::Percentage(66)])
        .split(area);
    let left_heights = compute_left_panel_heights(state, columns[0].height as usize, 2);
    let left = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(left_heights.files as u16),
            Constraint::Length(left_heights.branches as u16),
            Constraint::Length(left_heights.commits as u16),
            Constraint::Length(left_heights.stash as u16),
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
    lines: Vec<PanelLine>,
) {
    let focused = state.focus == panel;
    let border_style = if focused {
        focused_panel_style()
    } else {
        inactive_panel_style()
    };
    let title = Line::styled(format!(" {} ", panel_title(panel)), border_style);
    let items = lines
        .iter()
        .map(|line| ListItem::new(Line::from(line.text.clone())).style(row_style(line.role)))
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    let selected_index = lines.iter().position(|line| line.selected);
    if focused && let Some(index) = selected_index {
        list_state.select(Some(index));
    }
    let highlight_style = if focused
        && selected_index
            .and_then(|index| lines.get(index))
            .is_some_and(|line| line.role == RowRole::BatchSelected)
    {
        batch_selected_row_style()
    } else {
        selected_row_style()
    };
    let widget = List::new(items)
        .highlight_style(highlight_style)
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(border_style),
        );
    frame.render_stateful_widget(widget, area, &mut list_state);
}

fn render_shortcuts(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let widget = Paragraph::new(shortcuts_for_state(state));
    frame.render_widget(widget, area);
}

fn render_editor_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(editor) = &state.editor.kind else {
        return;
    };

    let target_height = match editor {
        EditorKind::Commit { .. } => 12,
        EditorKind::Stash { .. } => 8,
    };
    let modal = centered_rect(area, 76, target_height);
    if modal.width < 6 || modal.height < 4 {
        return;
    }

    let (title, lines) = match editor {
        EditorKind::Commit {
            message,
            body,
            active_field,
        } => build_commit_editor_lines(message, body, *active_field),
        EditorKind::Stash { title, scope } => build_stash_editor_lines(title, scope),
    };

    frame.render_widget(Clear, modal);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .border_style(focused_panel_style()),
        ),
        modal,
    );
}

fn build_commit_editor_lines(
    message: &str,
    body: &str,
    active_field: CommitField,
) -> (&'static str, Vec<Line<'static>>) {
    let mut lines = Vec::new();
    let message_style = if active_field == CommitField::Message {
        focused_panel_style()
    } else {
        Style::default()
    };
    let body_style = if active_field == CommitField::Body {
        focused_panel_style()
    } else {
        Style::default()
    };
    let message_prefix = if active_field == CommitField::Message {
        ">>"
    } else {
        "  "
    };
    let body_prefix = if active_field == CommitField::Body {
        ">>"
    } else {
        "  "
    };
    lines.push(Line::styled(
        format!("{message_prefix} message: {message}"),
        message_style,
    ));
    lines.push(Line::styled(format!("{body_prefix} body:"), body_style));
    if body.is_empty() {
        lines.push(Line::from("    "));
    } else {
        for line in body.split('\n').take(4) {
            lines.push(Line::from(format!("    {line}")));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "    Tab/Shift+Tab switch | Ctrl+J newline(body)",
    ));
    lines.push(Line::from("    Enter confirm | Esc cancel"));
    ("Commit Editor", lines)
}

fn build_stash_editor_lines(title: &str, scope: &StashScope) -> (&'static str, Vec<Line<'static>>) {
    let mut lines = Vec::new();
    lines.push(Line::styled(
        format!(">> title: {title}"),
        focused_panel_style(),
    ));
    let scope_text = match scope {
        StashScope::All => "all changes".to_string(),
        StashScope::SelectedPaths(paths) => format!("selected files ({})", paths.len()),
    };
    lines.push(Line::from(format!("   scope: {scope_text}")));
    lines.push(Line::from(""));
    lines.push(Line::from("   Enter confirm | Esc cancel"));
    ("Stash Editor", lines)
}

fn centered_rect(area: Rect, target_width: u16, target_height: u16) -> Rect {
    let max_width = area.width.saturating_sub(2).max(1);
    let max_height = area.height.saturating_sub(2).max(1);
    let width = target_width.min(max_width).max(20.min(area.width));
    let height = target_height.min(max_height).max(6.min(area.height));
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width, height)
}
