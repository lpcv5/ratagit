use super::{ansi::parse_ansi_line, DiffViewModel};
use crate::git::{
    patch::{Patch, PatchRowMetadata, PatchSource},
    DiffLineKind,
};
use crate::ui::theme::UiTheme;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::ListItem,
};

pub(super) fn items(
    model: &DiffViewModel<'_>,
    theme: &UiTheme,
    scroll: usize,
    visible_rows: usize,
    width: u16,
) -> Vec<ListItem<'static>> {
    let rows = model.patch.map(Patch::row_metadata).unwrap_or_default();
    let marked_lines = model.selection.marked_lines;
    let marked_hunks = model.selection.marked_hunks;
    let changes = model.patch.map(Patch::change_blocks).unwrap_or_default();
    let patch = model.patch;
    // Keep the gutter stable when focus or selection mode changes. Hide line
    // numbers in small panes so there is still room to read the actual change.
    let number_width = patch.map(|_| line_number_width(&rows)).unwrap_or(0);
    let show_numbers = number_width > 0 && usize::from(width) >= 2 * number_width + 24;
    model
        .lines
        .iter()
        .enumerate()
        .skip(scroll)
        .take(visible_rows)
        .map(|(index, line)| {
            if patch.is_none()
                && matches!(line.kind, DiffLineKind::Context)
                && line.content.contains('\u{1b}')
            {
                return ListItem::new(parse_ansi_line(&line.content));
            }

            let metadata = rows.get(index).copied().unwrap_or_default();
            let line_id = metadata.id;
            let staged = line_id
                .and_then(|id| patch?.files.get(id.file))
                .is_some_and(|file| file.source == PatchSource::Index);
            let (mut style, prefix) = match line.kind {
                DiffLineKind::Added => (
                    Style::default().fg(theme.success).bg(if staged {
                        theme.diff_staged_added_bg
                    } else {
                        theme.diff_added_bg
                    }),
                    "+",
                ),
                DiffLineKind::Removed => (
                    Style::default().fg(theme.error).bg(if staged {
                        theme.diff_staged_removed_bg
                    } else {
                        theme.diff_removed_bg
                    }),
                    "-",
                ),
                DiffLineKind::Header => (
                    Style::default().fg(theme.accent).bg(theme.surface_raised),
                    " ",
                ),
                DiffLineKind::Context => (Style::default().fg(theme.text_primary), " "),
            };
            let change = changes.iter().find(|block| block.range.contains(&index));
            let cursor = model.ready()
                && model.selection.focused
                && if model.selection.is_line_mode() {
                    line_id.is_some() && line_id == model.selection.line_cursor
                } else {
                    change.is_some_and(|block| Some(block.id) == model.selection.hunk_cursor)
                };
            let selected = model.ready()
                && model.selection.focused
                && if model.selection.is_line_mode() {
                    line_id.is_some_and(|id| marked_lines.contains(&id))
                } else {
                    change.is_some_and(|block| marked_hunks.contains(&block.id))
                };
            let highlight = model.selection.is_line_mode()
                || matches!(line.kind, DiffLineKind::Added | DiffLineKind::Removed);
            if highlight && (cursor || selected) {
                style = style.bg(selection_background(theme, &line.kind, cursor, staged));
            }
            let hunk_header = line.kind == DiffLineKind::Header && line.content.starts_with("@@");
            let cursor_marker = cursor
                && (model.selection.is_line_mode()
                    || change.is_some_and(|block| block.range.start == index));
            let marker_style = Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD);
            let mut spans = vec![Span::styled(
                format!(
                    "{}{} ",
                    if cursor_marker { ">" } else { " " },
                    if selected && highlight { "┃" } else { " " }
                ),
                marker_style,
            )];
            if show_numbers {
                spans.push(Span::styled(
                    format!(
                        "{} {} │ ",
                        line_number(metadata.old_line, number_width),
                        line_number(metadata.new_line, number_width)
                    ),
                    Style::default().fg(if cursor && highlight {
                        theme.accent
                    } else {
                        theme.text_muted
                    }),
                ));
            }
            spans.push(Span::styled(format!("{prefix} "), style));
            let content = line.content.trim_end_matches(['\r', '\n']);
            spans.push(Span::styled(
                content.replace('\t', "    "),
                if hunk_header || cursor_marker {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                },
            ));
            ListItem::new(Line::from(spans)).style(style)
        })
        .collect()
}

fn line_number_width(rows: &[PatchRowMetadata]) -> usize {
    rows.iter()
        .flat_map(|row| [row.old_line, row.new_line].into_iter().flatten())
        .max()
        .unwrap_or(1)
        .max(1)
        .ilog10() as usize
        + 1
}

fn line_number(number: Option<u32>, width: usize) -> String {
    number
        .map(|number| format!("{number:>width$}"))
        .unwrap_or_else(|| " ".repeat(width))
}

fn selection_background(theme: &UiTheme, kind: &DiffLineKind, cursor: bool, staged: bool) -> Color {
    match (kind, cursor, staged) {
        (DiffLineKind::Added, true, true) => theme.diff_staged_added_cursor_bg,
        (DiffLineKind::Added, false, true) => theme.diff_staged_added_selected_bg,
        (DiffLineKind::Removed, true, true) => theme.diff_staged_removed_cursor_bg,
        (DiffLineKind::Removed, false, true) => theme.diff_staged_removed_selected_bg,
        (DiffLineKind::Added, true, false) => theme.diff_added_cursor_bg,
        (DiffLineKind::Added, false, false) => theme.diff_added_selected_bg,
        (DiffLineKind::Removed, true, false) => theme.diff_removed_cursor_bg,
        (DiffLineKind::Removed, false, false) => theme.diff_removed_selected_bg,
        (_, true, _) => theme.selection_bg,
        (_, false, _) => theme.visual_selection_bg,
    }
}
