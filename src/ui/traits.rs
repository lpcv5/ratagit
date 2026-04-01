use ratatui::{layout::Rect, Frame};

/// A key binding descriptor for introspection and help display.
pub struct KeyBinding {
    pub key: String,
    pub description: String,
}

/// Base trait for all interactive UI components.
///
/// # Flux Architecture Note
///
/// In this application's Flux architecture, actual event handling flows through
/// the store system: `input_mapper → Dispatcher → Stores → AppStateSnapshot → UI`.
/// This trait does NOT replace that pipeline. Instead it provides a uniform
/// interface for rendering and keybinding introspection, enabling:
/// - Consistent rendering signatures across all panel components
/// - Programmatic discovery of available key bindings (e.g., for CommandPalette)
///
/// Panels that implement this trait should still dispatch `Action`s through the
/// Flux system rather than handling events directly in `handle_event`.
pub trait InteractiveWidget {
    type State;
    type Action;

    /// Render this widget into `frame` at `area` using the given `state`.
    fn render(&self, frame: &mut Frame, area: Rect, state: &Self::State);

    /// Return the list of key bindings active in the given `state`.
    /// Used by CommandPalette and the shortcut bar for display.
    fn keybindings(&self, state: &Self::State) -> Vec<KeyBinding>;
}

/// Extends [`InteractiveWidget`] with visual mode multi-selection support.
///
/// Implementors hold their own visual-mode state (anchor, cursor, selection set).
/// The Flux stores remain the authoritative source for committed selections;
/// this trait covers the in-panel transient selection state.
pub trait SelectableWidget: InteractiveWidget {
    /// Enter visual selection mode, anchoring at the current cursor position.
    fn enter_visual_mode(&mut self);

    /// Exit visual selection mode and clear the transient selection.
    fn exit_visual_mode(&mut self);

    /// Return the indices of all currently selected items.
    fn get_selection(&self) -> Vec<usize>;

    /// Toggle the selection state of the item at `index`.
    fn toggle_select(&mut self, index: usize);

    /// Whether the widget is currently in visual selection mode.
    fn is_in_visual_mode(&self) -> bool;
}

/// Trait for panels that support dynamic height adjustment based on focus and content.
///
/// Panels report their preferred height as a percentage of available space.
/// The layout system calls [`effective_height_percent`] to determine actual
/// allocation, expanding focused panels when content exceeds the threshold.
pub trait DynamicPanel {
    /// Height percentage (0–100) when the panel is not focused or content is
    /// within the expand threshold.
    fn default_height_percent(&self) -> u16;

    /// Height percentage (0–100) when the panel is focused and content exceeds
    /// [`expand_threshold`].
    fn focused_height_percent(&self) -> u16;

    /// Number of content lines above which a focused panel should expand.
    fn expand_threshold(&self) -> usize;

    /// Minimum height in terminal rows (used for the collapsed/unfocused state
    /// of panels like Stash that collapse to a single line).
    fn min_height(&self) -> u16;

    /// Whether this panel should expand given the current `content_lines` count.
    fn should_expand(&self, content_lines: usize) -> bool {
        content_lines > self.expand_threshold()
    }

    /// Effective height percentage given focus state and content line count.
    ///
    /// Returns [`focused_height_percent`] when `is_focused` is true and content
    /// exceeds the threshold; otherwise returns [`default_height_percent`].
    fn effective_height_percent(&self, is_focused: bool, content_lines: usize) -> u16 {
        if is_focused && self.should_expand(content_lines) {
            self.focused_height_percent()
        } else {
            self.default_height_percent()
        }
    }
}
