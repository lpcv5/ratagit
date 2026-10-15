use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::{RowRole, current_theme};
use ratagit_core::{
    AppContext,
    partial::{SelectionMode, cursor_ids, groups, marked_ids, marked_indices, patch_rows},
    patch::{PatchLineKind, PatchSource},
};
use ratatui::style::{Color, Modifier, Style};

pub(crate) fn render(state: &AppContext, max_lines: usize, width: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }
    let ui = &state.ui.details.patch;
    let patch = state.repo.details.patch.as_deref();
    let rows = patch.map(patch_rows).unwrap_or_default();
    let total = patch
        .map(|patch| groups(patch, ui.mode, ui.staged).len())
        .unwrap_or(0);
    let mode = if ui.mode == SelectionMode::Line {
        "Line"
    } else {
        "Hunk"
    };
    let added = rows
        .iter()
        .filter(|r| r.kind == PatchLineKind::Added)
        .count();
    let removed = rows
        .iter()
        .filter(|r| r.kind == PatchLineKind::Removed)
        .count();
    let mut lines = vec![PanelLine::new(
        format!(
            "{mode} {}/{} | {} selected{} | {} | +{added} -{removed}",
            if total == 0 {
                0
            } else {
                ui.selected[usize::from(ui.staged)] + 1
            },
            total,
            marked_indices(state).len(),
            if ui.anchor.is_some() { " | Range" } else { "" },
            if ui.staged { "staged" } else { "unstaged" }
        ),
        RowRole::Notice,
    )];
    if state.work.details.patch_request.is_some() {
        lines[0] = PanelLine::new("Loading reviewed patch...", RowRole::Notice);
    }
    if state.work.mutation.operation_pending.is_some() {
        lines[0] = PanelLine::new(
            format!("Applying selected {mode} changes..."),
            RowRole::Notice,
        );
    }
    lines.push(PanelLine::new(
        format!(
            "j/k:move a:mode v:range x:mark Tab:side Space:apply d:{} Esc:clear/back",
            if ui.staged { "unstage" } else { "discard" }
        ),
        RowRole::Muted,
    ));
    if state.ui.last_left_focus == ratagit_core::PanelFocus::Stash {
        lines[0] = PanelLine::new(
            "Stash diff (read only) | j/k:select Esc:back",
            RowRole::Notice,
        );
    }
    if let Some(error) = &state.repo.details.files_error {
        lines.push(PanelLine::new(error, RowRole::Error));
    }
    if let Some(patch) = patch {
        let ready = state.ui.focus == ratagit_core::PanelFocus::Details
            && state.work.details.patch_request.is_none()
            && state.work.mutation.operation_pending.is_none();
        let cursor: std::collections::HashSet<_> = cursor_ids(state).into_iter().collect();
        let marked: std::collections::HashSet<_> = marked_ids(state).into_iter().collect();
        let number_width = rows
            .iter()
            .flat_map(|r| [r.old_line, r.new_line])
            .flatten()
            .max()
            .unwrap_or(1)
            .max(1)
            .ilog10() as usize
            + 1;
        let show_numbers = width >= 2 * number_width + 24;
        let first_cursor = rows
            .iter()
            .find_map(|r| r.id.filter(|id| cursor.contains(id)));
        if rows.is_empty() {
            lines.push(PanelLine::new("No remaining changes", RowRole::Muted));
        }
        lines.extend(
            rows.into_iter()
                .skip(state.ui.details.scroll_offset)
                .take(max_lines.saturating_sub(lines.len()))
                .map(|row| {
                    let role = match row.kind {
                        PatchLineKind::Added => RowRole::DiffAdd,
                        PatchLineKind::Removed => RowRole::DiffRemove,
                        _ if row.id.is_none() => RowRole::DiffMeta,
                        _ => RowRole::Normal,
                    };
                    let focused = ready && row.id.is_some_and(|id| cursor.contains(&id));
                    let chosen = ready && row.id.is_some_and(|id| marked.contains(&id));
                    let staged = row
                        .id
                        .is_some_and(|id| patch.files[id.file].source == PatchSource::Index);
                    let highlight = ui.mode == SelectionMode::Line
                        || matches!(row.kind, PatchLineKind::Added | PatchLineKind::Removed);
                    let style =
                        diff_style(row.kind, staged, focused && highlight, chosen && highlight);
                    let marker =
                        focused && (ui.mode == SelectionMode::Line || row.id == first_cursor);
                    let mut spans = vec![PanelSpan {
                        text: format!(
                            "{}{} ",
                            if marker { ">" } else { " " },
                            if chosen && highlight { "┃" } else { " " }
                        ),
                        style: Style::default()
                            .fg(current_theme().accent)
                            .add_modifier(Modifier::BOLD),
                    }];
                    if show_numbers {
                        let number = |n: Option<u32>| {
                            n.map(|n| format!("{n:>number_width$}"))
                                .unwrap_or_else(|| " ".repeat(number_width))
                        };
                        spans.push(PanelSpan {
                            text: format!("{} {} │ ", number(row.old_line), number(row.new_line)),
                            style: Style::default().fg(current_theme().muted),
                        });
                    }
                    let prefix = match row.kind {
                        PatchLineKind::Added => "+ ",
                        PatchLineKind::Removed => "- ",
                        _ => "  ",
                    };
                    spans.push(PanelSpan {
                        text: format!("{prefix}{}", row.text),
                        style: if marker || row.text.starts_with("@@") {
                            style.add_modifier(Modifier::BOLD)
                        } else {
                            style
                        },
                    });
                    PanelLine::from_spans(spans, role).styled(style)
                }),
        );
    }
    lines.truncate(max_lines);
    lines
}

// PR #8's diff palette preserves add/remove polarity through cursor and marks.
fn diff_style(kind: PatchLineKind, staged: bool, cursor: bool, selected: bool) -> Style {
    let (foreground, background) = match (kind, staged, cursor, selected) {
        (PatchLineKind::Added, true, true, _) => (Color::Green, Color::Rgb(52, 100, 70)),
        (PatchLineKind::Added, true, false, true) => (Color::Green, Color::Rgb(44, 85, 60)),
        (PatchLineKind::Added, true, false, false) => (Color::Green, Color::Rgb(33, 67, 48)),
        (PatchLineKind::Added, false, true, _) => (Color::Green, Color::Rgb(40, 80, 58)),
        (PatchLineKind::Added, false, false, true) => (Color::Green, Color::Rgb(30, 65, 47)),
        (PatchLineKind::Added, false, false, false) => (Color::Green, Color::Rgb(24, 46, 36)),
        (PatchLineKind::Removed, true, true, _) => (Color::Red, Color::Rgb(111, 52, 64)),
        (PatchLineKind::Removed, true, false, true) => (Color::Red, Color::Rgb(94, 44, 55)),
        (PatchLineKind::Removed, true, false, false) => (Color::Red, Color::Rgb(73, 36, 45)),
        (PatchLineKind::Removed, false, true, _) => (Color::Red, Color::Rgb(88, 44, 54)),
        (PatchLineKind::Removed, false, false, true) => (Color::Red, Color::Rgb(72, 38, 46)),
        (PatchLineKind::Removed, false, false, false) => (Color::Red, Color::Rgb(51, 30, 37)),
        (_, _, true, _) => (Color::Reset, Color::Rgb(37, 57, 82)),
        (_, _, false, true) => (Color::Reset, Color::Rgb(46, 58, 86)),
        _ => return Style::default(),
    };
    Style::default().fg(foreground).bg(background)
}
