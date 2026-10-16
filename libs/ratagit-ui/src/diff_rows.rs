//! Shared presentation for passive previews and reviewed, interactive patches.
use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::{RowRole, current_theme};
use ratagit_core::patch::{PatchLineKind, PatchRow};
use ratatui::style::{Color, Modifier, Style};

// Explicit RGB text keeps contrast independent of the terminal's ANSI palette.
const ADD_FOREGROUND: Color = Color::Rgb(174, 239, 190);
const REMOVE_FOREGROUND: Color = Color::Rgb(255, 190, 198);
const CONTEXT_FOREGROUND: Color = Color::Rgb(224, 230, 240);

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

pub(super) fn render(rows: &[DiffRow], offset: usize, max_lines: usize) -> Vec<PanelLine> {
    let offset = offset.min(rows.len().saturating_sub(max_lines));
    rows.iter()
        .skip(offset)
        .take(max_lines)
        .map(render_row)
        .collect()
}

fn render_row(display: &DiffRow) -> PanelLine {
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
        Style::default().fg(current_theme().info)
    } else {
        diff_style(row.kind, display.staged, display.focused, display.chosen)
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
                    style.fg(CONTEXT_FOREGROUND)
                } else {
                    Style::default().fg(current_theme().accent)
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

// PR #8's diff palette preserves add/remove polarity through cursor and marks.
fn diff_style(kind: PatchLineKind, staged: bool, cursor: bool, selected: bool) -> Style {
    let (foreground, background) = match (kind, staged, cursor, selected) {
        (PatchLineKind::Added, true, true, _) => (ADD_FOREGROUND, Color::Rgb(52, 100, 70)),
        (PatchLineKind::Added, true, false, true) => (ADD_FOREGROUND, Color::Rgb(44, 85, 60)),
        (PatchLineKind::Added, true, false, false) => (ADD_FOREGROUND, Color::Rgb(33, 67, 48)),
        (PatchLineKind::Added, false, true, _) => (ADD_FOREGROUND, Color::Rgb(40, 80, 58)),
        (PatchLineKind::Added, false, false, true) => (ADD_FOREGROUND, Color::Rgb(30, 65, 47)),
        (PatchLineKind::Added, false, false, false) => (ADD_FOREGROUND, Color::Rgb(24, 46, 36)),
        (PatchLineKind::Removed, true, true, _) => (REMOVE_FOREGROUND, Color::Rgb(111, 52, 64)),
        (PatchLineKind::Removed, true, false, true) => (REMOVE_FOREGROUND, Color::Rgb(94, 44, 55)),
        (PatchLineKind::Removed, true, false, false) => (REMOVE_FOREGROUND, Color::Rgb(73, 36, 45)),
        (PatchLineKind::Removed, false, true, _) => (REMOVE_FOREGROUND, Color::Rgb(88, 44, 54)),
        (PatchLineKind::Removed, false, false, true) => (REMOVE_FOREGROUND, Color::Rgb(72, 38, 46)),
        (PatchLineKind::Removed, false, false, false) => {
            (REMOVE_FOREGROUND, Color::Rgb(51, 30, 37))
        }
        (_, _, true, _) => (CONTEXT_FOREGROUND, Color::Rgb(37, 57, 82)),
        (_, _, false, true) => (CONTEXT_FOREGROUND, Color::Rgb(46, 58, 86)),
        _ => return Style::default(),
    };
    Style::default().fg(foreground).bg(background)
}

#[cfg(test)]
#[path = "diff_rows_tests.rs"]
mod tests;
