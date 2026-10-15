use crate::ui::theme::UiTheme;
use ratatui::{style::Style, widgets::ListItem};

/// Build a single-item list showing an empty-state message.
pub fn empty_list_item(text: &str) -> Vec<ListItem<'static>> {
    let theme = UiTheme::default();
    vec![ListItem::new(text.to_owned()).style(Style::default().fg(theme.text_muted))]
}
