use crate::app::{CommitsPanelState, SidePanel};
use crate::git::{CommitSyncState, GraphCell};
use crate::ui::components::organisms::{
    empty_list_item, title_with_search, CommitsPanelViewState, CommitsTreeViewState,
    PanelRenderContext,
};
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};
use std::collections::HashSet;

pub fn draw_commits_panel(
    frame: &mut Frame,
    area: Rect,
    state: &CommitsPanelState,
    ctx: &PanelRenderContext<'_>,
) {
    let view = view_state_from_shell(state);
    draw_commits_panel_view(frame, area, &view, ctx);
}

pub fn draw_commits_panel_view(
    frame: &mut Frame,
    area: Rect,
    view: &CommitsPanelViewState,
    ctx: &PanelRenderContext<'_>,
) {
    let theme = UiTheme::default();
    let is_active = ctx.active_panel == SidePanel::Commits;
    let highlighted_oids = if ctx.highlighted_oids.is_empty() {
        &view.highlighted_oids
    } else {
        ctx.highlighted_oids
    };

    let items: Vec<ListItem> = if view.items.is_empty() {
        empty_list_item("No commits")
    } else {
        view.items
            .iter()
            .map(|c| {
                let g_spans = graph_spans(&c.graph, hash_color(c.sync_state), highlighted_oids);
                let hash_spans = highlighted_spans(
                    &c.oid[..7.min(c.oid.len())],
                    ctx.search_query,
                    Style::default().fg(hash_color(c.sync_state)),
                );
                let author_short = author_initials(&c.author);
                let author_spans = highlighted_spans(
                    &author_short,
                    ctx.search_query,
                    Style::default().fg(theme.text_muted),
                );
                let message_spans = highlighted_spans(
                    &c.message,
                    ctx.search_query,
                    Style::default().fg(Color::White),
                );

                let mut spans = Vec::with_capacity(
                    g_spans.len() + hash_spans.len() + author_spans.len() + message_spans.len() + 3,
                );
                spans.extend(g_spans);
                spans.push(Span::raw(" "));
                spans.extend(hash_spans);
                spans.push(Span::raw(" "));
                spans.extend(author_spans);
                spans.push(Span::raw(" "));
                spans.extend(message_spans);
                ListItem::new(Line::from(spans))
            })
            .collect()
    };

    let mut title = if let Some(title_override) = ctx.panel_title_override {
        title_override.to_string()
    } else if view.tree_mode.active {
        if let Some(ref oid) = view.tree_mode.selected_source {
            format!("Commit Files {} [Esc Back]", &oid[..oid.len().min(7)])
        } else {
            "Commit Files [Esc Back]".to_string()
        }
    } else {
        "Commits".to_string()
    };
    title = title_with_search(&title, ctx.search_summary);

    let mut list_state = ListState::default();
    list_state.select(view.selected_index);

    render_revision_tree_panel(
        frame,
        area,
        RevisionTreePanelProps {
            title: &title,
            is_active,
            tree_mode: view.tree_mode.active,
            tree_nodes: &view.tree_mode.nodes,
            tree_search_query: if view.tree_mode.active {
                ctx.search_query
            } else {
                None
            },
            list_items: items,
            list_state,
        },
    );
}

pub fn view_state_from_shell(state: &CommitsPanelState) -> CommitsPanelViewState {
    CommitsPanelViewState {
        selected_index: state.panel.list_state.selected(),
        items: state.items.clone(),
        tree_mode: CommitsTreeViewState {
            active: state.tree_mode.active,
            selected_source: state.tree_mode.selected_source.clone(),
            nodes: state.tree_mode.nodes.clone(),
        },
        highlighted_oids: state.highlighted_oids.clone(),
    }
}

impl DynamicPanel for CommitsPanelState {
    fn default_height_percent(&self) -> u16 {
        40
    }
    fn focused_height_percent(&self) -> u16 {
        50
    }
    fn expand_threshold(&self) -> usize {
        10
    }
    fn min_height(&self) -> u16 {
        3
    }
}

fn graph_spans(
    cells: &[GraphCell],
    color: Color,
    highlighted_oids: &HashSet<String>,
) -> Vec<Span<'static>> {
    let has_highlight = !highlighted_oids.is_empty();
    let mut spans = Vec::new();
    for cell in cells {
        let cell_color = if has_highlight {
            let matches_highlight = if !cell.pipe_oids.is_empty() {
                cell.pipe_oids
                    .iter()
                    .any(|oid| highlighted_oids.contains(oid))
            } else {
                cell.pipe_oid
                    .as_ref()
                    .map(|oid| highlighted_oids.contains(oid))
                    .unwrap_or(false)
            };
            if matches_highlight {
                color
            } else {
                Color::DarkGray
            }
        } else {
            color
        };
        spans.push(Span::styled(
            cell.text.clone(),
            Style::default().fg(cell_color),
        ));
    }
    spans
}

fn hash_color(sync_state: CommitSyncState) -> Color {
    match sync_state {
        CommitSyncState::DefaultBranch => Color::Green,
        CommitSyncState::RemoteBranch => Color::Yellow,
        CommitSyncState::LocalOnly => Color::Red,
    }
}

fn author_initials(author: &str) -> String {
    let words: Vec<&str> = author
        .split_whitespace()
        .filter(|word| !word.is_empty())
        .collect();

    if words.len() >= 2 {
        let mut initials = String::new();
        for word in words.iter().take(2) {
            if let Some(ch) = word.chars().find(|c| c.is_alphanumeric()) {
                initials.extend(ch.to_uppercase());
            }
        }
        if !initials.is_empty() {
            return initials;
        }
    }

    if let Some(word) = words.first() {
        let mut initials = String::new();
        for ch in word.chars().filter(|c| c.is_alphanumeric()).take(2) {
            initials.extend(ch.to_uppercase());
        }
        if !initials.is_empty() {
            return initials;
        }
    }

    "--".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::CommitInfo;
    use pretty_assertions::assert_eq;

    #[test]
    fn view_state_from_shell_projects_selection_without_shell_types() {
        let mut state = CommitsPanelState {
            items: vec![CommitInfo {
                oid: "abc123".to_string(),
                message: "test commit".to_string(),
                author: "tester".to_string(),
                graph: vec![GraphCell {
                    text: "●".to_string(),
                    lane: 0,
                    pipe_oid: None,
                    pipe_oids: vec![],
                }],
                time: "2026-04-11 00:00".to_string(),
                parent_count: 1,
                sync_state: CommitSyncState::DefaultBranch,
                parent_oids: vec![],
            }],
            ..Default::default()
        };
        state.panel.list_state.select(Some(0));

        let view = view_state_from_shell(&state);
        assert_eq!(view.selected_index, Some(0));
        assert_eq!(view.items.len(), 1);
    }
}
