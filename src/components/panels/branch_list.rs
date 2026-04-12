use crate::components::core::{render_branches, SimpleListPanel};

pub struct BranchListPanel(pub SimpleListPanel);

impl BranchListPanel {
    pub fn new() -> Self {
        Self(SimpleListPanel::new("Branches", render_branches))
    }

    pub fn state_mut(&mut self) -> &mut ratatui::widgets::ListState {
        self.0.state_mut()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.0.selected_index()
    }
}

impl Default for BranchListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::components::Component for BranchListPanel {
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
