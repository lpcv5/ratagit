use ratatui::{layout::Rect, widgets::ListState, Frame};

use crate::app::CachedData;
use crate::components::core::{SelectableList, LIST_HIGHLIGHT_SYMBOL};

pub struct SimpleListPanel {
    pub state: ListState,
    render_fn: fn(&mut Frame, Rect, bool, &CachedData, &mut ListState),
}

impl SimpleListPanel {
    pub fn new(
        _title: &'static str,
        render_fn: fn(&mut Frame, Rect, bool, &CachedData, &mut ListState),
    ) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self { state, render_fn }
    }

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        data: &CachedData,
    ) {
        (self.render_fn)(frame, area, is_focused, data, &mut self.state);
    }
}

pub fn render_branches(
    frame: &mut Frame,
    area: Rect,
    is_focused: bool,
    data: &CachedData,
    state: &mut ListState,
) {
    if data.branches.is_empty() {
        SelectableList::render_empty(frame, area, "Branches", is_focused);
        return;
    }
    let items: Vec<_> = data
        .branches
        .iter()
        .map(|b| {
            let marker = if b.is_head { "*" } else { " " };
            let upstream = b.upstream.as_deref().unwrap_or("-");
            ratatui::widgets::ListItem::new(format!("[{marker}] {}  ({upstream})", b.name))
        })
        .collect();
    SelectableList::new(items, "Branches", is_focused, LIST_HIGHLIGHT_SYMBOL)
        .render(frame, area, state);
}

pub fn render_stashes(
    frame: &mut Frame,
    area: Rect,
    is_focused: bool,
    data: &CachedData,
    state: &mut ListState,
) {
    if data.stashes.is_empty() {
        SelectableList::render_empty(frame, area, "Stash", is_focused);
        return;
    }
    let items: Vec<_> = data
        .stashes
        .iter()
        .map(|s| ratatui::widgets::ListItem::new(format!("#{} {}", s.index, s.message)))
        .collect();
    SelectableList::new(items, "Stash", is_focused, LIST_HIGHLIGHT_SYMBOL)
        .render(frame, area, state);
}
