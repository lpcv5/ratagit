#![allow(dead_code)]

use crate::app::{App, Message};
use crate::ui::View;
use crate::git::FileStatus;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// Documentation comment in English.
pub struct StatusViewState {
    /// Documentation comment in English.
    pub current_list: CurrentList,

    /// Documentation comment in English.
    pub unstaged_state: ListState,

    /// Documentation comment in English.
    pub staged_state: ListState,

    /// Documentation comment in English.
    pub untracked_state: ListState,
}

/// Documentation comment in English.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurrentList {
    Unstaged,
    Staged,
    Untracked,
}

/// Documentation comment in English.
pub struct StatusView {
    state: StatusViewState,
}

impl StatusView {
    pub fn new() -> Self {
        let mut unstaged_state = ListState::default();
        unstaged_state.select(Some(0));

        Self {
            state: StatusViewState {
                current_list: CurrentList::Unstaged,
                unstaged_state,
                staged_state: ListState::default(),
                untracked_state: ListState::default(),
            },
        }
    }

    /// Documentation comment in English.
    fn get_status_color(status: &FileStatus) -> Color {
        match status {
            FileStatus::New => Color::Green,
            FileStatus::Modified => Color::Yellow,
            FileStatus::Deleted => Color::Red,
            FileStatus::Renamed => Color::Magenta,
            FileStatus::TypeChange => Color::Cyan,
        }
    }

    /// Documentation comment in English.
    fn get_status_text(status: &FileStatus) -> &'static str {
        match status {
            FileStatus::New => "new",
            FileStatus::Modified => "mod",
            FileStatus::Deleted => "del",
            FileStatus::Renamed => "ren",
            FileStatus::TypeChange => "typ",
        }
    }
}

impl View for StatusView {
    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        // Comment in English.
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Comment in English.
        {
            let mut items = Vec::new();

            // Comment in English.
            if !app.status.unstaged.is_empty() {
                items.push(ListItem::new("=== Unstaged ===").style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
                for file in &app.status.unstaged {
                    let status_text = Self::get_status_text(&file.status);
                    let color = Self::get_status_color(&file.status);
                    let text = format!("{} {}", status_text, file.path.display());
                    items.push(ListItem::new(text).style(Style::default().fg(color)));
                }
            }

            // Comment in English.
            if !app.status.untracked.is_empty() {
                items.push(ListItem::new("=== Untracked ===").style(
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ));
                for file in &app.status.untracked {
                    let text = format!("??? {}", file.path.display());
                    items.push(ListItem::new(text).style(Style::default().fg(Color::Gray)));
                }
            }

            if items.is_empty() {
                items.push(ListItem::new("No changes"));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Changes"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_stateful_widget(list, chunks[0], &mut self.state.unstaged_state.clone());
        }

        // Comment in English.
        {
            let mut items = Vec::new();

            if !app.status.staged.is_empty() {
                for file in &app.status.staged {
                    let status_text = Self::get_status_text(&file.status);
                    let color = Self::get_status_color(&file.status);
                    let text = format!("{} {}", status_text, file.path.display());
                    items.push(ListItem::new(text).style(Style::default().fg(color)));
                }
            } else {
                items.push(ListItem::new("No staged changes"));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Staged"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_stateful_widget(list, chunks[1], &mut self.state.staged_state.clone());
        }
    }

    fn handle_key(&self, key: KeyEvent, _app: &App) -> Option<Message> {
        match key.code {
            // Comment in English.
            KeyCode::Char('j') | KeyCode::Down => {
                // TODO: implement this behavior.
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // TODO: implement this behavior.
                None
            }

            // Comment in English.
            KeyCode::Char(' ') => {
                // TODO: implement this behavior.
                None
            }

            // Comment in English.
            KeyCode::Char('r') => Some(Message::RefreshStatus),

            _ => None,
        }
    }
}
