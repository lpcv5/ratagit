use crate::app::App;
use crate::git::FileStatus;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
    Frame,
};

pub fn get_status_color(status: &FileStatus) -> Color {
    match status {
        FileStatus::New => Color::Green,
        FileStatus::Modified => Color::Yellow,
        FileStatus::Deleted => Color::Red,
        FileStatus::Renamed => Color::Magenta,
        FileStatus::TypeChange => Color::Cyan,
    }
}

pub fn get_status_text(status: &FileStatus) -> &'static str {
    match status {
        FileStatus::New => "new",
        FileStatus::Modified => "mod",
        FileStatus::Deleted => "del",
        FileStatus::Renamed => "ren",
        FileStatus::TypeChange => "typ",
    }
}

pub fn render_files_panel(frame: &mut Frame, area: Rect, app: &App, is_active: bool) {
    let border_style = if is_active {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let title_style = if is_active {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mut items = Vec::new();

    for file in &app.status.unstaged {
        let text = format!("{} {}", get_status_text(&file.status), file.path.display());
        items.push(ListItem::new(text).style(Style::default().fg(get_status_color(&file.status))));
    }

    for file in &app.status.untracked {
        let text = format!("??? {}", file.path.display());
        items.push(ListItem::new(text).style(Style::default().fg(Color::Gray)));
    }

    for file in &app.status.staged {
        let text = format!("✓{} {}", get_status_text(&file.status), file.path.display());
        items.push(ListItem::new(text).style(Style::default().fg(Color::Green)));
    }

    if items.is_empty() {
        items.push(ListItem::new("No changes").style(Style::default().fg(Color::DarkGray)));
    }

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Files")
                .border_style(border_style)
                .title_style(title_style),
        )
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD));

    let mut state = app.files_panel.list_state.clone();
    frame.render_stateful_widget(list, area, &mut state);
}
