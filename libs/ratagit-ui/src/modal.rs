use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::theme::{
    modal_danger_style, modal_footer_style, modal_info_style, modal_muted_style,
    modal_warning_style, selected_row_style,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModalTone {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModalSpec {
    pub(crate) title: &'static str,
    pub(crate) tone: ModalTone,
    pub(crate) target_width: u16,
    pub(crate) target_height: u16,
    pub(crate) min_width: u16,
    pub(crate) min_height: u16,
    pub(crate) footer_height: u16,
}

impl ModalSpec {
    pub(crate) fn new(
        title: &'static str,
        tone: ModalTone,
        target_width: u16,
        target_height: u16,
        min_width: u16,
        min_height: u16,
        footer_height: u16,
    ) -> Self {
        Self {
            title,
            tone,
            target_width,
            target_height,
            min_width,
            min_height,
            footer_height,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModalFrame {
    pub(crate) outer: Rect,
    pub(crate) content: Rect,
    pub(crate) footer: Option<Rect>,
    pub(crate) tone: ModalTone,
}

pub(crate) fn render_modal_frame(
    frame: &mut Frame<'_>,
    area: Rect,
    spec: ModalSpec,
) -> Option<ModalFrame> {
    let outer = modal_rect(area, spec)?;
    let tone_style = modal_tone_style(spec.tone);
    let block = Block::default()
        .title(Line::styled(format!(" {} ", spec.title), tone_style))
        .borders(Borders::ALL)
        .border_style(tone_style);
    let content_area = inset_rect(block.inner(outer), 1, 0);
    let (content, footer) = split_footer(content_area, spec.footer_height)?;

    frame.render_widget(Clear, outer);
    frame.render_widget(block, outer);

    Some(ModalFrame {
        outer,
        content,
        footer,
        tone: spec.tone,
    })
}

pub(crate) fn modal_content_rect(area: Rect, spec: ModalSpec) -> Option<Rect> {
    let outer = modal_rect(area, spec)?;
    let content_area = inset_rect(Block::default().borders(Borders::ALL).inner(outer), 1, 0);
    split_footer(content_area, spec.footer_height).map(|(content, _)| content)
}

pub(crate) fn render_section_label(frame: &mut Frame<'_>, area: Rect, label: impl Into<String>) {
    frame.render_widget(
        Paragraph::new(label.into()).style(modal_muted_style()),
        area,
    );
}

pub(crate) fn render_muted_text(frame: &mut Frame<'_>, area: Rect, text: impl Into<String>) {
    frame.render_widget(
        Paragraph::new(text.into())
            .style(modal_muted_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn render_warning_text(
    frame: &mut Frame<'_>,
    area: Rect,
    tone: ModalTone,
    text: impl Into<String>,
) {
    frame.render_widget(
        Paragraph::new(text.into())
            .style(modal_tone_style(tone))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn render_action_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    tone: ModalTone,
    actions: &[(&'static str, &'static str)],
) {
    let mut spans = Vec::new();
    for (index, (key, label)) in actions.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  |  ", modal_footer_style()));
        }
        spans.push(Span::styled(*key, modal_tone_style(tone)));
        spans.push(Span::styled(format!(" {label}"), modal_footer_style()));
    }
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

pub(crate) fn render_choice_list<T: Copy + PartialEq>(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    choices: &[(T, &'static str, Style)],
    selected: T,
    tone: ModalTone,
) {
    let items = choices
        .iter()
        .map(|(_, label, style)| ListItem::new(Line::styled(format!("  {label}"), *style)))
        .collect::<Vec<_>>();
    let selected_index = choices
        .iter()
        .position(|(choice, _, _)| *choice == selected)
        .unwrap_or(0);
    let mut list_state = ListState::default();
    list_state.select(Some(selected_index));
    let list = List::new(items)
        .highlight_style(selected_row_style())
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .title(Line::styled(format!(" {title} "), modal_tone_style(tone)))
                .borders(Borders::ALL)
                .border_style(modal_muted_style()),
        );
    frame.render_stateful_widget(list, area, &mut list_state);
}

pub(crate) fn render_input_block(
    frame: &mut Frame<'_>,
    area: Rect,
    title: &'static str,
    lines: Vec<Line<'static>>,
    active: bool,
    tone: ModalTone,
) {
    let border_style = if active {
        modal_tone_style(tone)
    } else {
        modal_muted_style()
    };
    let content = if lines.is_empty() {
        vec![Line::from(" ")]
    } else {
        lines
    };
    frame.render_widget(
        Paragraph::new(content).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(Line::styled(format!(" {title} "), border_style))
                .borders(Borders::ALL)
                .border_style(border_style),
        ),
        area,
    );
}

pub(crate) fn modal_tone_style(tone: ModalTone) -> Style {
    match tone {
        ModalTone::Info => modal_info_style(),
        ModalTone::Warning => modal_warning_style(),
        ModalTone::Danger => modal_danger_style(),
    }
}

fn split_footer(area: Rect, footer_height: u16) -> Option<(Rect, Option<Rect>)> {
    if footer_height == 0 {
        return Some((area, None));
    }
    if area.height <= footer_height {
        return None;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(footer_height)])
        .split(area);
    Some((rows[0], Some(rows[1])))
}

fn modal_rect(area: Rect, spec: ModalSpec) -> Option<Rect> {
    let max_width = area.width.saturating_sub(2).max(1);
    let max_height = area.height.saturating_sub(2).max(1);
    let width = spec
        .target_width
        .min(max_width)
        .max(spec.min_width.min(area.width));
    let height = spec
        .target_height
        .min(max_height)
        .max(spec.min_height.min(area.height));
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    (rect.width >= spec.min_width && rect.height >= spec.min_height).then_some(rect)
}

fn inset_rect(area: Rect, horizontal: u16, vertical: u16) -> Rect {
    let shrink_x = horizontal.saturating_mul(2).min(area.width);
    let shrink_y = vertical.saturating_mul(2).min(area.height);
    Rect::new(
        area.x.saturating_add(horizontal.min(area.width)),
        area.y.saturating_add(vertical.min(area.height)),
        area.width.saturating_sub(shrink_x),
        area.height.saturating_sub(shrink_y),
    )
}
