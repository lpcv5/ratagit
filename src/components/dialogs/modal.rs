use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use crate::app::events::{AppEvent, ModalEvent};
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
    Help {
        title: String,
        items: Vec<(String, Intent)>,
        selected: usize,
    },
    TextInput {
        title: String,
        prompt: String,
        buffer: String,
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

    pub fn help(title: String, items: Vec<(String, Intent)>) -> Self {
        Self {
            modal_type: ModalType::Help {
                title,
                items,
                selected: 0,
            },
        }
    }

    pub fn text_input(title: String, prompt: String) -> Self {
        Self {
            modal_type: ModalType::TextInput {
                title,
                prompt,
                buffer: String::new(),
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
            ModalType::Help {
                items, selected, ..
            } => match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    if *selected < items.len().saturating_sub(1) {
                        *selected += 1;
                    }
                    Intent::None
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    *selected = selected.saturating_sub(1);
                    Intent::None
                }
                KeyCode::Enter => {
                    let intent = items
                        .get(*selected)
                        .map(|(_, i)| i.clone())
                        .unwrap_or(Intent::None);
                    Intent::TriggerHelpItem(Box::new(intent))
                }
                KeyCode::Esc => Intent::CloseModal,
                _ => Intent::None,
            },
            ModalType::TextInput { buffer, .. } => match code {
                KeyCode::Enter => {
                    let msg = buffer.trim().to_string();
                    if msg.is_empty() {
                        Intent::None
                    } else {
                        Intent::CommitWithMessage(msg)
                    }
                }
                KeyCode::Esc => Intent::CloseModal,
                KeyCode::Backspace => {
                    buffer.pop();
                    Intent::None
                }
                KeyCode::Char(c) => {
                    buffer.push(*c);
                    Intent::None
                }
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
            ModalType::Help {
                title,
                items,
                selected,
            } => self.render_help(frame, area, title, items, *selected),
            ModalType::TextInput {
                title,
                prompt,
                buffer,
            } => self.render_text_input(frame, area, title, prompt, buffer),
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

    fn render_help(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        items: &[(String, Intent)],
        selected: usize,
    ) {
        let popup_area = centered_rect(70, 60, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(189, 147, 249)));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        let list_items: Vec<ListItem> = items
            .iter()
            .map(|(label, _)| ListItem::new(label.as_str()))
            .collect();

        let list = List::new(list_items)
            .highlight_style(
                Style::default()
                    .bg(Color::Rgb(68, 71, 90))
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▸ ");

        let mut state = ListState::default();
        state.select(Some(selected));

        frame.render_stateful_widget(list, inner, &mut state);
    }

    fn render_text_input(
        &self,
        frame: &mut Frame,
        area: Rect,
        title: &str,
        prompt: &str,
        buffer: &str,
    ) {
        let popup_area = centered_rect(60, 30, area);

        // Clear the area
        frame.render_widget(Clear, popup_area);

        // Create the block
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(139, 233, 253)))
            .style(Style::default().bg(Color::Rgb(40, 42, 54)));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        // Split into prompt and input areas
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Length(3)])
            .split(inner);

        // Render prompt
        let prompt_text = Paragraph::new(prompt)
            .style(Style::default().fg(Color::Rgb(248, 248, 242)))
            .wrap(Wrap { trim: true });
        frame.render_widget(prompt_text, chunks[0]);

        // Render input field with cursor
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

// ============================================================================
// V2 Implementation: AppEvent-based Modal (parallel migration)
// ============================================================================

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
        options: Vec<String>,
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

    pub fn selection(title: String, options: Vec<String>) -> Self {
        Self {
            modal_type: ModalTypeV2::Selection {
                title,
                options,
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
        Self {
            modal_type: ModalTypeV2::TextInput {
                title,
                prompt,
                buffer: String::new(),
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
            ModalTypeV2::Selection { options, selected, .. } => match key_code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if *selected > 0 {
                        *selected -= 1;
                    }
                    AppEvent::None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if *selected < options.len().saturating_sub(1) {
                        *selected += 1;
                    }
                    AppEvent::None
                }
                KeyCode::Enter => AppEvent::Modal(ModalEvent::ShowResetConfirmation(*selected)),
                KeyCode::Esc => AppEvent::Modal(ModalEvent::Close),
                _ => AppEvent::None,
            },
            ModalTypeV2::Help { items, selected, .. } => match key_code {
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
                    let event = items.get(*selected).map(|(_, e)| e.clone()).unwrap_or(AppEvent::None);
                    // Return the event directly - the processor will handle closing the modal
                    event
                }
                KeyCode::Esc => AppEvent::Modal(ModalEvent::Close),
                _ => AppEvent::None,
            },
            ModalTypeV2::TextInput { buffer, .. } => match key_code {
                KeyCode::Char(c) => {
                    buffer.push(c);
                    AppEvent::None
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    AppEvent::None
                }
                KeyCode::Enter => {
                    if buffer.is_empty() {
                        AppEvent::None
                    } else {
                        AppEvent::Git(crate::app::events::GitEvent::CommitWithMessage(
                            buffer.clone(),
                        ))
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
                options,
                selected,
            } => self.render_selection(frame, area, title, options, *selected),
            ModalTypeV2::Help {
                title,
                items,
                selected,
            } => self.render_help_v2(frame, area, title, items, *selected),
            ModalTypeV2::TextInput {
                title,
                prompt,
                buffer,
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
                Span::styled("y", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw(" - Yes  "),
                Span::styled("n", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
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
        options: &[String],
        selected: usize,
    ) {
        let popup_area = centered_rect(60, 40, area);

        frame.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = options
            .iter()
            .enumerate()
            .map(|(i, opt)| {
                let style = if i == selected {
                    Style::default()
                        .fg(Color::Rgb(80, 250, 123))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Rgb(248, 248, 242))
                };
                ListItem::new(opt.as_str()).style(style)
            })
            .collect();

        let list = List::new(items)
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
        let event = Event::Key(KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));

        // Test 'n' key returns Close event
        let event = Event::Key(KeyEvent::new(KeyCode::Char('n'), crossterm::event::KeyModifiers::NONE));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));

        // Test 'y' key returns the on_confirm event
        let event = Event::Key(KeyEvent::new(KeyCode::Char('y'), crossterm::event::KeyModifiers::NONE));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));
    }

    #[test]
    fn test_modal_v2_text_input_returns_app_event() {
        let mut modal = ModalDialogV2::text_input(
            "Commit".to_string(),
            "Enter message:".to_string(),
        );

        // Test typing characters
        let event = Event::Key(KeyEvent::new(KeyCode::Char('a'), crossterm::event::KeyModifiers::NONE));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::None);

        // Test Esc returns Close
        let event = Event::Key(KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE));
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
        let event = Event::Key(KeyEvent::new(KeyCode::Esc, crossterm::event::KeyModifiers::NONE));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::Modal(ModalEvent::Close));

        // Test navigation returns None
        let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), crossterm::event::KeyModifiers::NONE));
        let result = modal.handle_event_v2(&event);
        assert_eq!(result, AppEvent::None);
    }
}
