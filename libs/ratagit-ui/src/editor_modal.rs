use ratagit_core::{AppState, CommitField, EditorKind, StashScope};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::frame::TerminalCursor;
use crate::theme::focused_panel_style;

pub(crate) fn render_editor_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
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
    let (visual_line, visual_column) =
        wrapped_line_cursor_offset(before_cursor, content_width(area));
    let x = content_cursor_x(area, visual_column);
    Some(TerminalCursor {
        x,
        y: area.y.saturating_add(1 + visual_line as u16),
    })
}

fn multiline_cursor_position(area: Rect, text: &str, cursor: usize) -> Option<TerminalCursor> {
    if area.width < 3 || area.height < 3 {
        return None;
    }
    let cursor = cursor.min(text.len());
    let visible_height = area.height.saturating_sub(2).max(1) as usize;
    let content_width = content_width(area);
    let lines = body_lines(text);
    let cursor_line = line_index_at_cursor(text, cursor);
    let start = cursor_line.saturating_add(1).saturating_sub(visible_height);
    let visible_line = lines
        .iter()
        .skip(start)
        .take(cursor_line.saturating_sub(start))
        .map(|line| wrapped_line_count(line, content_width))
        .sum::<usize>();
    let before_cursor = text.get(..cursor).unwrap_or(text);
    let line_before_cursor = before_cursor.rsplit('\n').next().unwrap_or("");
    let (wrapped_line, visual_column) =
        wrapped_line_cursor_offset(line_before_cursor, content_width);
    let x = content_cursor_x(area, visual_column);
    Some(TerminalCursor {
        x,
        y: area
            .y
            .saturating_add(1 + visible_line.saturating_add(wrapped_line) as u16),
    })
}

fn content_width(area: Rect) -> usize {
    area.width.saturating_sub(2).max(1) as usize
}

fn content_cursor_x(area: Rect, text_width: usize) -> u16 {
    let content_width = content_width(area);
    let offset = text_width.min(content_width.saturating_sub(1)) as u16;
    area.x.saturating_add(1).saturating_add(offset)
}

fn wrapped_line_cursor_offset(text: &str, content_width: usize) -> (usize, usize) {
    let content_width = content_width.max(1);
    let mut visual_line: usize = 0;
    let mut visual_column: usize = 0;

    for ch in text.chars() {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if visual_column > 0 && visual_column.saturating_add(char_width) > content_width {
            visual_line += 1;
            visual_column = 0;
        }
        visual_column = visual_column.saturating_add(char_width);
        if visual_column >= content_width {
            visual_line += 1;
            visual_column = 0;
        }
    }

    (visual_line, visual_column)
}

fn wrapped_line_count(text: &str, content_width: usize) -> usize {
    let content_width = content_width.max(1);
    UnicodeWidthStr::width(text)
        .saturating_add(content_width.saturating_sub(1))
        .checked_div(content_width)
        .unwrap_or(0)
        .max(1)
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
