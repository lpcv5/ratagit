use ratagit_core::{AppContext, CommandPaletteSection};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::modal::{ModalSpec, ModalTone, render_modal};
use crate::theme::{modal_muted_style, modal_selected_row_style, modal_text_style};

enum PaletteRow {
    Header(&'static str),
    Command {
        command_index: usize,
        key: &'static str,
        label: &'static str,
    },
}

pub(crate) fn render_command_palette_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    if !state.ui.command_palette.active {
        return;
    }

    render_modal(
        frame,
        area,
        ModalSpec::new("Commands", ModalTone::Info, 78, 22, 24, 12, 1).with_icon("?"),
        &[&[("j/k", "select"), ("Enter", "run"), ("Esc", "close")]],
        |frame, content| {
            frame.render_widget(
                Paragraph::new(palette_lines(state, content.height as usize)),
                content,
            );
        },
    );
}

fn palette_lines(state: &AppContext, visible_rows: usize) -> Vec<Line<'static>> {
    let rows = palette_rows(state);
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

    rows.into_iter()
        .skip(start)
        .take(visible_rows)
        .map(|row| match row {
            PaletteRow::Header(label) => Line::styled(label, modal_muted_style()),
            PaletteRow::Command {
                command_index,
                key,
                label,
            } => command_line(
                key,
                label,
                command_index == state.ui.command_palette.selected,
            ),
        })
        .collect()
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

fn command_line(key: &'static str, label: &'static str, selected: bool) -> Line<'static> {
    let style = if selected {
        modal_selected_row_style()
    } else {
        modal_text_style()
    };
    Line::from(vec![Span::styled(format!("  {key:<8} {label}"), style)])
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
