#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorMove {
    Left,
    Right,
    Home,
    End,
}

pub(crate) fn insert_char_at_cursor(text: &mut String, cursor: &mut usize, ch: char) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    text.insert(*cursor, ch);
    *cursor += ch.len_utf8();
}

pub(crate) fn backspace_at_cursor(text: &mut String, cursor: &mut usize) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    let Some(previous) = previous_char_boundary(text, *cursor) else {
        return;
    };
    text.drain(previous..*cursor);
    *cursor = previous;
}

pub(crate) fn move_cursor_in_text(text: &str, cursor: &mut usize, movement: CursorMove) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    *cursor = match movement {
        CursorMove::Left => previous_char_boundary(text, *cursor).unwrap_or(0),
        CursorMove::Right => next_char_boundary(text, *cursor).unwrap_or(text.len()),
        CursorMove::Home => 0,
        CursorMove::End => text.len(),
    };
}

fn clamp_to_char_boundary(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }
    if text.is_char_boundary(cursor) {
        return cursor;
    }
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
        .unwrap_or(0)
}

fn previous_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
}

fn next_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .find(|index| *index > cursor)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_respect_unicode_boundaries() {
        let mut text = "a你".to_string();
        let mut cursor = 2;

        insert_char_at_cursor(&mut text, &mut cursor, '好');
        assert_eq!(text, "a好你");
        assert_eq!(cursor, "a好".len());

        move_cursor_in_text(&text, &mut cursor, CursorMove::Left);
        assert_eq!(cursor, "a".len());

        backspace_at_cursor(&mut text, &mut cursor);
        assert_eq!(text, "好你");
        assert_eq!(cursor, 0);
    }
}
