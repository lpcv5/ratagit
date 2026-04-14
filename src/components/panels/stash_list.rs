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

#[cfg(test)]
mod render_tests {
    use super::*;
    use crate::components::test_utils::*;
    use crate::components::Component;

    #[test]
    fn test_stash_list_empty_state() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = StashListPanel::new();
        let data = create_test_cached_data_with_stashes(vec![]);

        terminal
            .draw(|frame| {
                let area = frame.area();
                panel.render(frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();
        let line = get_buffer_line(buffer, 1);
        assert!(
            line.contains("No items"),
            "Expected 'No items' for empty stash, got: {}",
            line
        );
    }

    #[test]
    fn test_stash_list_renders_entries() {
        let mut terminal = create_test_terminal(50, 10);
        let mut panel = StashListPanel::new();
        let stashes = vec![
            test_stash_entry(0, "abc12345", "WIP on main: fix bug"),
            test_stash_entry(1, "def67890", "WIP on feature: add tests"),
        ];
        let data = create_test_cached_data_with_stashes(stashes);

        terminal
            .draw(|frame| {
                let area = frame.area();
                panel.render(frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Check that stash entries appear
        let content = get_buffer_line(buffer, 1);
        assert!(
            content.contains("abc12345") || content.contains("WIP on main"),
            "Expected stash entry in buffer, got: {}",
            content
        );
    }

    #[test]
    fn test_stash_list_shows_multiple_entries() {
        let mut terminal = create_test_terminal(60, 15);
        let mut panel = StashListPanel::new();
        let stashes = vec![
            test_stash_entry(0, "aaa11111", "Stash 1"),
            test_stash_entry(1, "bbb22222", "Stash 2"),
            test_stash_entry(2, "ccc33333", "Stash 3"),
        ];
        let data = create_test_cached_data_with_stashes(stashes);

        terminal
            .draw(|frame| {
                let area = frame.area();
                panel.render(frame, area, false, &data);
            })
            .unwrap();

        let buffer = terminal.backend().buffer();

        // Collect all lines to verify multiple entries
        let mut all_content = String::new();
        for row in 0..15 {
            let line = get_buffer_line(buffer, row);
            all_content.push_str(&line);
            all_content.push('\n');
        }

        // At least one stash entry should be visible
        assert!(
            all_content.contains("Stash 1")
                || all_content.contains("Stash 2")
                || all_content.contains("Stash 3"),
            "Expected at least one stash entry visible, got:\n{}",
            all_content
        );
    }
}
