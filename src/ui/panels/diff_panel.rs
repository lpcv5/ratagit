use crate::flux::git_backend::detail::{DetailPanelMode, DetailPanelViewState};
use crate::git::DiffLineKind;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

pub fn render_diff_panel(frame: &mut Frame, area: Rect, view: &DetailPanelViewState) {
    let theme = UiTheme::default();

    if view.is_loading {
        let paragraph = ratatui::widgets::Paragraph::new("Loading...")
            .style(Style::default().fg(theme.text_muted))
            .block(theme.panel_block(&view.panel_title, true));
        frame.render_widget(paragraph, area);
        return;
    }

    if view.lines.is_empty() {
        let paragraph = ratatui::widgets::Paragraph::new(view.empty_message.as_str())
            .style(Style::default().fg(theme.text_muted))
            .block(theme.panel_block(&view.panel_title, true));
        frame.render_widget(paragraph, area);
        return;
    }

    let scroll = view.scroll.min(view.lines.len().saturating_sub(1));
    let visible_rows = usize::from(area.height.saturating_sub(2)).max(1);
    let items: Vec<ListItem> = view
        .lines
        .iter()
        .skip(scroll)
        .take(visible_rows)
        .map(|line| {
            if matches!(line.kind, DiffLineKind::Context) && line.content.contains('\u{1b}') {
                return ListItem::new(parse_ansi_line(&line.content));
            }

            let (style, prefix) = match line.kind {
                DiffLineKind::Added => (Style::default().fg(Color::Green), "+"),
                DiffLineKind::Removed => (Style::default().fg(Color::Red), "-"),
                DiffLineKind::Header => (Style::default().fg(Color::Cyan), ""),
                DiffLineKind::Context => (Style::default().fg(Color::Gray), " "),
            };
            let text = Line::from(vec![Span::styled(
                format!("{}{}", prefix, line.content),
                style,
            )]);
            ListItem::new(text)
        })
        .collect();

    let total = view.lines.len();
    let end = (scroll + items.len()).min(total);
    let title = match view.mode {
        DetailPanelMode::Log => format!("Log [{}-{} / {}]", scroll + 1, end, total),
        DetailPanelMode::Diff => format!("Diff [{}-{} / {}]", scroll + 1, end, total),
    };
    let panel_label = match view.mode {
        DetailPanelMode::Log => "Log",
        DetailPanelMode::Diff => "Diff",
    };

    let list = List::new(items).block(theme.panel_block(panel_label, true).title(title));

    frame.render_widget(list, area);
}

fn parse_ansi_line(input: &str) -> Line<'static> {
    let mut spans = Vec::new();
    let mut chars = input.chars().peekable();
    let mut text = String::new();
    let mut style = Style::default();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            let _ = chars.next();
            if !text.is_empty() {
                spans.push(Span::styled(std::mem::take(&mut text), style));
            }

            let mut seq = String::new();
            while let Some(&c) = chars.peek() {
                let _ = chars.next();
                if c == 'm' {
                    break;
                }
                seq.push(c);
            }

            style = apply_sgr_codes(style, &seq);
            continue;
        }
        text.push(ch);
    }

    if !text.is_empty() {
        spans.push(Span::styled(text, style));
    }

    Line::from(spans)
}

fn apply_sgr_codes(mut style: Style, seq: &str) -> Style {
    let mut codes = seq
        .split(';')
        .filter_map(|part| part.parse::<u16>().ok())
        .peekable();

    if seq.is_empty() {
        return Style::default();
    }

    while let Some(code) = codes.next() {
        match code {
            0 => style = Style::default(),
            1 => style = style.add_modifier(ratatui::style::Modifier::BOLD),
            22 => style = style.remove_modifier(ratatui::style::Modifier::BOLD),
            30..=37 => style = style.fg(ansi_basic_color(code - 30, false)),
            90..=97 => style = style.fg(ansi_basic_color(code - 90, true)),
            39 => style = style.fg(Color::Reset),
            38 => {
                if let Some(mode) = codes.next() {
                    match mode {
                        5 => {
                            if let Some(idx) = codes.next() {
                                style = style.fg(Color::Indexed(idx as u8));
                            }
                        }
                        2 => {
                            let r = codes.next().unwrap_or(255) as u8;
                            let g = codes.next().unwrap_or(255) as u8;
                            let b = codes.next().unwrap_or(255) as u8;
                            style = style.fg(Color::Rgb(r, g, b));
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    style
}

fn ansi_basic_color(index: u16, bright: bool) -> Color {
    match (index, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::Reset,
    }
}
