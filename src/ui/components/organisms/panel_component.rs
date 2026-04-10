use crate::app::SidePanel;
use crate::ui::theme::UiTheme;
use ratatui::{style::Style, widgets::ListItem};
use std::{collections::HashSet, sync::OnceLock};

fn empty_visual_selected_indices() -> &'static HashSet<usize> {
    static EMPTY: OnceLock<HashSet<usize>> = OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

fn empty_highlighted_oids() -> &'static HashSet<String> {
    static EMPTY: OnceLock<HashSet<String>> = OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

pub struct PanelRenderContext<'a> {
    pub active_panel: SidePanel,
    pub panel_title_override: Option<&'a str>,
    pub search_query: Option<&'a str>,
    pub search_summary: Option<&'a str>,
    pub visual_selected_indices: &'a HashSet<usize>,
    pub highlighted_oids: &'a HashSet<String>,
}

impl<'a> PanelRenderContext<'a> {
    pub fn empty_visual_selected_indices() -> &'static HashSet<usize> {
        empty_visual_selected_indices()
    }

    pub fn empty_highlighted_oids() -> &'static HashSet<String> {
        empty_highlighted_oids()
    }
}

/// Append search summary to title if present.
pub fn title_with_search(title: &str, search_summary: Option<&str>) -> String {
    match search_summary {
        Some(s) => format!("{} [{}]", title, s),
        None => title.to_string(),
    }
}

/// Build a single-item list showing an empty-state message.
pub fn empty_list_item(text: &str) -> Vec<ListItem<'static>> {
    let theme = UiTheme::default();
    vec![ListItem::new(text.to_string()).style(Style::default().fg(theme.text_muted))]
}
