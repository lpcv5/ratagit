use ratatui::buffer::Buffer;
use ratatui::style::{Color, Style};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::theme::{batch_selected_row_style, selected_row_style};

pub type TerminalBuffer = Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub width: usize,
    pub height: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCursor {
    pub x: u16,
    pub y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RenderContext {
    pub spinner_frame: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedFrame {
    pub lines: Vec<String>,
}

impl RenderedFrame {
    pub fn as_text(&self) -> String {
        self.lines.join("\n")
    }
}

pub(crate) fn pad_and_truncate(text: String, width: usize) -> String {
    let truncated = truncate_to_width(&text, width);
    let used = UnicodeWidthStr::width(truncated.as_str());
    format!("{truncated}{}", " ".repeat(width.saturating_sub(used)))
}

pub(crate) fn normalize_lines(mut lines: Vec<String>, size: TerminalSize) -> RenderedFrame {
    for line in &mut lines {
        *line = pad_and_truncate(line.clone(), size.width);
    }

    if lines.len() > size.height {
        lines.truncate(size.height);
    } else {
        while lines.len() < size.height {
            lines.push(" ".repeat(size.width));
        }
    }

    RenderedFrame { lines }
}

pub(crate) fn buffer_to_text(buffer: &Buffer) -> String {
    let width = buffer.area.width as usize;
    buffer
        .content()
        .chunks(width)
        .map(|cells| cells.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn buffer_contains_selected_text(buffer: &TerminalBuffer, needle: &str) -> bool {
    buffer_contains_text_with_style(buffer, needle, selected_row_style())
}

pub fn buffer_contains_batch_selected_text(buffer: &TerminalBuffer, needle: &str) -> bool {
    buffer_contains_text_with_style(buffer, needle, batch_selected_row_style())
}

pub fn buffer_to_text_with_selected_marker(buffer: &TerminalBuffer) -> String {
    let width = buffer.area.width as usize;
    buffer
        .content()
        .chunks(width)
        .map(|cells| {
            let selected = cells
                .iter()
                .any(|cell| cell_matches_style(cell, selected_row_style()));
            let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
            if selected {
                format!(">{line}")
            } else {
                format!(" {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn buffer_contains_text_with_style(
    buffer: &TerminalBuffer,
    needle: &str,
    style: Style,
) -> bool {
    let width = buffer.area.width as usize;
    if width == 0 {
        return false;
    }

    buffer.content().chunks(width).any(|cells| {
        let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
        line.contains(needle) && cells.iter().any(|cell| cell_matches_style(cell, style))
    })
}

fn truncate_to_width(text: &str, width: usize) -> String {
    let mut output = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > width {
            break;
        }
        output.push(ch);
        used += ch_width;
    }
    output
}

fn cell_matches_style(cell: &ratatui::buffer::Cell, style: Style) -> bool {
    style.fg.is_none_or(|fg| color_matches(cell.fg, fg))
        && style.bg.is_none_or(|bg| color_matches(cell.bg, bg))
        && cell.modifier.contains(style.add_modifier)
}

fn color_matches(actual: Color, expected: Color) -> bool {
    actual == expected
}
