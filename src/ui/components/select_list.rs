#![allow(dead_code)]
use crate::ui::theme::UiTheme;
use crate::ui::LIST_SCROLL_PADDING;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, StatefulWidget},
};

/// A single item in a [`SelectList`].
pub struct SelectItem<T> {
    /// Display label shown in the list.
    pub label: String,
    /// Optional secondary description shown after the label.
    pub description: Option<String>,
    /// The value returned when this item is selected.
    pub value: T,
    /// Disabled items are rendered dimmed and cannot be selected.
    pub enabled: bool,
}

/// Generic selection list widget.
///
/// Renders a bordered list with optional filter input. Used for modal choice
/// dialogs such as reset-mode selection, branch deletion options, and the
/// command palette.
///
/// # Usage
///
/// ```rust,ignore
/// let list = SelectList::new("Reset Mode", items);
/// let mut state = SelectListState::default();
/// frame.render_stateful_widget(list, area, &mut state);
/// ```
pub struct SelectList<T> {
    title: String,
    items: Vec<SelectItem<T>>,
    filter: String,
    show_filter: bool,
    is_active: bool,
}

/// Mutable state for [`SelectList`]: tracks cursor position.
#[derive(Debug, Default, Clone)]
pub struct SelectListState {
    pub list_state: ListState,
}

impl SelectListState {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.list_state.select(Some(0));
        s
    }

    /// Index of the currently highlighted item.
    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    /// Move cursor down by one, wrapping at the end.
    pub fn move_down(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        let next =
            self.list_state
                .selected()
                .map_or(0, |i| if i + 1 >= item_count { 0 } else { i + 1 });
        self.list_state.select(Some(next));
    }

    /// Move cursor up by one, wrapping at the start.
    pub fn move_up(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        let prev = self.list_state.selected().map_or(0, |i| {
            if i == 0 {
                item_count.saturating_sub(1)
            } else {
                i - 1
            }
        });
        self.list_state.select(Some(prev));
    }
}

impl<T> SelectList<T> {
    /// Create a new `SelectList` with the given `title` and `items`.
    pub fn new(title: impl Into<String>, items: Vec<SelectItem<T>>) -> Self {
        Self {
            title: title.into(),
            items,
            filter: String::new(),
            show_filter: false,
            is_active: true,
        }
    }

    /// Show a filter input line at the top.
    pub fn with_filter(mut self, filter: impl Into<String>) -> Self {
        self.filter = filter.into();
        self.show_filter = true;
        self
    }

    /// Whether this widget has keyboard focus (affects border color).
    pub fn active(mut self, active: bool) -> Self {
        self.is_active = active;
        self
    }

    /// Return the value of the item at `index`, if in bounds and enabled.
    pub fn get_value(&self, index: usize) -> Option<&T> {
        self.items
            .get(index)
            .filter(|i| i.enabled)
            .map(|i| &i.value)
    }

    /// Number of items in the list.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// True if there are no items.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl<T> StatefulWidget for SelectList<T> {
    type State = SelectListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let theme = UiTheme::default();

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(if self.is_active {
                Style::default().fg(theme.accent)
            } else {
                Style::default().fg(theme.border_inactive)
            });

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                if item.enabled {
                    let mut spans = vec![Span::styled(
                        item.label.clone(),
                        Style::default().fg(theme.text_primary),
                    )];
                    if let Some(ref desc) = item.description {
                        spans.push(Span::raw("  "));
                        spans.push(Span::styled(
                            desc.clone(),
                            Style::default().fg(theme.text_muted),
                        ));
                    }
                    ListItem::new(Line::from(spans))
                } else {
                    ListItem::new(Span::styled(
                        item.label.clone(),
                        Style::default()
                            .fg(theme.text_muted)
                            .add_modifier(Modifier::DIM),
                    ))
                }
            })
            .collect();

        let highlight = theme.active_highlight();

        let list = List::new(items)
            .block(block)
            .scroll_padding(LIST_SCROLL_PADDING)
            .highlight_style(highlight);

        StatefulWidget::render(list, area, buf, &mut state.list_state);

        // Render filter line above the list content if requested.
        if self.show_filter && area.height > 3 {
            let filter_text = format!("/{}", self.filter);
            let filter_area = Rect {
                x: area.x + 1,
                y: area.y + area.height.saturating_sub(2),
                width: area.width.saturating_sub(2),
                height: 1,
            };
            let filter_span = Span::styled(filter_text, Style::default().fg(theme.accent));
            ratatui::widgets::Widget::render(
                ratatui::widgets::Paragraph::new(filter_span),
                filter_area,
                buf,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_items(count: usize) -> Vec<SelectItem<usize>> {
        (0..count)
            .map(|i| SelectItem {
                label: format!("item {}", i),
                description: None,
                value: i,
                enabled: true,
            })
            .collect()
    }

    #[test]
    fn move_down_wraps_to_start() {
        let mut state = SelectListState::new();
        state.move_down(1);
        assert_eq!(state.selected(), Some(0));
    }

    #[test]
    fn move_up_wraps_to_end() {
        let mut state = SelectListState::new();
        state.move_up(3);
        assert_eq!(state.selected(), Some(2));
    }

    #[test]
    fn get_value_returns_none_for_disabled() {
        let mut items = make_items(2);
        items[0].enabled = false;
        let list = SelectList::new("Test", items);
        assert!(list.get_value(0).is_none());
        assert_eq!(*list.get_value(1).unwrap(), 1);
    }

    #[test]
    fn move_down_advances_cursor() {
        let mut state = SelectListState::new();
        state.move_down(3);
        assert_eq!(state.selected(), Some(1));
        state.move_down(3);
        assert_eq!(state.selected(), Some(2));
    }
}
