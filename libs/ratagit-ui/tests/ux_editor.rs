use ratagit_core::{
    Action, AppContext, CommitEditorIntent, CommitField, EditorKind, GitResult, StashScope,
    UiAction, update,
};
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::{
    TerminalBuffer, TerminalCursor, TerminalSize, render_terminal_buffer_with_cursor,
};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

fn commit_editor(subject: &str, body: &str, active_field: CommitField) -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    state.work.clear_details_pending();
    state.ui.editor.kind = Some(EditorKind::Commit {
        message: subject.to_string(),
        message_cursor: subject.len(),
        body: body.to_string(),
        body_cursor: body.len(),
        active_field,
        intent: CommitEditorIntent::Create,
    });
    state
}

fn stash_editor(title: &str) -> AppContext {
    let mut state = commit_editor("", "", CommitField::Message);
    state.ui.editor.kind = Some(EditorKind::Stash {
        title: title.to_string(),
        title_cursor: title.len(),
        scope: StashScope::All,
    });
    state
}

fn field_interior(buffer: &TerminalBuffer, title: &str) -> Rect {
    for y in 0..buffer.area.height {
        let row: String = (0..buffer.area.width)
            .map(|x| buffer[(x, y)].symbol())
            .collect();
        if !row.contains(&format!(" {title} ")) {
            continue;
        }
        let title_x = (0..buffer.area.width)
            .find(|&x| {
                title.chars().enumerate().all(|(index, ch)| {
                    let x = x as usize + index;
                    x < buffer.area.width as usize
                        && buffer[(x as u16, y)].symbol() == ch.to_string()
                })
            })
            .unwrap();
        let left = (0..title_x)
            .rfind(|&x| buffer[(x, y)].symbol() == "╭")
            .unwrap();
        let right = (title_x..buffer.area.width)
            .find(|&x| buffer[(x, y)].symbol() == "╮")
            .unwrap();
        let bottom = (y + 1..buffer.area.height)
            .find(|&y| buffer[(left, y)].symbol() == "╰")
            .unwrap();
        return Rect::new(left + 1, y + 1, right - left - 1, bottom - y - 1);
    }
    panic!("missing editor field {title}");
}

fn field_rows(buffer: &TerminalBuffer, area: Rect) -> Vec<String> {
    (area.y..area.bottom())
        .map(|y| {
            (area.x..area.right())
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

fn assert_cursor_in_field(
    buffer: &TerminalBuffer,
    cursor: Option<TerminalCursor>,
    title: &str,
) -> (Rect, TerminalCursor) {
    let area = field_interior(buffer, title);
    let cursor = cursor.expect("active editor field has a visible cursor");
    assert!(
        cursor.x >= area.x && cursor.x < area.right(),
        "cursor {cursor:?}, field {area:?}"
    );
    assert!(
        cursor.y >= area.y && cursor.y < area.bottom(),
        "cursor {cursor:?}, field {area:?}"
    );
    (area, cursor)
}

fn screen(buffer: &TerminalBuffer) -> String {
    buffer
        .content()
        .chunks(buffer.area.width as usize)
        .map(|row| row.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn long_subject_and_stash_title_follow_home_end_without_mutating_source() {
    let text = format!("feat: {}tail", "0123456789".repeat(12));
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    for (mut state, field) in [
        (commit_editor(&text, "", CommitField::Message), "Subject"),
        (stash_editor(&text), "Title"),
    ] {
        let before = state.clone();
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, cursor) = assert_cursor_in_field(&buffer, cursor, field);
        assert_eq!(area.height, 1);
        assert_eq!(cursor.x, area.right() - 1);
        assert!(field_rows(&buffer, area)[0].ends_with("tail "));
        assert_eq!(state, before, "render must not alter text or cursor state");
        let field_row = field_rows(&buffer, area)[0].clone();
        match field {
            "Subject" => insta::assert_snapshot!("long_subject_end", screen(&buffer)),
            _ => insta::assert_snapshot!("long_stash_title_end", screen(&buffer)),
        }
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorHome));
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, cursor) = assert_cursor_in_field(&buffer, cursor, field);
        assert_eq!(cursor.x, area.x);
        assert!(field_rows(&buffer, area)[0].starts_with("feat: 0123456789"));
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorEnd));
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, _) = assert_cursor_in_field(&buffer, cursor, field);
        assert_eq!(field_rows(&buffer, area)[0], field_row);
        assert_eq!(state, before);
    }
}

