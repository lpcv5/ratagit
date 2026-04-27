use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::theme::{
    modal_active_input_style, modal_background_style, modal_border_style, modal_danger_style,
    modal_footer_style, modal_info_style, modal_muted_style, modal_scrim_style,
    modal_selected_row_style, modal_text_style, modal_warning_style,
};

pub(crate) type ModalAction = (&'static str, &'static str);

const DESIGN_MIN_WIDTH: u16 = 40;
const DESIGN_MAX_WIDTH: u16 = 72;
const DESIGN_TARGET_WIDTH_PERCENT: u16 = 60;
const TOP_BIAS_PERCENT: u16 = 30;
const SELECT_LIST_MAX_ITEMS: u16 = 10;
const SELECT_LIST_BORDER_ROWS: u16 = 2;
const CHOICE_MENU_DESCRIPTION_MIN_ROWS: u16 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ModalTone {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModalSpec {
    pub(crate) title: &'static str,
    pub(crate) icon: &'static str,
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
            icon: default_icon_for_tone(tone),
            tone,
            target_width,
            target_height,
            min_width,
            min_height,
            footer_height,
        }
    }

    pub(crate) fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = icon;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModalFrame {
    pub(crate) outer: Rect,
    pub(crate) content: Rect,
    pub(crate) footer: Option<Rect>,
    pub(crate) tone: ModalTone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfirmBody {
    pub(crate) primary: String,
    pub(crate) secondary: Option<String>,
    pub(crate) details: Option<String>,
}

impl ConfirmBody {
    pub(crate) fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            secondary: None,
            details: None,
        }
    }

    pub(crate) fn secondary(mut self, secondary: impl Into<String>) -> Self {
        self.secondary = Some(secondary.into());
        self
    }

    pub(crate) fn details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

pub(crate) struct ChoiceMenuBody<'a, T: Copy + PartialEq> {
    pub(crate) intro: String,
    pub(crate) list_title: &'static str,
    pub(crate) choices: &'a [(T, &'static str, Style)],
    pub(crate) selected: T,
    pub(crate) list_height: u16,
    pub(crate) description: String,
}

pub(crate) fn render_modal(
    frame: &mut Frame<'_>,
    area: Rect,
    spec: ModalSpec,
    action_rows: &[&[ModalAction]],
    render_content: impl FnOnce(&mut Frame<'_>, Rect),
) -> Option<ModalFrame> {
    let modal = render_modal_frame(frame, area, spec)?;
    render_content(frame, modal.content);
    render_action_rows(frame, modal.footer, modal.tone, action_rows);
    Some(modal)
}

fn render_modal_frame(frame: &mut Frame<'_>, area: Rect, spec: ModalSpec) -> Option<ModalFrame> {
    let outer = modal_rect(area, spec)?;
    let tone_style = modal_tone_style(spec.tone);
    let block = Block::default()
        .title(modal_title(spec))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(tone_style);
    let content_area = inset_rect(block.inner(outer), 1, 0);
    let sections = split_footer(content_area, spec.footer_height)?;

    frame.render_widget(Block::default().style(modal_scrim_style()), area);
    frame.render_widget(Clear, outer);
    frame.render_widget(Block::default().style(modal_background_style()), outer);
    frame.render_widget(block, outer);
    if let Some(separator) = sections.separator {
        render_footer_separator(frame, separator);
    }

    Some(ModalFrame {
        outer,
        content: sections.content,
        footer: sections.footer,
        tone: spec.tone,
    })
}

fn render_action_rows(
    frame: &mut Frame<'_>,
    footer: Option<Rect>,
    tone: ModalTone,
    action_rows: &[&[ModalAction]],
) {
    let Some(footer) = footer else {
        return;
    };
    if action_rows.is_empty() {
        return;
    }

    if action_rows.len() == 1 {
        render_action_footer(frame, footer, tone, action_rows[0]);
        return;
    }

    let constraints = action_rows
        .iter()
        .map(|_| Constraint::Length(1))
        .collect::<Vec<_>>();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(footer);
    for (area, actions) in rows.iter().zip(action_rows.iter()) {
        render_action_footer(frame, *area, tone, actions);
    }
}

pub(crate) fn render_confirm_body(
    frame: &mut Frame<'_>,
    area: Rect,
    tone: ModalTone,
    body: ConfirmBody,
) {
    let secondary_height = u16::from(body.secondary.is_some());
    let details_height = body
        .details
        .as_ref()
        .map(|details| {
            details
                .lines()
                .count()
                .try_into()
                .unwrap_or(u16::MAX)
                .saturating_add(1)
        })
        .unwrap_or(0);
    let total_height = 1u16
        .saturating_add(secondary_height)
        .saturating_add(details_height)
        .min(area.height);
    let start_y = area.y + area.height.saturating_sub(total_height) / 2;
    let centered = Rect::new(area.x, start_y, area.width, total_height);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(secondary_height),
            Constraint::Min(0),
        ])
        .split(centered);

    frame.render_widget(
        Paragraph::new(body.primary)
            .style(modal_tone_style(tone))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        rows[0],
    );

    if let Some(secondary) = body.secondary {
        frame.render_widget(
            Paragraph::new(secondary)
                .style(modal_text_style())
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            rows[1],
        );
    }

    if let Some(details) = body.details {
        let detail_area = Rect::new(
            rows[2].x,
            rows[2].y.saturating_add(1),
            rows[2].width,
            rows[2].height.saturating_sub(1),
        );
        frame.render_widget(
            Paragraph::new(details)
                .style(modal_muted_style())
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: false }),
            detail_area,
        );
    }
}

