use ratagit_core::{AppContext, CommitField, EditorKind, StashScope};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::frame::TerminalCursor;
use crate::modal::{
    ModalSpec, ModalTone, render_input_block, render_modal, render_section_label, render_text,
};
use crate::theme::UiTheme;

pub(crate) fn render_editor_modal(
    frame: &mut Frame<'_>,
    state: &AppContext,
    area: Rect,
    theme: &UiTheme,
) {
    let Some(editor) = &state.ui.editor.kind else {
        return;
    };

    let mut cursor = None;
    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => render_modal(
            frame,
            area,
            editor_modal_spec(editor),
            &[
                &[
                    ("Tab/Shift+Tab", "field"),
                    ("Left/Right/Home/End", "cursor"),
                ],
                &[
                    ("Ctrl+J", "newline"),
                    ("Enter", "confirm"),
                    ("Esc", "cancel"),
                ],
            ],
            |frame, content| {
                cursor = render_commit_editor(
                    frame,
                    content,
                    CommitEditorContent {
                        message,
                        message_cursor: *message_cursor,
                        body,
                        body_cursor: *body_cursor,
                        active_field: *active_field,
                    },
                    theme,
                );
            },
            theme,
        ),
        EditorKind::Stash {
            title,
            title_cursor,
            scope,
        } => render_modal(
            frame,
            area,
            editor_modal_spec(editor),
            &[
                &[("Left/Right/Home/End", "cursor")],
                &[("Enter", "confirm"), ("Esc", "cancel")],
            ],
            |frame, content| {
                cursor = render_stash_editor(frame, content, title, *title_cursor, scope, theme);
            },
            theme,
        ),
    };

    if let Some(cursor) = cursor {
        frame.set_cursor_position((cursor.x, cursor.y));
    }
}

fn editor_modal_spec(editor: &EditorKind) -> ModalSpec {
    let (target_height, min_height) = match editor {
        EditorKind::Commit { .. } => (15, 13),
        EditorKind::Stash { .. } => (11, 10),
    };
    ModalSpec::new(
        editor_modal_title(editor),
        ModalTone::Info,
        76,
        target_height,
        20,
        min_height,
        2,
    )
    .with_icon("✎")
}

fn editor_modal_title(editor: &EditorKind) -> &'static str {
    match editor {
        EditorKind::Commit { .. } => "Commit Message",
        EditorKind::Stash { .. } => "Stash Message",
    }
}

struct CommitEditorContent<'a> {
    message: &'a str,
    message_cursor: usize,
    body: &'a str,
    body_cursor: usize,
    active_field: CommitField,
}

fn render_commit_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    content: CommitEditorContent<'_>,
    theme: &UiTheme,
) -> Option<TerminalCursor> {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(3),
        ])
        .split(area);
    render_section_label(
        frame,
        rows[0],
        "Edit commit subject and optional body",
        theme,
    );
    let subject = single_line_viewport(content.message, content.message_cursor, rows[1]);
    let body = body_viewport(content.body, content.body_cursor, rows[2]);
    let cursor = match content.active_field {
        CommitField::Message => subject.cursor,
        CommitField::Body => body.cursor,
    };
    render_editor_input(
        frame,
        rows[1],
        "Subject",
        subject,
        content.active_field == CommitField::Message,
        theme,
    );
    render_editor_input(
        frame,
        rows[2],
        "Body",
        body,
        content.active_field == CommitField::Body,
        theme,
    );
    cursor
}

fn render_stash_editor(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &str,
    title_cursor: usize,
    scope: &StashScope,
    theme: &UiTheme,
) -> Option<TerminalCursor> {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(area);
    render_section_label(frame, rows[0], "Name the stash before saving", theme);
    let title = single_line_viewport(title, title_cursor, rows[1]);
    let cursor = title.cursor;
    render_editor_input(frame, rows[1], "Title", title, true, theme);
    let scope_text = match scope {
        StashScope::All => "all changes".to_string(),
        StashScope::SelectedPaths(paths) => format!("selected files ({})", paths.len()),
    };
    render_text(frame, rows[2], format!("Scope: {scope_text}"), theme);
    if rows.len() > 3 {
        frame.render_widget(Paragraph::new(""), rows[3]);
    }
    cursor
}

struct EditorViewport {
    lines: Vec<Line<'static>>,
    cursor: Option<TerminalCursor>,
}

fn render_editor_input(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    viewport: EditorViewport,
    active: bool,
    theme: &UiTheme,
) {
    render_input_block(
        frame,
        area,
        title,
        Vec::new(),
        active,
        ModalTone::Info,
        theme,
    );
    // The projection already wraps and scrolls in terminal cells. A second word
    // wrap would move spaces and words away from their computed cursor positions.
    frame.render_widget(
        Paragraph::new(viewport.lines).style(theme.modal_text_style()),
        input_interior(area),
    );
}

