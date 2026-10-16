//! Shared presentation for passive previews and reviewed, interactive patches.
use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::{RowRole, UiTheme};
use ratagit_core::patch::{PatchLineKind, PatchRow};
use ratatui::style::{Modifier, Style};

pub(super) struct DiffRow {
    pub row: PatchRow,
    pub staged: bool,
    pub focused: bool,
    pub chosen: bool,
    pub marker: bool,
}

impl DiffRow {
    pub(super) fn preview(row: PatchRow, staged: bool) -> Self {
        Self {
            row,
            staged,
            focused: false,
            chosen: false,
            marker: false,
        }
    }
}

pub(super) fn render(
    rows: &[DiffRow],
    offset: usize,
    max_lines: usize,
    theme: &UiTheme,
) -> Vec<PanelLine> {
    let offset = offset.min(rows.len().saturating_sub(max_lines));
    rows.iter()
        .skip(offset)
        .take(max_lines)
        .map(|row| render_row(row, theme))
        .collect()
}

fn render_row(display: &DiffRow, theme: &UiTheme) -> PanelLine {
    let row = &display.row;
    let role = match row.kind {
        PatchLineKind::Added => RowRole::DiffAdd,
        PatchLineKind::Removed => RowRole::DiffRemove,
        _ => RowRole::Normal,
    };
    let is_hunk_header = row.kind == PatchLineKind::Context
        && row.old_line.is_none()
        && row.new_line.is_none()
        && row.text.starts_with("@@ ");
    let style = if is_hunk_header {
        Style::default().fg(theme.info)
    } else {
        diff_style(
            row.kind,
            display.staged,
            display.focused,
            display.chosen,
            theme,
        )
    };
    let prefix = match row.kind {
        PatchLineKind::Added => "+",
        PatchLineKind::Removed => "-",
        PatchLineKind::Context
            if row.id.is_some() || row.old_line.is_some() || row.new_line.is_some() =>
        {
            " "
        }
        _ => "",
    };
    PanelLine::from_spans(
        vec![
            PanelSpan {
                text: format!(
                    "{}{} ",
                    if display.marker { ">" } else { " " },
                    if display.chosen { "┃" } else { " " }
                ),
                style: if style.bg.is_some() {
                    style.fg(theme.diff.context_text)
                } else {
                    Style::default().fg(theme.diff.gutter)
                }
                .add_modifier(Modifier::BOLD),
            },
            PanelSpan {
                text: expand_tabs(&format!("{prefix}{}", row.text)),
                style,
            },
        ],
        role,
    )
    .styled(style)
}

// Git's diff text uses eight-column tab stops, measured after its +/- prefix.
fn expand_tabs(text: &str) -> String {
    use unicode_width::UnicodeWidthChar;
    let mut output = String::new();
    let mut column = 0;
    for ch in text.chars() {
        if ch == '\t' {
            let width = 8 - column % 8;
            output.extend(std::iter::repeat_n(' ', width));
            column += width;
        } else {
            output.push(ch);
            column += ch.width().unwrap_or(0);
        }
    }
    output
}

// Cursor wins over marks; worktree and index retain separate add/remove shades.
fn diff_style(
    kind: PatchLineKind,
    staged: bool,
    cursor: bool,
    selected: bool,
    theme: &UiTheme,
) -> Style {
    let state = if cursor { 2 } else { usize::from(selected) };
    let source = usize::from(staged);
    let (foreground, background) = match kind {
        PatchLineKind::Added => (theme.diff.added_text, theme.diff.added[source][state]),
        PatchLineKind::Removed => (theme.diff.removed_text, theme.diff.removed[source][state]),
        _ if cursor => (theme.diff.context_text, theme.diff.context_cursor),
        _ if selected => (theme.diff.context_text, theme.diff.context_marked),
        _ => return Style::default(),
    };
    Style::default().fg(foreground).bg(background)
}

#[cfg(test)]
#[path = "diff_rows_tests.rs"]
mod tests;
