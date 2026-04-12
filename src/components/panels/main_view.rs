use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::Paragraph,
    Frame,
};

use crate::app::CachedData;

use crate::components::core::{panel_block, theme};
use crate::components::Component;
use crate::components::Intent;

/// 主视面板组件（持有自身滚动状态）
pub struct MainViewPanel {
    scroll: u16,
}

impl MainViewPanel {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn scroll_to(&mut self, offset: u16) {
        self.scroll = offset;
    }

    pub fn scroll_by_clamped(&mut self, delta: i16, max_scroll: u16) {
        let current = self.scroll.min(max_scroll);

        if delta.is_negative() {
            self.scroll = current.saturating_sub(delta.unsigned_abs());
        } else {
            self.scroll = current.saturating_add(delta as u16).min(max_scroll);
        }
    }
}

impl Default for MainViewPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for MainViewPanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
                KeyCode::Char('u') if key.modifiers == KeyModifiers::CONTROL => {
                    return Intent::ScrollMainView(-5)
                }
                KeyCode::Char('d') if key.modifiers == KeyModifiers::CONTROL => {
                    return Intent::ScrollMainView(5)
                }
                _ => {}
            }
        }

        Intent::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        let title = if let Some((path, _)) = &data.current_diff {
            format!("Main View · Diff · {path}")
        } else {
            "Main View".to_string()
        };

        let content = data
            .current_diff
            .as_ref()
            .map(|(_, diff)| diff.as_str())
            .unwrap_or("No file diff loaded. Select a file to see its diff here.");

        let block = panel_block(&title, is_focused);
        let paragraph = if contains_ansi_escape(content) {
            Paragraph::new(colorize_ansi_text(content))
        } else if is_diff_text(content) {
            Paragraph::new(colorize_diff_text(content))
        } else {
            Paragraph::new(content.to_string())
        }
        .block(block)
        .scroll((self.scroll, 0));

        frame.render_widget(paragraph, area);
    }
}

fn is_diff_text(content: &str) -> bool {
    content.lines().any(|line| {
        line.starts_with("diff --git ")
            || line.starts_with("index ")
            || line.starts_with("@@ ")
            || line.starts_with("+++ ")
            || line.starts_with("--- ")
    })
}

fn contains_ansi_escape(content: &str) -> bool {
    content.contains('\u{1b}')
}

fn colorize_diff_text(content: &str) -> Text<'static> {
    let lines: Vec<Line<'static>> = content.lines().map(colorize_diff_line).collect();
    Text::from(lines)
}

fn colorize_diff_line(line: &str) -> Line<'static> {
    let style = if line.starts_with("diff --git ") {
        Style::default().fg(theme().diff_header)
    } else if line.starts_with("index ")
        || line.starts_with("new file mode ")
        || line.starts_with("deleted file mode ")
        || line.starts_with("similarity index ")
        || line.starts_with("rename from ")
        || line.starts_with("rename to ")
    {
        Style::default().fg(theme().diff_meta)
    } else if line.starts_with("@@ ") {
        Style::default().fg(theme().diff_hunk)
    } else if line.starts_with("+++ ") || line.starts_with("--- ") {
        Style::default().fg(theme().diff_file)
    } else if line.starts_with('+') && !line.starts_with("+++") {
        Style::default().fg(theme().diff_added)
    } else if line.starts_with('-') && !line.starts_with("---") {
        Style::default().fg(theme().diff_removed)
    } else if line.starts_with("\\ No newline at end of file") {
        Style::default().fg(Color::Rgb(98, 114, 164)) // Dracula Comment
    } else {
        Style::default()
    };

    Line::from(Span::styled(line.to_string(), style))
}

fn colorize_ansi_text(content: &str) -> Text<'static> {
    let lines: Vec<Line<'static>> = content.lines().map(parse_ansi_line).collect();
    Text::from(lines)
}

fn parse_ansi_line(line: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut style = Style::default();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && matches!(chars.peek(), Some('[')) {
            let _ = chars.next();
            if !current.is_empty() {
                spans.push(Span::styled(current.clone(), style));
                current.clear();
            }

            let mut seq = String::new();
            let mut found_terminator = false;
            for code_ch in chars.by_ref() {
                if code_ch == 'm' {
                    found_terminator = true;
                    break;
                }
                seq.push(code_ch);
            }

            if found_terminator {
                apply_sgr_sequence(&seq, &mut style);
            } else {
                current.push('\u{1b}');
                current.push('[');
                current.push_str(&seq);
                break;
            }
        } else {
            current.push(ch);
        }
    }

    if !current.is_empty() {
        spans.push(Span::styled(current, style));
    }

    Line::from(spans)
}

fn apply_sgr_sequence(seq: &str, style: &mut Style) {
    if seq.is_empty() {
        *style = Style::default();
        return;
    }

    for part in seq.split(';') {
        let Ok(code) = part.parse::<u8>() else {
            continue;
        };

        match code {
            0 => *style = Style::default(),
            1 => *style = style.add_modifier(Modifier::BOLD),
            22 => *style = style.remove_modifier(Modifier::BOLD),
            30 => *style = style.fg(Color::Black),
            31 => *style = style.fg(Color::Red),
            32 => *style = style.fg(Color::Green),
            33 => *style = style.fg(Color::Yellow),
            34 => *style = style.fg(Color::Blue),
            35 => *style = style.fg(Color::Magenta),
            36 => *style = style.fg(Color::Cyan),
            37 => *style = style.fg(Color::Gray),
            39 => *style = style.fg(Color::Reset),
            40 => *style = style.bg(Color::Black),
            41 => *style = style.bg(Color::Red),
            42 => *style = style.bg(Color::Green),
            43 => *style = style.bg(Color::Yellow),
            44 => *style = style.bg(Color::Blue),
            45 => *style = style.bg(Color::Magenta),
            46 => *style = style.bg(Color::Cyan),
            47 => *style = style.bg(Color::Gray),
            49 => *style = style.bg(Color::Reset),
            90 => *style = style.fg(Color::DarkGray),
            91 => *style = style.fg(Color::LightRed),
            92 => *style = style.fg(Color::LightGreen),
            93 => *style = style.fg(Color::LightYellow),
            94 => *style = style.fg(Color::LightBlue),
            95 => *style = style.fg(Color::LightMagenta),
            96 => *style = style.fg(Color::LightCyan),
            97 => *style = style.fg(Color::White),
            100 => *style = style.bg(Color::DarkGray),
            101 => *style = style.bg(Color::LightRed),
            102 => *style = style.bg(Color::LightGreen),
            103 => *style = style.bg(Color::LightYellow),
            104 => *style = style.bg(Color::LightBlue),
            105 => *style = style.bg(Color::LightMagenta),
            106 => *style = style.bg(Color::LightCyan),
            107 => *style = style.bg(Color::White),
            _ => {}
        }
    }
}
