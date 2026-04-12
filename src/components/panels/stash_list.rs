use crate::components::core::{render_stashes, SimpleListPanel};

pub struct StashListPanel(pub SimpleListPanel);

impl StashListPanel {
    pub fn new() -> Self {
        Self(SimpleListPanel::new("Stash", render_stashes))
    }

    pub fn state_mut(&mut self) -> &mut ratatui::widgets::ListState {
        self.0.state_mut()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.0.selected_index()
    }
}

impl Default for StashListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::components::Component for StashListPanel {
    fn handle_event(
        &mut self,
        event: &crossterm::event::Event,
        data: &crate::app::CachedData,
    ) -> crate::components::Intent {
        self.0.handle_event(event, data)
    }

    fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        data: &crate::app::CachedData,
    ) {
        self.0.render(frame, area, is_focused, data);
    }
}
