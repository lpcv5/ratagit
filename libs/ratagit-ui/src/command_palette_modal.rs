use ratagit_core::{AppContext, CommandPaletteSection};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::Line;

use crate::modal::{ModalSpec, ModalTone, command_menu_line, render_menu_lines, render_modal};
use crate::theme::UiTheme;

enum PaletteRow {
    Header(&'static str),
    Command {
        command_index: usize,
        key: &'static str,
        label: &'static str,
    },
}

pub(crate) fn render_command_palette_modal(
    frame: &mut Frame<'_>,
    state: &AppContext,
    area: Rect,
    theme: &UiTheme,
) {
    if !state.ui.command_palette.active {
        return;
    }

    let hints: &[(&str, &str)] = if state.ui.command_palette.search_active {
        &[("arrows", "select"), ("Enter/Esc", "finish search")]
    } else {
        &[
            ("j/k ↑/↓", "select"),
            ("/", "search"),
            ("Enter", "run"),
            ("Esc", "close"),
        ]
    };
    render_modal(
        frame,
        area,
        ModalSpec::new("Commands", ModalTone::Info, 78, 22, 24, 12, 1).with_icon("?"),
        &[hints],
        |frame, content| {
            render_menu_lines(
                frame,
                content,
                palette_lines(state, content.height as usize, theme),
                theme,
            );
        },
        theme,
    );
}

fn palette_lines(state: &AppContext, visible_rows: usize, theme: &UiTheme) -> Vec<Line<'static>> {
    let filtering =
        state.ui.command_palette.search_active || !state.ui.command_palette.query.is_empty();
    let visible_rows = visible_rows.saturating_sub(usize::from(filtering));
    let rows = palette_rows(state);
    let no_matches = rows.is_empty();
    let selected_row = rows
        .iter()
        .position(|row| {
            matches!(
                row,
                PaletteRow::Command { command_index, .. }
                    if *command_index == state.ui.command_palette.selected
            )
        })
        .unwrap_or(0);
    let start = viewport_start(selected_row, rows.len(), visible_rows);

    let mut lines: Vec<_> = rows
        .into_iter()
        .skip(start)
        .take(visible_rows)
        .map(|row| match row {
            PaletteRow::Header(label) => Line::styled(label, theme.modal_muted_style()),
            PaletteRow::Command {
                command_index,
                key,
                label,
            } => command_menu_line(
                key,
                label,
                command_index == state.ui.command_palette.selected,
                theme,
            ),
        })
        .collect();
    if filtering {
        lines.insert(
            0,
            Line::styled(
                format!("Filter: {}", state.ui.command_palette.query),
                theme.modal_text_style(),
            ),
        );
    }
    if no_matches && visible_rows > 0 {
        lines.push(Line::styled(
            "No matching commands",
            theme.modal_muted_style(),
        ));
    }
    lines
}

fn palette_rows(state: &AppContext) -> Vec<PaletteRow> {
    let mut rows = Vec::new();
    let mut current_section = None;

    for (command_index, entry) in state.command_palette_entries().iter().enumerate() {
        if current_section != Some(entry.section) {
            current_section = Some(entry.section);
            rows.push(PaletteRow::Header(section_label(entry.section)));
        }
        rows.push(PaletteRow::Command {
            command_index,
            key: entry.key,
            label: entry.label,
        });
    }

    rows
}

fn section_label(section: CommandPaletteSection) -> &'static str {
    match section {
        CommandPaletteSection::Local => "Local commands",
        CommandPaletteSection::Global => "Global commands",
    }
}

fn viewport_start(selected_row: usize, row_count: usize, visible_rows: usize) -> usize {
    if visible_rows == 0 || row_count <= visible_rows {
        return 0;
    }

    selected_row
        .saturating_sub(visible_rows / 2)
        .min(row_count - visible_rows)
}