pub(crate) fn render_choice_menu_body<T: Copy + PartialEq>(
    frame: &mut Frame<'_>,
    area: Rect,
    tone: ModalTone,
    body: ChoiceMenuBody<'_, T>,
) {
    let list_height = choice_list_height(body.choices, body.list_height);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(list_height),
            Constraint::Length(1),
            Constraint::Min(CHOICE_MENU_DESCRIPTION_MIN_ROWS),
        ])
        .split(area);
    render_text(frame, rows[0], body.intro);
    render_choice_list(
        frame,
        rows[1],
        body.list_title,
        body.choices,
        body.selected,
        tone,
    );
    render_section_label(frame, rows[2], "Description");
    render_text(frame, rows[3], body.description);
}

pub(crate) fn modal_content_rect(area: Rect, spec: ModalSpec) -> Option<Rect> {
    let outer = modal_rect(area, spec)?;
    let content_area = inset_rect(Block::default().borders(Borders::ALL).inner(outer), 1, 0);
    split_footer(content_area, spec.footer_height).map(|sections| sections.content)
}

pub(crate) fn choice_menu_modal_height(choice_count: usize, footer_height: u16) -> u16 {
    let content_height = 1u16
        .saturating_add(choice_list_height_for_count(choice_count))
        .saturating_add(1)
        .saturating_add(CHOICE_MENU_DESCRIPTION_MIN_ROWS);
    let footer_section_height = if footer_height == 0 {
        0
    } else {
        footer_height.saturating_add(1)
    };
    content_height
        .saturating_add(footer_section_height)
        .saturating_add(2)
}

pub(crate) fn render_section_label(frame: &mut Frame<'_>, area: Rect, label: impl Into<String>) {
    frame.render_widget(
        Paragraph::new(label.into()).style(modal_muted_style()),
        area,
    );
}