fn input_interior(area: Rect) -> Rect {
    Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(2),
    )
}

fn single_line_viewport(text: &str, cursor: usize, area: Rect) -> EditorViewport {
    let interior = input_interior(area);
    let width = interior.width as usize;
    let cursor = char_boundary(text, cursor);
    let graphemes = display_graphemes(text);
    let mut column = graphemes
        .iter()
        .take_while(|grapheme| grapheme.start < cursor)
        .map(|grapheme| {
            let prefix_len = cursor
                .saturating_sub(grapheme.start)
                .min(grapheme.symbol.len());
            UnicodeWidthStr::width(&grapheme.symbol[..prefix_len]).min(grapheme.width)
        })
        .sum::<usize>();
    let mut start = 0;
    // Leave an interior cell for the insertion cursor, as in workflow prompts.
    while column >= width.max(1) && start < graphemes.len() {
        column = column.saturating_sub(graphemes[start].width);
        start += 1;
    }
    let mut visible = Vec::new();
    let mut used = 0;
    for grapheme in &graphemes[start..] {
        if used + grapheme.width > width {
            break;
        }
        visible.push(Span::raw(grapheme.symbol.to_string()));
        used += grapheme.width;
    }
    EditorViewport {
        lines: vec![Line::from(visible)],
        cursor: (interior.width > 0 && interior.height > 0).then_some(TerminalCursor {
            x: interior.x.saturating_add(column as u16),
            y: interior.y,
        }),
    }
}

fn body_viewport(text: &str, cursor: usize, area: Rect) -> EditorViewport {
    let interior = input_interior(area);
    let width = (interior.width as usize).max(1);
    let height = (interior.height as usize).max(1);
    let cursor = char_boundary(text, cursor);
    let mut lines: Vec<Vec<Span<'static>>> = vec![Vec::new()];
    let mut column = 0;
    let mut cursor_row = 0;
    let mut cursor_column = 0;
    let mut line_start = 0;
    let mut logical_lines = text.split('\n').peekable();
    while let Some(line) = logical_lines.next() {
        let mut wrapped_at_end = false;
        let mut last_end = line_start;
        for grapheme in display_graphemes(line) {
            // A glyph wider than the entire field cannot be displayed there.
            // Use one replacement cell in that degenerate viewport only.
            let (symbol, glyph_width) = if grapheme.width > width {
                ("�", 1)
            } else {
                (grapheme.symbol, grapheme.width)
            };
            if column + glyph_width > width {
                lines.push(Vec::new());
                column = 0;
            }
            let start = line_start + grapheme.start;
            let end = start + grapheme.symbol.len();
            if (last_end..end).contains(&cursor) {
                let prefix_len = cursor.saturating_sub(start).min(grapheme.symbol.len());
                let prefix_width =
                    UnicodeWidthStr::width(&grapheme.symbol[..prefix_len]).min(glyph_width);
                cursor_row = lines.len() - 1 + (column + prefix_width) / width;
                cursor_column = (column + prefix_width) % width;
            }
            lines
                .last_mut()
                .expect("at least one visual row")
                .push(Span::raw(symbol.to_string()));
            column += glyph_width;
            wrapped_at_end = column == width;
            if wrapped_at_end {
                lines.push(Vec::new());
                column = 0;
            }
            last_end = end;
        }
        let line_end = line_start + line.len();
        if (last_end..=line_end).contains(&cursor) {
            cursor_row = lines.len() - 1;
            cursor_column = column;
        }
        // A full row already has its continuation row. An explicit newline
        // uses that row rather than introducing a second, phantom blank line.
        if logical_lines.peek().is_some() && !wrapped_at_end {
            lines.push(Vec::new());
            column = 0;
        }
        line_start = line_end.saturating_add(1);
    }
    let start = cursor_row.saturating_add(1).saturating_sub(height);
    EditorViewport {
        lines: lines
            .into_iter()
            .skip(start)
            .take(height)
            .map(Line::from)
            .collect(),
        cursor: (interior.width > 0 && interior.height > 0).then_some(TerminalCursor {
            x: interior.x.saturating_add(cursor_column as u16),
            y: interior.y.saturating_add((cursor_row - start) as u16),
        }),
    }
}

fn char_boundary(text: &str, cursor: usize) -> usize {
    let mut cursor = cursor.min(text.len());
    while !text.is_char_boundary(cursor) {
        cursor -= 1;
    }
    cursor
}

struct EditorGrapheme<'a> {
    symbol: &'a str,
    start: usize,
    width: usize,
}

