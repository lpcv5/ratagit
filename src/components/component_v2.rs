// src/components/component_v2.rs
//
// ComponentV2 trait for event-driven UI components.
//
// This trait defines the interface for all UI panels in the event-driven architecture.
// Components are stateless - they read from AppState and return AppEvent.
// They never mutate state directly.

use crate::app::events::AppEvent;
use crate::app::AppState;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

/// Component trait for event-driven architecture
///
/// All UI panels implement this trait. Components follow these principles:
/// - Read state from AppState (immutable reference)
/// - Return AppEvent to communicate upward
/// - Never mutate state directly
/// - Keep rendering logic separate from event handling
pub trait ComponentV2 {
    /// Handle keyboard input and return an event
    ///
    /// This is called when the component has focus and receives input.
    /// The component should:
    /// 1. Check the key event
    /// 2. Consult AppState if needed (e.g., check selection)
    /// 3. Return appropriate AppEvent
    ///
    /// Common patterns:
    /// - Navigation keys (j/k) → AppEvent::SelectionChanged
    /// - Action keys (Space, Enter) → AppEvent::Git(...) or AppEvent::Modal(...)
    /// - Panel switch keys (h/l, 1-4) → AppEvent::SwitchPanel(...)
    /// - Unhandled keys → AppEvent::None
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent;

    /// Render the component
    ///
    /// This is called every frame. The component should:
    /// 1. Read data from AppState
    /// 2. Render to the provided buffer
    /// 3. Not perform any side effects
    ///
    /// Rendering is decoupled from event handling - state changes happen
    /// via events, not during rendering.
    #[allow(dead_code)] // Trait method, used by implementors
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