pub(crate) fn render_text(frame: &mut Frame<'_>, area: Rect, text: impl Into<String>) {
    frame.render_widget(
        Paragraph::new(text.into())
            .style(modal_text_style())
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn render_action_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    tone: ModalTone,
    actions: &[(&'static str, &'static str)],
) {
    let mut spans = Vec::new();
    for (index, (key, label)) in actions.iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled("  ", modal_footer_style()));
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
        .map(|(choice, label, style)| {
            let row_style = if *choice == selected {
                modal_selected_row_style()
            } else {
                *style
            };
            ListItem::new(Line::styled(*label, row_style)).style(row_style)
        })
        .collect::<Vec<_>>();
    let selected_index = choices
        .iter()
        .position(|(choice, _, _)| *choice == selected)
        .unwrap_or(0);
    let mut list_state = ListState::default();
    list_state.select(Some(selected_index));
    let list = List::new(items)
        .highlight_style(modal_selected_row_style())
        .highlight_symbol("▌ ")
        .highlight_spacing(HighlightSpacing::Always)
        .block(
            Block::default()
                .title(Line::styled(format!(" {title} "), modal_tone_style(tone)))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(modal_border_style()),
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
        modal_border_style()
    };
    let content_style = if active {
        modal_active_input_style()
    } else {
        modal_text_style()
    };
    let content = if lines.is_empty() {
        vec![Line::from(" ")]
    } else {
        lines
    };
    frame.render_widget(
        Paragraph::new(content)
            .style(content_style)
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .title(Line::styled(format!(" {title} "), border_style))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ModalSections {
    content: Rect,
    separator: Option<Rect>,
    footer: Option<Rect>,
}

fn split_footer(area: Rect, footer_height: u16) -> Option<ModalSections> {
    if footer_height == 0 {
        return Some(ModalSections {
            content: area,
            separator: None,
            footer: None,
        });
    }
    if area.height <= footer_height.saturating_add(1) {
        return None;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(footer_height),
        ])
        .split(area);
    Some(ModalSections {
        content: rows[0],
        separator: Some(rows[1]),
        footer: Some(rows[2]),
    })
}

fn modal_rect(area: Rect, spec: ModalSpec) -> Option<Rect> {
    let max_width = area.width.saturating_sub(2).max(1);
    let max_height = area.height.saturating_sub(2).max(1);
    let min_width = spec.min_width.max(DESIGN_MIN_WIDTH).min(max_width);
    let percent_width =
        ((area.width as u32).saturating_mul(DESIGN_TARGET_WIDTH_PERCENT as u32) / 100) as u16;
    let width = spec
        .target_width
        .min(DESIGN_MAX_WIDTH)
        .min(percent_width.max(1))
        .min(max_width)
        .max(min_width);
    let height = spec
        .target_height
        .min(max_height)
        .max(spec.min_height.min(area.height));
    let top_offset = area
        .height
        .saturating_sub(height)
        .saturating_mul(TOP_BIAS_PERCENT)
        / 100;
    let rect = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + top_offset,
        width,
        height,
    );
    (rect.width >= min_width && rect.height >= spec.min_height).then_some(rect)
}

fn default_icon_for_tone(tone: ModalTone) -> &'static str {
    match tone {
        ModalTone::Info => "ℹ",
        ModalTone::Warning => "⚠",
        ModalTone::Danger => "‼",
    }
}

fn modal_title(spec: ModalSpec) -> Line<'static> {
    Line::styled(
        format!(" {} {} ", spec.icon, spec.title),
        modal_tone_style(spec.tone),
    )
}

fn render_footer_separator(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new("─".repeat(area.width as usize)).style(modal_border_style()),
        area,
    );
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

fn choice_list_height<T>(choices: &[(T, &'static str, Style)], requested_height: u16) -> u16 {
    requested_height
        .max(choice_list_height_for_count(choices.len()))
        .min(SELECT_LIST_MAX_ITEMS.saturating_add(SELECT_LIST_BORDER_ROWS))
}

fn choice_list_height_for_count(choice_count: usize) -> u16 {
    u16::try_from(choice_count)
        .unwrap_or(u16::MAX)
        .clamp(1, SELECT_LIST_MAX_ITEMS)
        .saturating_add(SELECT_LIST_BORDER_ROWS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choice_list_height_caps_visible_items_at_ten() {
        assert_eq!(choice_list_height_for_count(3), 5);
        assert_eq!(choice_list_height_for_count(10), 12);
        assert_eq!(choice_list_height_for_count(42), 12);
    }

    #[test]
    fn choice_list_height_respects_cap_even_when_requested_larger() {
        let choices = [(0, "choice", Style::default()); 42];

        assert_eq!(choice_list_height(&choices, 20), 12);
    }

    #[test]
    fn choice_menu_modal_height_allows_ten_visible_items() {
        assert_eq!(choice_menu_modal_height(4, 1), 14);
        assert_eq!(choice_menu_modal_height(42, 1), 20);
    }
}