fn display_graphemes(text: &str) -> Vec<EditorGrapheme<'_>> {
    let line = Line::from(text);
    let mut offset = 0;
    line.styled_graphemes(Style::default())
        .map(|grapheme| {
            // Ratatui filters control characters. Keep source byte positions
            // even when such characters occur between displayed graphemes.
            let start = offset + text[offset..].find(grapheme.symbol).unwrap_or(0);
            offset = start + grapheme.symbol.len();
            EditorGrapheme {
                symbol: &text[start..offset],
                start,
                width: UnicodeWidthStr::width(grapheme.symbol),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(text: &str, cursor: usize, width: u16, height: u16) -> (Vec<String>, (u16, u16)) {
        let viewport = body_viewport(text, cursor, Rect::new(0, 0, width + 2, height + 2));
        let cursor = viewport.cursor.expect("input has an interior");
        (
            viewport.lines.iter().map(ToString::to_string).collect(),
            (cursor.x - 1, cursor.y - 1),
        )
    }

    #[test]
    fn body_preserves_spaces_instead_of_rewrapping_words() {
        let (lines, cursor) = body("ab  cdef gh", 8, 5, 3);
        assert_eq!(lines, ["ab  c", "def g", "h"]);
        assert_eq!(cursor, (3, 1));
    }

    #[test]
    fn body_wraps_whole_wide_glyphs_and_maps_utf8_cursor_boundaries() {
        let text = "ab中文界";
        let (lines, cursor) = body(text, "ab中".len(), 5, 4);
        assert_eq!(lines, ["ab中", "文界"]);
        assert_eq!(cursor, (0, 1));
        assert_eq!(body(text, text.len(), 5, 4).1, (4, 1));
        // Invalid byte offsets are clamped to the same character boundary as core.
        assert_eq!(body(text, 3, 5, 4).1, (2, 0));
    }

    #[test]
    fn body_newlines_and_full_rows_do_not_introduce_phantom_blank_lines() {
        assert_eq!(
            body("abcd\ne", 6, 4, 4),
            (vec!["abcd".into(), "e".into()], (1, 1))
        );
        assert_eq!(
            body("abcd\n\ne", 7, 4, 4),
            (vec!["abcd".into(), "".into(), "e".into()], (1, 2))
        );
        assert_eq!(
            body("abcd", 4, 4, 4),
            (vec!["abcd".into(), "".into()], (0, 1))
        );
        assert_eq!(
            body("ab\n", 3, 4, 4),
            (vec!["ab".into(), "".into()], (0, 1))
        );
        assert_eq!(body("", 0, 4, 4), (vec!["".into()], (0, 0)));
    }

    #[test]
    fn body_cursor_at_soft_wrap_boundary_is_on_the_next_glyph() {
        let (lines, cursor) = body("abcdeZ", 5, 5, 3);
        assert_eq!(lines, ["abcde", "Z"]);
        assert_eq!(cursor, (0, 1));
    }

    #[test]
    fn body_retains_combining_sequences_and_emoji_graphemes() {
        let text = "e\u{301}👨‍👩‍👧‍👦中";
        let (lines, cursor) = body(text, "e\u{301}👨‍👩‍👧‍👦".len(), 4, 3);
        assert_eq!(lines, ["e\u{301}👨‍👩‍👧‍👦", "中"]);
        assert_eq!(cursor, (0, 1));
    }

    #[test]
    fn single_line_scrolls_at_grapheme_boundaries_and_uses_cell_width() {
        let text = "甲乙丙丁戊";
        let viewport = single_line_viewport(text, text.len(), Rect::new(0, 0, 7, 3));
        assert_eq!(viewport.lines[0].to_string(), "丁戊");
        assert_eq!(viewport.cursor, Some(TerminalCursor { x: 5, y: 1 }));
        let viewport = single_line_viewport(text, "甲乙".len(), Rect::new(0, 0, 7, 3));
        assert_eq!(viewport.lines[0].to_string(), "甲乙");
        assert_eq!(viewport.cursor, Some(TerminalCursor { x: 5, y: 1 }));
    }

    #[test]
    fn body_scrolls_visual_rows_and_recomputes_on_home_end_and_resize() {
        let text = "0123456789abcdefghijklmnop";
        assert_eq!(
            body(text, text.len(), 6, 2),
            (vec!["ijklmn".into(), "op".into()], (2, 1))
        );
        assert_eq!(
            body(text, 0, 6, 2),
            (vec!["012345".into(), "6789ab".into()], (0, 0))
        );
        assert_eq!(
            body(text, text.len(), 10, 2),
            (vec!["abcdefghij".into(), "klmnop".into()], (6, 1))
        );
    }

    #[test]
    fn inputs_without_an_interior_do_not_set_a_cursor() {
        for area in [
            Rect::new(0, 0, 2, 3),
            Rect::new(0, 0, 3, 2),
            Rect::default(),
        ] {
            assert_eq!(single_line_viewport("长标题", 6, area).cursor, None);
            assert_eq!(body_viewport("长正文", 6, area).cursor, None);
        }
    }
}
