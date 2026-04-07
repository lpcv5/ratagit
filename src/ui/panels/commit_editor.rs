use crate::app::{CommitFieldFocus, InputMode};
use crate::flux::snapshot::AppStateSnapshot;
use crate::ui::panels::{centered_rect, render_overlay_chrome};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::Paragraph,
    Frame,
};

pub fn render_commit_editor(frame: &mut Frame, snapshot: &AppStateSnapshot<'_>) {
    if snapshot.input_mode != Some(InputMode::CommitEditor) {
        return;
    }

    let theme = UiTheme::default();
    let area = centered_rect(frame.area(), 70, 60);
    // Render chrome with empty title first; we'll overwrite with the dynamic title below.
    let inner = render_overlay_chrome(frame, area, "", &theme);

    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(5)])
        .split(inner);

    let message_active = snapshot.commit_focus == CommitFieldFocus::Message;
    let desc_active = snapshot.commit_focus == CommitFieldFocus::Description;
    let (msg_line, msg_col) = line_col(snapshot.commit_message_buffer);
    let (desc_line, desc_col) = line_col(snapshot.commit_description_buffer);
    let (focus_name, focus_line, focus_col) = if message_active {
        ("Message", msg_line, msg_col)
    } else {
        ("Description", desc_line, desc_col)
    };

    // Overwrite the outer block with the dynamic title.
    let outer_title = format!(
        "Commit Editor | Focus: {} | Ln {} Col {}",
        focus_name, focus_line, focus_col
    );
    frame.render_widget(theme.panel_block(&outer_title, true), area);

    let message_title = if message_active {
        "Message [ACTIVE] (Enter to confirm commit)"
    } else {
        "Message (Enter to confirm commit)"
    };
    let message = Paragraph::new(snapshot.commit_message_buffer)
        .block(theme.panel_block(message_title, message_active));
    frame.render_widget(message, sections[0]);

    let desc_title = if desc_active {
        "Description [ACTIVE] (Tab switch, Enter newline)"
    } else {
        "Description (Tab switch, Enter newline)"
    };
    let description = Paragraph::new(snapshot.commit_description_buffer)
        .style(Style::default().fg(theme.text_primary))
        .block(theme.panel_block(desc_title, desc_active));
    frame.render_widget(description, sections[1]);

    let help = Paragraph::new(Line::from(
        "Tab switch field | Enter confirm/newline | Esc cancel",
    ))
    .style(Style::default().fg(theme.text_muted));
    let help_area = Rect {
        x: inner.x,
        y: inner.y + inner.height.saturating_sub(1),
        width: inner.width,
        height: 1,
    };
    frame.render_widget(help, help_area);

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
