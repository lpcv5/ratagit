use crate::app::SidePanel;
use crate::git::{DiffLine, DiffLineKind};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{List, ListItem},
    Frame,
};

pub struct DiffViewProps<'a> {
    pub lines: &'a [DiffLine],
    pub scroll: usize,
    pub active_panel: SidePanel,
    pub is_loading: bool,
}

pub fn render_diff_panel(frame: &mut Frame, area: Rect, props: DiffViewProps<'_>) {
    let theme = UiTheme::default();

    if props.is_loading {
        let panel_title = if props.active_panel == SidePanel::LocalBranches {
            "Log"
        } else {
            "Diff"
        };
        let paragraph = ratatui::widgets::Paragraph::new("Loading...")
            .style(Style::default().fg(theme.text_muted))
            .block(theme.panel_block(panel_title, true));
        frame.render_widget(paragraph, area);
        return;
    }

    if props.lines.is_empty() {
        let hint = match props.active_panel {
            SidePanel::Files => "Select a file to view diff",
            SidePanel::LocalBranches => "Select a branch to view log",
            SidePanel::Commits => "Select a commit/file to view diff",
            SidePanel::Stash => "Select a stash entry/file to view diff",
        };
        let panel_title = if props.active_panel == SidePanel::LocalBranches {
            "Log"
        } else {
            "Diff"
        };
        let paragraph = ratatui::widgets::Paragraph::new(hint)
            .style(Style::default().fg(theme.text_muted))
            .block(theme.panel_block(panel_title, true));
        frame.render_widget(paragraph, area);
        return;
    }

    let scroll = props.scroll.min(props.lines.len().saturating_sub(1));
    let visible_rows = usize::from(area.height.saturating_sub(2)).max(1);
    let items: Vec<ListItem> = props
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

    let total = props.lines.len();
    let end = (scroll + items.len()).min(total);
    let (panel_label, title) = if props.active_panel == SidePanel::LocalBranches {
        ("Log", format!("Log [{}-{} / {}]", scroll + 1, end, total))
    } else {
        ("Diff", format!("Diff [{}-{} / {}]", scroll + 1, end, total))
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
