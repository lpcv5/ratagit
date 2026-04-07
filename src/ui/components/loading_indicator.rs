use crate::ui::theme::UiTheme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::Widget,
};

/// Braille spinner frames used for the loading animation.
const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Animated spinner widget displayed in the shortcut bar while background
/// Git tasks are running.
///
/// Callers advance the animation by calling [`tick`] once per render frame
/// (typically driven by the UI tick event). The widget renders inline as a
/// single `Span`, so it can be embedded in any [`Line`].
///
/// # Example
///
/// ```rust,ignore
/// // In shortcut_bar rendering:
/// if snapshot.has_background_tasks {
///     spans.push(indicator.current_span());
///     spans.push(Span::raw("  "));
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LoadingIndicator {
    current_frame: usize,
    message: String,
}

impl LoadingIndicator {
    /// Create a new indicator with an empty message.
    pub fn new() -> Self {
        Self {
            current_frame: 0,
            message: String::new(),
        }
    }

    /// Create a new indicator with the given operation `message`.
    #[allow(dead_code)]
    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            current_frame: 0,
            message: message.into(),
        }
    }

    /// Set or update the operation message shown next to the spinner.
    #[allow(dead_code)]
    pub fn set_message(&mut self, message: impl Into<String>) {
        self.message = message.into();
    }

    /// Advance to the next animation frame.
    pub fn tick(&mut self) {
        self.current_frame = (self.current_frame + 1) % SPINNER_FRAMES.len();
    }

    /// Return the current spinner character.
    pub fn current_spinner(&self) -> &'static str {
        SPINNER_FRAMES[self.current_frame]
    }

    /// Build a styled [`Span`] containing spinner + optional message.
    /// Suitable for inline embedding in a shortcut bar [`Line`].
    pub fn current_span(&self) -> Span<'static> {
        let theme = UiTheme::default();
        let text = if self.message.is_empty() {
            self.current_spinner().to_string()
        } else {
            format!("{} {}", self.current_spinner(), self.message)
        };
        Span::styled(text, Style::default().fg(theme.accent))
    }

    /// Render the spinner as a full single-line widget into the given area.
    /// Typically used when the indicator occupies its own dedicated row.
    pub fn into_line(self) -> Line<'static> {
        let theme = UiTheme::default();
        let spinner = Span::styled(
            SPINNER_FRAMES[self.current_frame].to_string(),
            Style::default().fg(theme.accent),
        );
        if self.message.is_empty() {
            Line::from(spinner)
        } else {
            Line::from(vec![
                spinner,
                Span::raw(" "),
                Span::styled(self.message, Style::default().fg(theme.text_primary)),
            ])
        }
    }
}

impl Default for LoadingIndicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for LoadingIndicator {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = self.into_line();
        ratatui::widgets::Paragraph::new(line).render(area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_cycles_through_all_frames() {
        let mut ind = LoadingIndicator::new();
        let total = SPINNER_FRAMES.len();
        for _ in 0..total {
            ind.tick();
        }
        // After a full cycle we're back to frame 0.
        assert_eq!(ind.current_frame, 0);
    }

    #[test]
    fn current_span_includes_message() {
        let ind = LoadingIndicator::with_message("git status");
        let span_text = ind.current_span().content.to_string();
        assert!(span_text.contains("git status"), "got: {span_text}");
    }

    #[test]
    fn current_span_no_message_returns_only_spinner() {
        let ind = LoadingIndicator::new();
        let spinner = ind.current_spinner();
        let span_text = ind.current_span().content.to_string();
        assert_eq!(span_text, spinner);
    }
}
