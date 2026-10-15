use crate::ui::components::molecules::{render_modal, shortcut_hint};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph, Wrap},
    Frame,
};

/// Content-sized overlays keep a gutter on normal terminals and fill tiny ones.
pub fn dialog_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(
        area.width
            .saturating_sub(if area.width >= 40 { 4 } else { 0 }),
    );
    let height = height.min(
        area.height
            .saturating_sub(if area.height >= 12 { 2 } else { 0 }),
    );
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

pub fn render_overlay_chrome(frame: &mut Frame, area: Rect, title: &str, theme: &UiTheme) -> Rect {
    // A terminal scrim preserves context while giving the modal visual priority.
    for cell in &mut frame.buffer_mut().content {
        cell.set_style(
            Style::default()
                .fg(theme.text_muted)
                .bg(theme.canvas)
                .add_modifier(Modifier::DIM),
        );
    }
    let shadow = Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width,
        area.height,
    )
    .intersection(frame.area());
    frame.render_widget(
        Block::default().style(Style::default().bg(theme.canvas)),
        shadow,
    );
    let inner = render_modal(frame, area, title, theme);
    inner.inner(Margin::new(
        u16::from(area.width >= 40),
        u16::from(area.height >= 10),
    ))
}

pub fn text_height(text: &str, width: u16) -> u16 {
    let width = usize::from(width.max(1));
    text.split('\n')
        .map(|line| {
            let mut rows = 1usize;
            let mut used = 0;
            for word in line.split_whitespace() {
                let cells = Line::from(word).width();
                let gap = usize::from(used > 0);
                if used > 0 && used + gap + cells > width {
                    rows += 1;
                    used = 0;
                } else {
                    used += gap;
                }
                rows += cells.saturating_sub(1) / width;
                used += if cells > width {
                    (cells - 1) % width + 1
                } else {
                    cells
                };
            }
            rows
        })
        .sum::<usize>()
        .min(u16::MAX as usize) as u16
}

/// Whole actions move to the next row on narrow terminals.
pub fn action_lines(hints: &[(String, String)], width: u16, theme: &UiTheme) -> Vec<Line<'static>> {
    let mut rows = Vec::new();
    let mut spans = Vec::new();
    let mut used = 0;
    for (index, (key, label)) in hints.iter().enumerate() {
        let action = shortcut_hint(key, label, theme);
        let len = action.iter().map(Span::width).sum::<usize>();
        if used > 0 && used + 3 + len > usize::from(width) {
            rows.push(Line::from(std::mem::take(&mut spans)));
            used = 0;
        }
        if used > 0 {
            spans.push(Span::raw("   "));
            used += 3;
        }
        if index + 1 == hints.len() {
            spans.extend(action.into_iter().map(|span| {
                span.style(
                    Style::default()
                        .fg(theme.canvas)
                        .bg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                )
            }));
        } else {
            spans.extend(action);
        }
        used += len;
    }
    if !spans.is_empty() {
        rows.push(Line::from(spans));
    }
    rows
}

/// Reserve the footer before the body so actions survive a short viewport.
pub fn dialog_sections(inner: Rect, footer_height: u16) -> (Rect, Rect) {
    let height = footer_height.min(inner.height);
    let body = Rect {
        height: inner.height.saturating_sub(height + 1),
        ..inner
    };
    let footer = Rect::new(inner.x, inner.bottom() - height, inner.width, height);
    (body, footer)
}

pub fn render_confirmation(
    frame: &mut Frame,
    title: &str,
    summary: &str,
    detail: &str,
    action: &str,
) {
    let theme = UiTheme::default();
    let area = dialog_rect(frame.area(), 72, frame.area().height);
    let width = area.width.saturating_sub(4);
    let hints = vec![
        ("N/Esc".into(), "Cancel".into()),
        ("Y/Enter".into(), action.into()),
    ];
    let actions = action_lines(&hints, width, &theme);
    let height =
        (text_height(summary, width) + text_height(detail, width) + actions.len() as u16 + 6)
            .max(10);
    let area = dialog_rect(frame.area(), 72, height);
    let inner = render_overlay_chrome(frame, area, title, &theme);
    let (body, footer) = dialog_sections(inner, actions.len() as u16);
    let lines = vec![
        Line::styled(
            summary.to_owned(),
            Style::default()
                .fg(theme.warning)
                .add_modifier(Modifier::BOLD),
        ),
        Line::default(),
        Line::styled(detail.to_owned(), Style::default().fg(theme.text_muted)),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
    frame.render_widget(Paragraph::new(actions), footer);
}
