use crate::app::help::HelpGroup;
use crate::app::InputMode;
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render_keybindings(
    frame: &mut Frame,
    snapshot: &AppStateSnapshot<'_>,
    viewport: &mut super::selection::HelpViewport,
) {
    if snapshot.input_mode != Some(InputMode::Keybindings) {
        *viewport = Default::default();
        return;
    }
    let help = snapshot.help;
    let context = (help.context.clone(), help.query.clone());
    if viewport.context.as_ref() != Some(&context) {
        viewport.list = ListState::default();
        viewport.context = Some(context);
    }
    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 90, 90);
    let title = format!("Keybindings - {}", help.context);
    let inner = render_overlay_chrome(frame, area, &title, &theme);
    let sections = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(inner);
    let entries = help.filtered_entries();
    let summary = HelpGroup::ALL
        .map(|group| {
            format!(
                "{} ({})",
                group.title(),
                entries.iter().filter(|entry| entry.group == group).count()
            )
        })
        .join("   ");
    frame.render_widget(
        Paragraph::new(summary).style(Style::default().fg(theme.accent)),
        sections[0],
    );
    let filter = if help.filtering {
        format!("Filter: /{}", help.query)
    } else {
        "Press / to filter by description, command or key".into()
    };
    let filter_width = Line::from(filter.as_str()).width();
    frame.render_widget(
        Paragraph::new(filter).style(Style::default().fg(theme.text_muted)),
        sections[1],
    );

    let mut rows = Vec::new();
    let mut selected_row = None;
    let key_width = entries
        .iter()
        .map(|entry| entry.keys.join(", ").len())
        .max()
        .unwrap_or(0)
        .min(24)
        .min(usize::from(sections[2].width) / 2);
    for group in HelpGroup::ALL {
        rows.push(ListItem::new(Line::styled(
            group.title(),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )));
        let mut count = 0;
        for (index, entry) in entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.group == group)
        {
            count += 1;
            if index == help.selected {
                selected_row = Some(rows.len());
            }
            let keys = entry.keys.join(", ");
            let keys: String = keys.chars().take(key_width).collect();
            rows.push(ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{keys:>key_width$}  "),
                    Style::default().fg(theme.warning),
                ),
                Span::styled(&entry.description, Style::default().fg(theme.text_primary)),
            ])));
        }
        if count == 0 {
            let message = if group == HelpGroup::Custom && help.query.is_empty() {
                "No custom keybindings configured"
            } else {
                "No matching keybindings"
            };
            rows.push(ListItem::new(Line::styled(
                message,
                Style::default().fg(theme.text_muted),
            )));
        }
    }
    let count = rows.len();
    let list = List::new(rows)
        .style(Style::default().bg(theme.surface))
        .highlight_style(theme.active_highlight())
        .scroll_padding(crate::ui::LIST_SCROLL_PADDING)
        .highlight_symbol("> ");
    viewport.list.select(selected_row);
    *viewport.list.offset_mut() = super::selection::scroll_offset(
        viewport.list.offset(),
        selected_row,
        count,
        usize::from(sections[2].height),
    );
    frame.render_stateful_widget(list, sections[2], &mut viewport.list);
    let position = if entries.is_empty() {
        0
    } else {
        help.selected + 1
    };
    let controls = if help.filtering {
        "Up/Down Move  Enter Execute  Type to filter  Esc Clear"
    } else {
        "Up/Down Move  Enter/Space Execute  / Filter  Esc Close"
    };
    let footer = format!("{position}/{}  {controls}", entries.len());
    frame.render_widget(
        Paragraph::new(footer).style(Style::default().fg(theme.text_muted)),
        sections[3],
    );
    if help.filtering && sections[1].width > 0 && sections[1].height > 0 {
        let offset = filter_width.min(usize::from(sections[1].width - 1));
        frame.set_cursor_position((sections[1].x + offset as u16, sections[1].y));
    }
}
