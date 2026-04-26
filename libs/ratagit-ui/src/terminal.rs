use ratagit_core::{AppState, CommitField, EditorKind, PanelFocus, StashScope};
use ratatui::backend::{Backend, TestBackend};
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};
use ratatui::{Frame, Terminal};
use unicode_width::UnicodeWidthStr;

use crate::frame::{TerminalBuffer, TerminalCursor, TerminalSize, buffer_to_text};
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
    render_terminal_buffer_with_cursor(state, size).0
}

pub fn render_terminal_buffer_with_cursor(
    state: &AppState,
    size: TerminalSize,
) -> (TerminalBuffer, Option<TerminalCursor>) {
    let backend = TestBackend::new(size.width.max(1) as u16, size.height.max(1) as u16);
    let mut terminal = Terminal::new(backend).expect("test terminal should initialize");
    terminal
        .draw(|frame| render_terminal(frame, state))
        .expect("terminal render should succeed");
    let cursor = if state.editor.is_active() {
        let position = terminal
            .backend_mut()
            .get_cursor_position()
            .expect("test backend should expose cursor position");
        Some(TerminalCursor {
            x: position.x,
            y: position.y,
        })
    } else {
        None
    };
    (terminal.backend().buffer().clone(), cursor)
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

    let Some(modal) = editor_modal_rect(editor, area) else {
        return;
    };
    let block = Block::default()
        .title(format!(" {} ", editor_modal_title(editor)))
        .borders(Borders::ALL)
        .border_style(focused_panel_style());
    let inner = block.inner(modal);
    let content = inset_rect(inner, 1, 0);

    frame.render_widget(Clear, modal);
    frame.render_widget(block, modal);

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
        } => render_commit_editor(
            frame,
            content,
            message,
            *message_cursor,
            body,
            *body_cursor,
            *active_field,
        ),
        EditorKind::Stash {
            title,
            title_cursor,
            scope,
        } => render_stash_editor(frame, content, title, *title_cursor, scope),
    }

    if let Some(cursor) = editor_cursor_position(state, area) {
        frame.set_cursor_position(Position::new(cursor.x, cursor.y));
    }
}

fn editor_modal_rect(editor: &EditorKind, area: Rect) -> Option<Rect> {
    let target_height = match editor {
        EditorKind::Commit { .. } => 15,
        EditorKind::Stash { .. } => 10,
    };
    let modal = centered_rect(area, 76, target_height);
    (modal.width >= 20 && modal.height >= 6).then_some(modal)
}

fn editor_modal_title(editor: &EditorKind) -> &'static str {
    match editor {
        EditorKind::Commit { .. } => "Commit Message",
        EditorKind::Stash { .. } => "Stash Message",
    }
}

fn render_commit_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    message: &str,
    _message_cursor: usize,
    body: &str,
    body_cursor: usize,
    active_field: CommitField,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(3),
            Constraint::Length(2),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new("Edit the commit subject and optional body."),
        rows[0],
    );
    render_input_block(
        frame,
        rows[1],
        "Subject",
        vec![Line::from(message.to_string())],
        active_field == CommitField::Message,
    );
    let body_lines = body_visible_lines(body, body_cursor, rows[2].height.saturating_sub(2));
    render_input_block(
        frame,
        rows[2],
        "Body",
        body_lines,
        active_field == CommitField::Body,
    );
    frame.render_widget(
        Paragraph::new("Tab/Shift+Tab field  |  Left/Right/Home/End cursor\nCtrl+J newline  |  Enter confirm  |  Esc cancel"),
        rows[3],
    );
}

fn render_stash_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    _title_cursor: usize,
    scope: &StashScope,
) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .split(area);
    frame.render_widget(Paragraph::new("Name the stash before saving."), rows[0]);
    render_input_block(
        frame,
        rows[1],
        "Title",
        vec![Line::from(title.to_string())],
        true,
    );
    let scope_text = match scope {
        StashScope::All => "all changes".to_string(),
        StashScope::SelectedPaths(paths) => format!("selected files ({})", paths.len()),
    };
    frame.render_widget(Paragraph::new(format!("Scope: {scope_text}")), rows[2]);
    frame.render_widget(
        Paragraph::new("Left/Right/Home/End cursor\nEnter confirm  |  Esc cancel"),
        rows[3],
    );
}

