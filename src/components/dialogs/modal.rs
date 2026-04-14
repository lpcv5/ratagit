use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::components::Intent;

/// Modal dialog types
#[derive(Debug, Clone)]
pub enum ModalType {
    Confirmation {
        title: String,
        message: String,
        on_confirm: Box<Intent>,
    },
    Selection {
        title: String,
        options: Vec<String>,
        selected: usize,
    },
}

/// Modal dialog component
pub struct ModalDialog {
    modal_type: ModalType,
}

impl ModalDialog {
    pub fn confirmation(title: String, message: String, on_confirm: Intent) -> Self {
        Self {
            modal_type: ModalType::Confirmation {
                title,
                message,
                on_confirm: Box::new(on_confirm),
            },
        }
    }

    pub fn selection(title: String, options: Vec<String>) -> Self {
        Self {
            modal_type: ModalType::Selection {
                title,
                options,
                selected: 0,
            },
        }
    }

    /// Handle input events for the modal
    pub fn handle_event(&mut self, event: &Event) -> Intent {
        let Event::Key(KeyEvent { code, kind, .. }) = event else {
            return Intent::None;
        };

        if *kind != KeyEventKind::Press {
            return Intent::None;
        }

        match &mut self.modal_type {
            ModalType::Confirmation { on_confirm, .. } => match code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    // Return both the confirm action and close modal
                    // The executor will need to handle this properly
                    *on_confirm.clone()
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => Intent::CloseModal,
                _ => Intent::None,
            },
            ModalType::Selection {
                options, selected, ..
            } => match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if *selected < options.len().saturating_sub(1) {
                        *selected += 1;
                    }
                    Intent::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                    Intent::None
                }
                KeyCode::Enter => Intent::ExecuteResetOption(*selected),
                KeyCode::Esc => Intent::CloseModal,
                _ => Intent::None,
            },
        }
    }

    /// Render the modal dialog
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match &self.modal_type {
            ModalType::Confirmation { title, message, .. } => {
                self.render_confirmation(frame, area, title, message)
            }
            ModalType::Selection {
                title,
                options,
                selected,
            } => self.render_selection(frame, area, title, options, *selected),
        }
    }

    fn render_confirmation(&self, frame: &mut Frame, area: Rect, title: &str, message: &str) {
        let popup_area = centered_rect(60, 30, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(inner);

        let message_widget = Paragraph::new(message)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Center);
        frame.render_widget(message_widget, chunks[0]);

        let prompt = Paragraph::new(Line::from(vec![
            Span::styled("Press ", Style::default()),
            Span::styled(
                "y",
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Green),
            ),
            Span::styled(" to confirm, ", Style::default()),
            Span::styled(
                "n",
                Style::default().add_modifier(Modifier::BOLD).fg(Color::Red),
            ),
            Span::styled(" to cancel", Style::default()),
        ]))
        .alignment(Alignment::Center);
        frame.render_widget(prompt, chunks[1]);
    }

    fn render_selection(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        options: &[String],
        selected: usize,
    ) {
        let popup_area = centered_rect(60, 50, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let items: Vec<ListItem> = options
            .iter()
            .map(|opt| ListItem::new(opt.as_str()))
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        let mut state = ListState::default();
        state.select(Some(selected));

        frame.render_stateful_widget(list, inner, &mut state);
    }
}

/// Helper function to create a centered rect
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