#[test]
fn long_body_scrolls_visual_rows_on_cursor_movement_and_resize() {
    let body = format!("{}FINISH", "0123456789".repeat(65));
    let mut state = commit_editor("feat: visual body viewport", &body, CommitField::Body);
    for size in [
        TerminalSize {
            width: 80,
            height: 24,
        },
        TerminalSize {
            width: 100,
            height: 30,
        },
        TerminalSize {
            width: 120,
            height: 40,
        },
        TerminalSize {
            width: 40,
            height: 14,
        },
    ] {
        let before = state.clone();
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, cursor) = assert_cursor_in_field(&buffer, cursor, "Body");
        let rows = field_rows(&buffer, area);
        let visual_row = body.len() / area.width as usize;
        assert_eq!(
            cursor.y,
            area.y + visual_row.min(area.height as usize - 1) as u16
        );
        assert!(
            rows[(cursor.y - area.y) as usize]
                .trim_end()
                .ends_with("FINISH")
        );
        assert_eq!(cursor.x - area.x, (body.len() % area.width as usize) as u16);
        assert_eq!(state, before);
        if size.width == 80 {
            insta::assert_snapshot!("long_body_end", screen(&buffer));
        }
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorHome));
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, cursor) = assert_cursor_in_field(&buffer, cursor, "Body");
        assert_eq!((cursor.x, cursor.y), (area.x, area.y));
        assert!(field_rows(&buffer, area)[0].starts_with("0123456789"));
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorEnd));
        assert_eq!(state, before);
    }
}

#[test]
fn body_preserves_spaces_and_cursor_matches_the_character_after_soft_wrap() {
    let size = TerminalSize {
        width: 80,
        height: 24,
    };
    let mut state = commit_editor("feat: preserve exact input", "", CommitField::Body);
    let (buffer, _) = render_terminal_buffer_with_cursor(&state, size);
    let area = field_interior(&buffer, "Body");
    let prefix = format!("{} ", "x".repeat(area.width as usize - 5));
    let body = format!("{prefix}abc def  ghi\n\n  next line\n");
    if let Some(EditorKind::Commit {
        body: text,
        body_cursor,
        ..
    }) = &mut state.ui.editor.kind
    {
        *text = body.clone();
        *body_cursor = prefix.len();
    }
    let before = state.clone();
    let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
    let (area, cursor) = assert_cursor_in_field(&buffer, cursor, "Body");
    let rows = field_rows(&buffer, area);
    assert_eq!(rows[0], format!("{prefix}abc "));
    assert!(rows[1].starts_with("def  ghi"));
    assert_eq!(buffer[(cursor.x, cursor.y)].symbol(), "a");
    assert_eq!(state, before);
    insta::assert_snapshot!("spaced_body_at_wrap", screen(&buffer));

    for _ in 0..4 {
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorRight));
    }
    let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
    let (area, cursor) = assert_cursor_in_field(&buffer, cursor, "Body");
    assert_eq!((cursor.x, cursor.y), (area.x, area.y + 1));
    assert_eq!(buffer[(cursor.x, cursor.y)].symbol(), "d");
}

#[test]
fn unicode_fields_use_byte_cursors_and_terminal_cell_widths() {
    let text = format!("{}甲乙丙丁", "界".repeat(42));
    for (mut state, field) in [
        (commit_editor(&text, "", CommitField::Message), "Subject"),
        (stash_editor(&text), "Title"),
        (commit_editor("subject", &text, CommitField::Body), "Body"),
    ] {
        for size in [
            TerminalSize {
                width: 80,
                height: 24,
            },
            TerminalSize {
                width: 100,
                height: 30,
            },
        ] {
            update(&mut state, Action::Ui(UiAction::EditorMoveCursorEnd));
            update(&mut state, Action::Ui(UiAction::EditorMoveCursorLeft));
            let before = state.clone();
            let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
            let (area, cursor) = assert_cursor_in_field(&buffer, cursor, field);
            assert_eq!(buffer[(cursor.x, cursor.y)].symbol(), "丁");
            assert_eq!((cursor.x - area.x) % 2, 0);
            assert_eq!(state, before);
            update(&mut state, Action::Ui(UiAction::EditorMoveCursorHome));
            let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
            let (area, cursor) = assert_cursor_in_field(&buffer, cursor, field);
            assert_eq!((cursor.x, cursor.y), (area.x, area.y));
            assert_eq!(buffer[(cursor.x, cursor.y)].symbol(), "界");
        }
    }
}

