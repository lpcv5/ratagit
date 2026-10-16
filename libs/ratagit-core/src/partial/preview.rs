//! Read-only coordinates from bounded Git output; never creates writable LineIds.
use crate::patch::{PatchLineKind, PatchRow};

pub struct PreviewRow {
    pub row: PatchRow,
    pub staged: bool,
}

pub fn preview_rows(text: &str) -> Vec<PreviewRow> {
    let mut staged = false;
    let mut coordinates = None;
    text.lines()
        .zip(text.lines().skip(1).chain(std::iter::once("")))
        .filter_map(|(line, next)| {
            let text = strip_ansi(line);
            if text.is_empty() && matches!(next, "### unstaged" | "### staged") {
                return None;
            }
            let mut row = PatchRow {
                text: text.clone(),
                id: None,
                kind: PatchLineKind::Context,
                old_line: None,
                new_line: None,
            };
            if matches!(text.as_str(), "### unstaged" | "### staged") {
                staged = text == "### staged";
                coordinates = None;
                return None;
            }
            if text.starts_with("### ") || text.starts_with("diff --git ") {
                coordinates = None;
            } else if text.starts_with("@@ ") {
                coordinates = hunk_start(&text);
            } else if let Some((old, new)) = coordinates.as_mut() {
                match text.as_bytes().first() {
                    Some(b'-') => {
                        row.kind = PatchLineKind::Removed;
                        row.old_line = Some(*old);
                        *old = old.saturating_add(1);
                    }
                    Some(b'+') => {
                        row.kind = PatchLineKind::Added;
                        row.new_line = Some(*new);
                        *new = new.saturating_add(1);
                    }
                    Some(b' ') => {
                        row.old_line = Some(*old);
                        row.new_line = Some(*new);
                        *old = old.saturating_add(1);
                        *new = new.saturating_add(1);
                    }
                    Some(b'\\') => row.kind = PatchLineKind::NoNewline,
                    _ => coordinates = None,
                }
                if row.old_line.is_some() || row.new_line.is_some() {
                    row.text = text[1..].to_string();
                }
            }
            Some(PreviewRow { row, staged })
        })
        .collect()
}

fn hunk_start(text: &str) -> Option<(u32, u32)> {
    let mut fields = text.strip_prefix("@@ ")?.split_whitespace();
    let start = |field: &str, prefix| field.strip_prefix(prefix)?.split(',').next()?.parse().ok();
    let old = start(fields.next()?, '-')?;
    let new = start(fields.next()?, '+')?;
    (fields.next()? == "@@").then_some((old, new))
}

fn strip_ansi(text: &str) -> String {
    let mut chars = text.chars().peekable();
    let mut plain = String::new();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
                    break;
                }
            }
        } else {
            plain.push(ch);
        }
    }
    plain
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_tracks_coordinates_without_writable_ids() {
        let rows = preview_rows(concat!(
            "### unstaged\n",
            "diff --git a/a b/a\n--- a/a\n+++ b/a\n",
            "\x1b[36m@@ -10,2 +20,2 @@ function\x1b[m\n",
            "\x1b[31m-old\x1b[m\n\\ No newline at end of file\n",
            "\x1b[32m+new\x1b[m\n context\n",
            "@@ -40 +50 @@\n-later\n+later\n",
            "### staged\n@@ -0,0 +1 @@\n+tracked\n",
            "diff --git a/b b/b\nBinary files a/b and b/b differ\n",
        ));
        let coordinates: Vec<_> = rows
            .iter()
            .filter(|r| r.row.old_line.is_some() || r.row.new_line.is_some())
            .map(|r| {
                (
                    r.row.text.as_str(),
                    r.row.old_line,
                    r.row.new_line,
                    r.staged,
                )
            })
            .collect();
        assert_eq!(
            coordinates,
            vec![
                ("old", Some(10), None, false),
                ("new", None, Some(20), false),
                ("context", Some(11), Some(21), false),
                ("later", Some(40), None, false),
                ("later", None, Some(50), false),
                ("tracked", None, Some(1), true),
            ]
        );
        assert!(rows.iter().all(|r| r.row.id.is_none()));
        assert!(rows.iter().all(|r| !r.row.text.contains('\x1b')));
        assert_eq!(rows[5].row.kind, PatchLineKind::NoNewline);
        assert!(rows.last().unwrap().row.old_line.is_none());
    }

    #[test]
    fn preview_preserves_metadata_and_resets_at_files_and_invalid_hunks() {
        let rows = preview_rows(
            "@@ -1 +1 @@\n-old\n+new\ndiff --git a/b b/b\n--- a/b\n+++ b/b\n@@ invalid @@\n+metadata\n@@ -1,0 +0,0 @@\n-deleted\n",
        );
        assert_eq!(rows[4].row.text, "--- a/b");
        assert_eq!(rows[5].row.text, "+++ b/b");
        assert!(rows[7].row.new_line.is_none());
        assert_eq!(rows[9].row.old_line, Some(1));
        assert_eq!(rows[9].row.new_line, None);
    }

    #[test]
    fn preview_hides_only_synthetic_sections_and_keeps_visible_scroll_count() {
        let text =
            "### unstaged\n@@ -1 +1 @@\n-old\n+new\n\n### staged\n@@ -1 +1 @@\n-before\n+after\n";
        let rows = preview_rows(text);
        assert_eq!(rows.len(), 6);
        assert!(rows.iter().take(3).all(|r| !r.staged));
        assert!(rows.iter().skip(3).all(|r| r.staged));
        let mut state = crate::AppContext::default();
        state.repo.details.files_diff = text.into();
        crate::details::scroll_down(&mut state, 100, 4);
        assert_eq!(state.ui.details.scroll_offset, 2);
    }
}
