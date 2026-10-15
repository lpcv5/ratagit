use super::empty_list_item;
use super::props::StashPanelProps;
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use ratatui::{layout::Rect, style::Style, text::Line, widgets::ListItem, Frame};

pub fn draw_stash_panel(
    frame: &mut Frame,
    area: Rect,
    props: StashPanelProps<'_>,
    viewport: &mut crate::ui::panels::selection::RevisionViewport,
) {
    let query = props.header.search_query;
    let content = props.content.map(|rows| {
        if rows.is_empty() {
            return empty_list_item("No stashes");
        }
        rows.iter()
            .map(|stash| {
                let text = format!("stash@{{{}}} {}", stash.index, stash.message);
                ListItem::new(Line::from(highlighted_spans(
                    &text,
                    query,
                    Style::default().fg(UiTheme::default().warning),
                )))
            })
            .collect()
    });
    render_revision_tree_panel(
        frame,
        area,
        RevisionTreePanelProps {
            header: props.header,
            content,
        },
        viewport,
    );
}

impl DynamicPanel for StashPanelProps<'_> {
    /// Stash collapses to min_height when unfocused; return 0 so the layout
    /// falls back to min_height directly.
    fn default_height_percent(&self) -> u16 {
        0
    }
    fn focused_height_percent(&self) -> u16 {
        25
    }
    /// Expand as soon as there is more than one stash entry.
    fn expand_threshold(&self) -> usize {
        1
    }
    fn min_height(&self) -> u16 {
        3
    }
}