#[test]
fn full_body_row_end_and_explicit_newline_share_one_continuation_row() {
    let size = TerminalSize {
        width: 80,
        height: 24,
    };
    let mut state = commit_editor("subject", "", CommitField::Body);
    let (buffer, _) = render_terminal_buffer_with_cursor(&state, size);
    let area = field_interior(&buffer, "Body");
    let text = "x".repeat(area.width as usize);
    for body in [text.clone(), format!("{text}\n"), format!("{text}\nnext")] {
        state = commit_editor("subject", &body, CommitField::Body);
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, cursor) = assert_cursor_in_field(&buffer, cursor, "Body");
        let expected_column = if body.ends_with("next") { 4 } else { 0 };
        assert_eq!((cursor.x, cursor.y), (area.x + expected_column, area.y + 1));
        let rows = field_rows(&buffer, area);
        assert_eq!(rows[0], text);
        assert_eq!(
            rows[1].trim_end(),
            if expected_column == 0 { "" } else { "next" }
        );
    }
}

#[test]
fn snapshot_unicode_body_viewport() {
    let text = format!(
        "first logical line\n\n{}\n最后一行  trailing spaces  ",
        "中文  keep   every space 界 ".repeat(22)
    );
    let state = commit_editor(
        "feat: preserve spaces and wide characters",
        &text,
        CommitField::Body,
    );
    let (buffer, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 80,
            height: 24,
        },
    );
    assert_cursor_in_field(&buffer, cursor, "Body");
    insta::assert_snapshot!("unicode_body_viewport", screen(&buffer));
}

#[test]
fn filtered_controls_cannot_merge_separately_measured_editor_graphemes() {
    let text = "👨\t\u{200d}👩Z";
    let cursor_byte = text.find('Z').unwrap();
    for field in [CommitField::Message, CommitField::Body] {
        let mut state = commit_editor(text, text, field);
        let EditorKind::Commit {
            message_cursor,
            body_cursor,
            ..
        } = state.ui.editor.kind.as_mut().unwrap()
        else {
            unreachable!()
        };
        *message_cursor = cursor_byte;
        *body_cursor = cursor_byte;
        let (buffer, cursor) = render_terminal_buffer_with_cursor(
            &state,
            TerminalSize {
                width: 80,
                height: 24,
            },
        );
        let title = if field == CommitField::Body {
            "Body"
        } else {
            "Subject"
        };
        let (_, cursor) = assert_cursor_in_field(&buffer, cursor, title);
        assert_eq!(
            buffer[(cursor.x, cursor.y)].symbol(),
            "Z",
            "{title} cursor must sit at the displayed insertion position"
        );
    }
}

#[test]
fn editor_retains_border_focus_and_plain_text_styles() {
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    for (state, active_field, inactive_field) in [
        (
            commit_editor("subject", "body", CommitField::Message),
            "Subject",
            Some("Body"),
        ),
        (
            commit_editor("subject", "body", CommitField::Body),
            "Body",
            Some("Subject"),
        ),
        (stash_editor("title"), "Title", None),
    ] {
        let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, size);
        let (area, _) = assert_cursor_in_field(&buffer, cursor, active_field);
        let border = &buffer[(area.x - 1, area.y)];
        assert_eq!(border.fg, Color::Cyan);
        assert!(border.modifier.contains(Modifier::BOLD));
        let text = &buffer[(area.x, area.y)];
        assert_eq!(text.fg, Color::Reset);
        assert_eq!(text.bg, Color::Reset);
        assert!(!text.modifier.contains(Modifier::BOLD));
        if let Some(inactive_field) = inactive_field {
            let area = field_interior(&buffer, inactive_field);
            assert_eq!(buffer[(area.x - 1, area.y)].fg, Color::DarkGray);
        }
    }
}
