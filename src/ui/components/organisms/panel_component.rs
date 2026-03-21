use crate::app::SidePanel;
use ratatui::{layout::Rect, Frame};
use std::{collections::HashSet, sync::OnceLock};

fn empty_visual_selected_indices() -> &'static HashSet<usize> {
    static EMPTY: OnceLock<HashSet<usize>> = OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

pub trait PanelComponent {
    fn draw(&self, _frame: &mut Frame, _area: Rect, _ctx: &PanelRenderContext<'_>) {}
}

pub struct PanelRenderContext<'a> {
    pub active_panel: SidePanel,
    pub search_query: Option<&'a str>,
    pub search_summary: Option<&'a str>,
    pub visual_selected_indices: &'a HashSet<usize>,
}

impl<'a> PanelRenderContext<'a> {
    pub fn empty_visual_selected_indices() -> &'static HashSet<usize> {
        empty_visual_selected_indices()
    }
}
