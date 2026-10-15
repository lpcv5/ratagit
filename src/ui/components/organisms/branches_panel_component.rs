use super::props::BranchesPanelProps;
use super::{draw_commits_panel, empty_list_item};
use crate::ui::components::atoms::{badge, Tone};
use crate::ui::components::molecules::card;
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use crate::ui::LIST_SCROLL_PADDING;
use ratatui::{
    layout::Rect,
    style::Style,
    text::Line,
    widgets::{List, ListItem},
    Frame,
};

pub fn draw_branches_panel(
    frame: &mut Frame,
    area: Rect,
    props: BranchesPanelProps<'_>,
    viewport: &mut crate::ui::panels::selection::BranchesViewport,
) {
    let (header, rows) = match props {
        BranchesPanelProps::Commits { props, view_id } => {
            draw_commits_panel(frame, area, props, viewport.commits(view_id));
            return;
        }
        BranchesPanelProps::List { header, rows } => (header, rows),
    };
    let theme = UiTheme::default();
    let items: Vec<ListItem> = if rows.is_empty() {
        empty_list_item("No branches")
    } else {
        rows.iter()
            .map(|branch| {
                let (prefix, color) = if branch.is_current {
                    ("* ", theme.accent)
                } else {
                    ("  ", theme.text_primary)
                };
                let text = format!("{}{}", prefix, branch.name);
                ListItem::new(Line::from(highlighted_spans(
                    &text,
                    header.search_query,
                    Style::default().fg(color),
                )))
            })
            .collect()
    };
    let list = List::new(items)
        .block(card(&theme, &header.title, header.is_active).title_top(
            Line::from(badge(rows.len().to_string(), Tone::Neutral, &theme)).right_aligned(),
        ))
        .scroll_padding(LIST_SCROLL_PADDING)
        .highlight_style(theme.highlight_for(header.is_active));
    crate::ui::panels::selection::prepare(
        &mut viewport.list,
        header.selected,
        rows.len(),
        usize::from(area.height.saturating_sub(2)),
    );
    frame.render_stateful_widget(list, area, &mut viewport.list);
}

impl DynamicPanel for BranchesPanelProps<'_> {
    fn default_height_percent(&self) -> u16 {
        25
    }
    /// Branches panel does not expand on focus; content rarely overflows.
    fn focused_height_percent(&self) -> u16 {
        25
    }
    /// Use usize::MAX so should_expand always returns false.
    fn expand_threshold(&self) -> usize {
        usize::MAX
    }
    fn min_height(&self) -> u16 {
        3
    }
}
