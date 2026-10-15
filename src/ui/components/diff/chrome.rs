use super::{DiffStatus, DiffViewModel};
use crate::git::{patch::Patch, DiffLineKind};
use crate::ui::{components::molecules::card, theme::UiTheme};
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::Block,
};

fn status(model: &DiffViewModel<'_>, scroll: usize, end: usize) -> String {
    let suffix = match model.status {
        DiffStatus::Idle => "",
        DiffStatus::Loading => " | Loading diff...",
        DiffStatus::Applying if model.selection.is_line_mode() => " | Applying lines",
        DiffStatus::Applying => " | Applying hunks",
        DiffStatus::ReadOnly => " | Read only",
    };
    if model.selection.focused && model.ready() {
        let mode = if model.selection.is_line_mode() {
            "Line"
        } else {
            "Hunk"
        };
        let position = model.selection.position;
        let selected = if model.selection.is_line_mode() {
            model.selection.marked_lines.len()
        } else {
            model.selection.marked_hunks.len()
        };
        let range = if model.selection.range_active {
            " | Range"
        } else {
            ""
        };
        format!(
            "{mode} {}/{} | {selected} selected{range}{suffix}",
            position.current, position.total
        )
    } else {
        format!("Rows {}-{end} / {}{suffix}", scroll + 1, model.lines.len())
    }
}

pub(super) fn block(
    model: &DiffViewModel<'_>,
    theme: &UiTheme,
    scroll: usize,
    end: usize,
    width: u16,
) -> Block<'static> {
    let patch = model.patch;
    let file_count = patch.map(Patch::display_file_count).unwrap_or(0);
    let file = patch.and_then(|patch| {
        model
            .selection
            .hunk_cursor
            .filter(|_| model.selection.focused)
            .and_then(|id| patch.files.get(id.file))
            .or_else(|| (file_count == 1).then(|| &patch.files[0]))
    });
    let title = file
        .and_then(|file| file.new.path.as_ref().or(file.old.path.as_ref()))
        .map(|path| format!("Diff · {}", path.display()))
        .unwrap_or_else(|| {
            if file_count > 1 {
                format!("Diff · {file_count} files")
            } else {
                "Diff".into()
            }
        });
    let mut block = card(theme, &title, model.selection.focused).title_bottom(
        Line::from(format!(" {} ", status(model, scroll, end)))
            .style(Style::default().fg(theme.text_muted)),
    );
    let added = model
        .lines
        .iter()
        .filter(|line| matches!(line.kind, DiffLineKind::Added))
        .count();
    let removed = model
        .lines
        .iter()
        .filter(|line| matches!(line.kind, DiffLineKind::Removed))
        .count();
    let summary = Line::from(vec![
        Span::styled(format!(" +{added} "), Style::default().fg(theme.success)),
        Span::styled(format!("-{removed} "), Style::default().fg(theme.error)),
    ]);
    if (added > 0 || removed > 0)
        && Line::raw(&title).width() + summary.width() + 6 <= usize::from(width)
    {
        block = block.title_top(summary.right_aligned());
    }
    block
}
