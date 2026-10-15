#![allow(dead_code)]

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
