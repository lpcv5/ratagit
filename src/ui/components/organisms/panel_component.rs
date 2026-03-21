use crate::app::SidePanel;
use ratatui::{layout::Rect, Frame};
use std::collections::HashSet;

pub trait PanelComponent {
    fn draw(&self, _frame: &mut Frame, _area: Rect, _ctx: &PanelRenderContext<'_>) {}
}

pub struct PanelRenderContext<'a> {
    pub active_panel: SidePanel,
    pub search_query: Option<&'a str>,
    pub search_summary: Option<String>,
    pub visual_selected_indices: HashSet<usize>,
}
