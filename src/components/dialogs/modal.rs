use crossterm::event::{Event, KeyCode, KeyEventKind};

#[cfg(test)]
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::events::{AppEvent, GitEvent, ModalEvent};

#[derive(Debug, Clone)]
pub struct SelectionItemV2 {
    pub label: String,
    pub event: AppEvent,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub enum TextSubmitAction {
    CommitMessage,
    CreateBranch { from_branch: String },
}

/// Modal dialog types for V2 (AppEvent-based)
#[derive(Debug, Clone)]
pub enum ModalTypeV2 {
    Confirmation {
        title: String,
        message: String,
        on_confirm: Box<AppEvent>,
    },
    Selection {
        title: String,
        items: Vec<SelectionItemV2>,
        selected: usize,
    },
    Help {
        title: String,
        items: Vec<(String, AppEvent)>,
        selected: usize,
    },
    TextInput {
        title: String,
        prompt: String,
        buffer: String,
        submit_action: TextSubmitAction,
    },
}

/// Modal dialog component V2 (returns AppEvent)
pub struct ModalDialogV2 {
    modal_type: ModalTypeV2,
}

impl ModalDialogV2 {
    pub fn confirmation(title: String, message: String, on_confirm: AppEvent) -> Self {
        Self {
            modal_type: ModalTypeV2::Confirmation {
                title,
                message,
                on_confirm: Box::new(on_confirm),
            },
        }
    }

    pub fn selection(title: String, items: Vec<SelectionItemV2>) -> Self {
        Self {
            modal_type: ModalTypeV2::Selection {
                title,
                items,
                selected: 0,
            },
        }
    }

    pub fn help(title: String, items: Vec<(String, AppEvent)>) -> Self {
        Self {
            modal_type: ModalTypeV2::Help {
                title,
                items,
                selected: 0,
            },
        }
    }

    pub fn text_input(title: String, prompt: String) -> Self {
        Self::text_input_with_action(title, prompt, TextSubmitAction::CommitMessage)
    }

    pub fn text_input_with_action(
        title: String,
        prompt: String,
        submit_action: TextSubmitAction,
    ) -> Self {
        Self {
            modal_type: ModalTypeV2::TextInput {
                title,
                prompt,
                buffer: String::new(),
                submit_action,
            },
        }
    }

    /// Handle keyboard events and return AppEvent
    pub fn handle_event_v2(&mut self, event: &Event) -> AppEvent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return AppEvent::None;
            }

