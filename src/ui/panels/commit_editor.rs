use super::props::CommitEditorProps;
use crate::app::CommitFieldFocus;
use crate::ui::components::molecules::{card, shortcut_hint};
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_commit_editor(frame: &mut Frame, props: CommitEditorProps<'_>) {
    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 70, 60);
    let message_active = props.focus == CommitFieldFocus::Message;
    let desc_active = props.focus == CommitFieldFocus::Description;
    let (msg_line, msg_col) = line_col(props.message);
    let (desc_line, desc_col) = line_col(props.description);
    let (focus_name, focus_line, focus_col) = if message_active {
        ("Message", msg_line, msg_col)
    } else {
        ("Description", desc_line, desc_col)
    };

    let outer_title = format!(
        "Commit Editor | Focus: {} | Ln {} Col {}",
        focus_name, focus_line, focus_col
    );
    let inner = render_overlay_chrome(frame, area, &outer_title, &theme);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    let message = Paragraph::new(props.message).block(card(&theme, "Message", message_active));
    frame.render_widget(message, sections[0]);

    let description = Paragraph::new(props.description)
        .style(Style::default().fg(theme.text_primary))
        .block(card(&theme, "Description", desc_active));
    frame.render_widget(description, sections[1]);

    let help = Paragraph::new(Line::from(
        props
            .hints
            .iter()
            .flat_map(|(key, label)| shortcut_hint(key, &format!("{label}  "), &theme))
            .collect::<Vec<_>>(),
    ));
    frame.render_widget(help, sections[2]);

    // Place terminal cursor at active input field for direct typing feedback.
    let cursor = if message_active {
        let width = sections[0].width.saturating_sub(2).max(1);
        let x = sections[0]
            .x
            .saturating_add(1)
            .saturating_add(msg_col.saturating_sub(1).min(width.saturating_sub(1)));
        let y = sections[0].y.saturating_add(1);
        (x, y)
    } else {
        let width = sections[1].width.saturating_sub(2).max(1);
        let height = sections[1].height.saturating_sub(2).max(1);
        let x = sections[1]
            .x
            .saturating_add(1)
            .saturating_add(desc_col.saturating_sub(1).min(width.saturating_sub(1)));
        let y = sections[1]
            .y
            .saturating_add(1)
            .saturating_add(desc_line.saturating_sub(1).min(height.saturating_sub(1)));
        (x, y)
    };
    frame.set_cursor_position(cursor);
}

fn line_col(text: &str) -> (u16, u16) {
    let mut line = 1u16;
    let mut col = 1u16;
    for ch in text.chars() {
        if ch == '\n' {
            line = line.saturating_add(1);
            col = 1;
        } else {
            col = col.saturating_add(1);
        }
    }
    (line, col)
}
