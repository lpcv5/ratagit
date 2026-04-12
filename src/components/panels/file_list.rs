use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use crate::app::CachedData;

use crate::components::core::SelectableList;
use crate::components::Component;
use crate::components::Intent;

/// 文件列表面板组件（持有自身状态）
pub struct FileListPanel {
    state: ListState,
}

impl FileListPanel {
    pub fn new() -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state }
    }

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }
}

impl Default for FileListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for FileListPanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return Intent::SelectNext,
                KeyCode::Char('k') | KeyCode::Up => return Intent::SelectPrevious,
                KeyCode::Char(' ') if key.modifiers.is_empty() => return Intent::ToggleStageFile,
                KeyCode::Enter => return Intent::ActivatePanel,
                _ => {}
            }
        }

        Intent::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        if data.files.is_empty() {
            SelectableList::render_empty(frame, area, "Files", is_focused);
            return;
        }

        let items: Vec<ListItem<'_>> = data
            .files
            .iter()
            .map(|file| {
                let marker = match (file.is_staged, file.is_unstaged) {
                    (true, true) => "SU",
                    (true, false) => "S ",
                    (false, true) => "U ",
                    (false, false) => "  ",
                };

                let file_color = match (file.is_staged, file.is_unstaged) {
                    (true, true) => Color::LightCyan,
                    (true, false) => Color::Green,
                    (false, true) => Color::Yellow,
                    (false, false) => Color::Reset,
                };

                ListItem::new(Line::from(vec![
                    Span::styled(format!("[{marker}] "), Style::default().fg(Color::DarkGray)),
                    Span::styled(file.path.clone(), Style::default().fg(file_color)),
                ]))
            })
            .collect();

        let list = SelectableList::new(items, "Files", is_focused, "> ");
        let state = &mut self.state.clone();
        list.render(frame, area, state);
    }
}
