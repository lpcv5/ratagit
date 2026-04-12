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

/// 提交列表面板组件（持有自身状态）
pub struct CommitListPanel {
    state: ListState,
}

impl CommitListPanel {
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

impl Default for CommitListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for CommitListPanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
                KeyCode::Char('j') | KeyCode::Down => return Intent::SelectNext,
                KeyCode::Char('k') | KeyCode::Up => return Intent::SelectPrevious,
                KeyCode::Enter => return Intent::ActivatePanel,
                _ => {}
            }
        }

        Intent::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        if data.commits.is_empty() {
            SelectableList::render_empty(frame, area, "Commits", is_focused);
            return;
        }

        let items: Vec<ListItem<'_>> = data
            .commits
            .iter()
            .map(|commit| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{} ", commit.short_id),
                        Style::default().fg(Color::LightBlue),
                    ),
                    Span::raw(commit.summary.clone()),
                ]))
            })
            .collect();

        let list = SelectableList::new(items, "Commits", is_focused, "> ");
        let state = &mut self.state.clone();
        list.render(frame, area, state);
    }
}