fn render_input_block(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    lines: Vec<Line<'static>>,
    active: bool,
) {
    let border_style = if active {
        focused_panel_style()
    } else {
        Style::default()
    };
    let content = if lines.is_empty() {
        vec![Line::from(" ")]
    } else {
        lines
    };
    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .border_style(border_style),
        ),
        area,
    );
}

fn body_visible_lines(body: &str, cursor: usize, visible_height: u16) -> Vec<Line<'static>> {
    let visible_height = visible_height.max(1) as usize;
    let lines = body_lines(body);
    let cursor_line = line_index_at_cursor(body, cursor);
    let start = cursor_line.saturating_add(1).saturating_sub(visible_height);
    lines
        .into_iter()
        .skip(start)
        .take(visible_height)
        .map(Line::from)
        .collect()
}

fn body_lines(body: &str) -> Vec<String> {
    if body.is_empty() {
        return vec![String::new()];
    }
    body.split('\n').map(str::to_string).collect()
}

fn editor_cursor_position(state: &AppState, area: Rect) -> Option<TerminalCursor> {
    let editor = state.editor.kind.as_ref()?;
    let modal = editor_modal_rect(editor, area)?;
    let content = inset_rect(Block::default().borders(Borders::ALL).inner(modal), 1, 0);
    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
        } => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Min(3),
                    Constraint::Length(2),
                ])
                .split(content);
            match active_field {
                CommitField::Message => {
                    single_line_cursor_position(rows[1], message, *message_cursor)
                }
                CommitField::Body => multiline_cursor_position(rows[2], body, *body_cursor),
            }
        }
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => {
            let rows = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Length(2),
                ])
                .split(content);
            single_line_cursor_position(rows[1], title, *title_cursor)
        }
    }
}

fn single_line_cursor_position(area: Rect, text: &str, cursor: usize) -> Option<TerminalCursor> {
    if area.width < 3 || area.height < 3 {
        return None;
    }
    let cursor = cursor.min(text.len());
    let before_cursor = text.get(..cursor).unwrap_or(text);
    let x = content_cursor_x(area, UnicodeWidthStr::width(before_cursor));
    Some(TerminalCursor {
        x,
        y: area.y.saturating_add(1),
    })
}

fn multiline_cursor_position(area: Rect, text: &str, cursor: usize) -> Option<TerminalCursor> {
    if area.width < 3 || area.height < 3 {
        return None;
    }
    let cursor = cursor.min(text.len());
    let visible_height = area.height.saturating_sub(2).max(1) as usize;
    let cursor_line = line_index_at_cursor(text, cursor);
    let start = cursor_line.saturating_add(1).saturating_sub(visible_height);
    let visible_line = cursor_line.saturating_sub(start);
    let before_cursor = text.get(..cursor).unwrap_or(text);
    let line_before_cursor = before_cursor.rsplit('\n').next().unwrap_or("");
    let x = content_cursor_x(area, UnicodeWidthStr::width(line_before_cursor));
    Some(TerminalCursor {
        x,
        y: area.y.saturating_add(1 + visible_line as u16),
    })
}

fn content_cursor_x(area: Rect, text_width: usize) -> u16 {
    let content_width = area.width.saturating_sub(2) as usize;
    let offset = text_width.min(content_width.saturating_sub(1)) as u16;
    area.x.saturating_add(1).saturating_add(offset)
}

fn line_index_at_cursor(text: &str, cursor: usize) -> usize {
    let cursor = cursor.min(text.len());
    text.get(..cursor)
        .unwrap_or(text)
        .chars()
        .filter(|ch| *ch == '\n')
        .count()
}

fn inset_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    let shrink_x = horizontal.saturating_mul(2).min(area.width);
    let shrink_y = vertical.saturating_mul(2).min(area.height);
    Rect::new(
        area.x.saturating_add(horizontal.min(area.width)),
        area.y.saturating_add(vertical.min(area.height)),
        area.width.saturating_sub(shrink_x),
        area.height.saturating_sub(shrink_y),
    )
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
