// src/components/component_v2.rs
use crate::app::events::AppEvent;
use crate::app::AppState;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// New component trait that returns AppEvent instead of Intent
pub trait ComponentV2 {
    /// Handle keyboard input and return an event
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent;

    /// Render the component
    fn render(&self, area: Rect, buf: &mut Buffer, state: &AppState);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::AppEvent;
    use crate::app::AppState;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;

    struct TestComponent;

    impl ComponentV2 for TestComponent {
        fn handle_key_event(&mut self, _key: KeyEvent, _state: &AppState) -> AppEvent {
            AppEvent::None
        }

        fn render(&self, _area: Rect, _buf: &mut Buffer, _state: &AppState) {}
    }

    #[test]
    fn test_component_v2_trait() {
        let mut component = TestComponent;
        let state = mock_state();
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = component.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::None);
    }

    fn mock_state() -> AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        AppState::new(cmd_tx, event_rx)
    }
}