            self.handle_modal_specific_key_v2(key.code)
        } else {
            AppEvent::None
        }
    }

    fn handle_modal_specific_key_v2(&mut self, key_code: KeyCode) -> AppEvent {
        match &mut self.modal_type {
            ModalTypeV2::Confirmation { on_confirm, .. } => match key_code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => *on_confirm.clone(),
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    AppEvent::Modal(ModalEvent::Close)
                }
                _ => AppEvent::None,
            },
            ModalTypeV2::Selection {
                items, selected, ..
            } => match key_code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    AppEvent::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected < items.len().saturating_sub(1) {
                        *selected += 1;
                    }
                    AppEvent::None
                }
                KeyCode::Enter => items
                    .get(*selected)
                    .filter(|item| item.enabled)
                    .map(|item| item.event.clone())
                    .unwrap_or(AppEvent::None),
                KeyCode::Esc => AppEvent::Modal(ModalEvent::Close),
                _ => AppEvent::None,
            },
            ModalTypeV2::Help {
                items, selected, ..
            } => match key_code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    AppEvent::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected < items.len().saturating_sub(1) {
                        *selected += 1;
                    }
                    AppEvent::None
                }
                KeyCode::Enter => {
                    let event = items
                        .get(*selected)
                        .map(|(_, e)| e.clone())
                        .unwrap_or(AppEvent::None);
                    // Return the event directly - the processor will handle closing the modal
                    event
                }
                KeyCode::Esc => AppEvent::Modal(ModalEvent::Close),
                _ => AppEvent::None,
            },
            ModalTypeV2::TextInput {
                buffer,
                submit_action,
                ..
            } => match key_code {
                KeyCode::Char(c) => {
                    buffer.push(c);
                    AppEvent::None
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    AppEvent::None
                }
                KeyCode::Enter => {
                    let value = buffer.trim();
                    if value.is_empty() {
                        AppEvent::None
                    } else {
                        match submit_action {
                            TextSubmitAction::CommitMessage => {
                                AppEvent::Git(GitEvent::CommitWithMessage(value.to_string()))
                            }
                            TextSubmitAction::CreateBranch { from_branch } => {
                                AppEvent::Git(GitEvent::CreateBranch {
                                    new_name: value.to_string(),
                                    from_branch: from_branch.clone(),
                                })
                            }
                        }
                    }
                }
                KeyCode::Esc => AppEvent::Modal(ModalEvent::Close),
                _ => AppEvent::None,
            },
        }
    }

    /// Render the modal dialog (reuses existing rendering logic)
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match &self.modal_type {
            ModalTypeV2::Confirmation { title, message, .. } => {
                self.render_confirmation(frame, area, title, message)
            }
            ModalTypeV2::Selection {
                title,
                items,
                selected,
            } => self.render_selection(frame, area, title, items, *selected),
            ModalTypeV2::Help {
                title,
                items,
                selected,
            } => self.render_help_v2(frame, area, title, items, *selected),
            ModalTypeV2::TextInput {
                title,
                prompt,
                buffer,
                ..
            } => self.render_text_input(frame, area, title, prompt, buffer),
        }
    }

    // Reuse existing rendering methods from ModalDialog
    fn render_confirmation(&self, frame: &mut Frame, area: Rect, title: &str, message: &str) {
        let popup_area = centered_rect(60, 30, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .style(Style::default().bg(Color::Rgb(40, 42, 54)));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let text = vec![
            Line::from(message).style(Style::default().fg(Color::Rgb(248, 248, 242))),
            Line::from(""),
            Line::from(vec![
                Span::styled(
                    "y",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - Yes  "),
                Span::styled(
                    "n",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" - No"),
            ]),
        ];

        let paragraph = Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });

        frame.render_widget(paragraph, inner);
    }

    fn render_selection(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        items: &[SelectionItemV2],
        selected: usize,
    ) {
        let popup_area = centered_rect(60, 40, area);

        frame.render_widget(Clear, popup_area);

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if !item.enabled {
                    Style::default().fg(Color::DarkGray)
                } else if i == selected {
                    Style::default()
                        .fg(Color::Rgb(80, 250, 123))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(248, 248, 242))
                };
                ListItem::new(item.label.as_str()).style(style)
            })
            .collect();

        let list = List::new(list_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(139, 233, 253)))
                    .style(Style::default().bg(Color::Rgb(40, 42, 54))),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(80, 250, 123))
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(selected));

        frame.render_stateful_widget(list, popup_area, &mut state);
    }

    fn render_help_v2(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        items: &[(String, AppEvent)],
        selected: usize,
    ) {
        let popup_area = centered_rect(70, 60, area);

        frame.render_widget(Clear, popup_area);

        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, (text, _))| {
                let style = if i == selected {
                    Style::default()
                        .fg(Color::Rgb(80, 250, 123))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(248, 248, 242))
                };
                ListItem::new(text.as_str()).style(style)
            })
            .collect();

        let list = List::new(list_items)
            .block(
                Block::default()
                    .title(title)
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(139, 233, 253)))
                    .style(Style::default().bg(Color::Rgb(40, 42, 54))),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Rgb(80, 250, 123))
                    .add_modifier(Modifier::BOLD),
            );

        let mut state = ListState::default();
        state.select(Some(selected));

        frame.render_stateful_widget(list, popup_area, &mut state);
    }

    fn render_text_input(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        prompt: &str,
        buffer: &str,
    ) {
        let popup_area = centered_rect(60, 20, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(139, 233, 253)))
            .style(Style::default().bg(Color::Rgb(40, 42, 54)));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(3)])
            .split(inner);

        let prompt_text = Paragraph::new(prompt)
            .style(Style::default().fg(Color::Rgb(248, 248, 242)))
            .wrap(Wrap { trim: true });
        frame.render_widget(prompt_text, chunks[0]);

        let input_text = format!("{}_", buffer);
        let input = Paragraph::new(input_text)
            .style(Style::default().fg(Color::Rgb(80, 250, 123)))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Rgb(98, 114, 164))),
            );
        frame.render_widget(input, chunks[1]);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::{AppEvent, ModalEvent};

    #[test]
    fn test_modal_v2_confirmation_returns_app_event() {
        let mut modal = ModalDialogV2::confirmation(
            "Test".to_string(),
            "Test message".to_string(),
            AppEvent::Modal(ModalEvent::Close),
        );

        // Test Esc key returns Close event
        let event = Event::Key(KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));

        // Test 'n' key returns Close event
        let event = Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));

        // Test 'y' key returns the on_confirm event
        let event = Event::Key(KeyEvent::new(
            KeyCode::Char('y'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));
    }

    #[test]
    fn test_modal_v2_text_input_returns_app_event() {
        let mut modal =
            ModalDialogV2::text_input("Commit".to_string(), "Enter message:".to_string());

        // Test typing characters
        let event = Event::Key(KeyEvent::new(
            KeyCode::Char('a'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::None);

        // Test Esc returns Close
        let event = Event::Key(KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));
    }

    #[test]
    fn test_modal_v2_help_returns_app_event() {
        let items = vec![
            ("j - Down".to_string(), AppEvent::None),
            ("k - Up".to_string(), AppEvent::None),
        ];
        let mut modal = ModalDialogV2::help("Help".to_string(), items);

        // Test Esc returns Close
        let event = Event::Key(KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));

        // Test navigation returns None
        let event = Event::Key(KeyEvent::new(
            KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        ));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::None);
    }
}
