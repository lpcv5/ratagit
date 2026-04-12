use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    widgets::{ListItem, ListState},
    Frame,
};

use crate::app::CachedData;

use crate::components::core::{SelectableList, LIST_HIGHLIGHT_SYMBOL};
use crate::components::Component;
use crate::components::Intent;

/// 分支列表面板组件（持有自身状态）
pub struct BranchListPanel {
    state: ListState,
}

impl BranchListPanel {
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

impl Default for BranchListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for BranchListPanel {
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

    fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        if data.branches.is_empty() {
            SelectableList::render_empty(frame, area, "Branches", is_focused);
            return;
        }

        let items: Vec<ListItem<'_>> = data
            .branches
            .iter()
            .map(|branch| {
                let marker = if branch.is_head { "*" } else { " " };
                let upstream = branch.upstream.as_deref().unwrap_or("-");
                ListItem::new(format!("[{marker}] {}  ({upstream})", branch.name))
            })
            .collect();

        let list = SelectableList::new(items, "Branches", is_focused, LIST_HIGHLIGHT_SYMBOL);
        list.render(frame, area, &mut self.state);
    }
}
