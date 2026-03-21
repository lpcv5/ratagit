use crate::app::{BranchesPanelState, SidePanel};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{List, ListItem},
    Frame,
};

impl PanelComponent for BranchesPanelState {
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext<'_>) {
        let theme = UiTheme::default();
        let is_active = ctx.active_panel == SidePanel::LocalBranches;

        let items: Vec<ListItem> = if self.items.is_empty() {
            vec![ListItem::new("No branches").style(Style::default().fg(theme.text_muted))]
        } else {
            self.items
                .iter()
                .map(|b| {
                    let (prefix, color) = if b.is_current {
                        ("* ", theme.accent)
                    } else {
                        ("  ", theme.text_primary)
                    };
                    let text = format!("{}{}", prefix, b.name);
                    let spans =
                        highlighted_spans(&text, ctx.search_query, Style::default().fg(color));
                    ListItem::new(Line::from(spans))
                })
                .collect()
        };

        let highlight = if is_active {
            theme.active_highlight()
        } else {
            theme.inactive_highlight()
        };
        let mut title = "Local Branches".to_string();
        if let Some(search) = &ctx.search_summary {
            title = format!("{} [{}]", title, search);
        }

        let list = List::new(items)
            .block(theme.panel_block(&title, is_active))
            .highlight_style(highlight);

        let mut state = self.panel.list_state;
        frame.render_stateful_widget(list, area, &mut state);
    }
}
