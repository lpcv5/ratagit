use ratatui::buffer::Buffer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalSize {
    pub width: usize,
    pub height: usize,
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
    let truncated = if text.len() > width {
        text.chars().take(width).collect::<String>()
    } else {
        text
    };
    format!("{truncated:width$}", width = width)
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
