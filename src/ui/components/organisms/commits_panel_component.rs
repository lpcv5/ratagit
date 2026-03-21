use crate::app::{CommitsPanelState, SidePanel};
use crate::git::{CommitSyncState, GraphCell};
use crate::ui::components::organisms::{PanelComponent, PanelRenderContext};
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};

/// Render graph cells into colored Span list.
/// When `highlighted_oids` is non-empty, cells whose owners do not intersect it are dimmed.
fn graph_spans(
    cells: &[GraphCell],
    color: Color,
    highlighted_oids: &std::collections::HashSet<String>,
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

impl PanelComponent for CommitsPanelState {
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelRenderContext<'_>) {
        let theme = UiTheme::default();
        let is_active = ctx.active_panel == SidePanel::Commits;

        let items: Vec<ListItem> = if self.items.is_empty() {
            vec![ListItem::new("No commits").style(Style::default().fg(theme.text_muted))]
        } else {
            self.items
                .iter()
                .map(|c| {
                    let g_spans =
                        graph_spans(&c.graph, hash_color(c.sync_state), ctx.highlighted_oids);
                    let hash_spans = highlighted_spans(
                        &c.short_hash,
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
                        g_spans.len()
                            + hash_spans.len()
                            + author_spans.len()
                            + message_spans.len()
                            + 3,
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

        let mut title = if self.tree_mode.active {
            if let Some(ref oid) = self.tree_mode.selected_source {
                format!("Commit Files {} [Esc Back]", &oid[..oid.len().min(7)])
            } else {
                "Commit Files [Esc Back]".to_string()
            }
        } else {
            "Commits".to_string()
        };
        if let Some(search) = &ctx.search_summary {
            title = format!("{} [{}]", title, search);
        }

        render_revision_tree_panel(
            frame,
            area,
            RevisionTreePanelProps {
                title: &title,
                is_active,
                tree_mode: self.tree_mode.active,
                tree_nodes: &self.tree_mode.nodes,
                tree_search_query: if self.tree_mode.active {
                    ctx.search_query
                } else {
                    None
                },
                list_items: items,
                list_state: self.panel.list_state,
            },
        );
    }
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
    use std::collections::HashSet;

    #[test]
    fn test_graph_spans_prioritizes_highlight_when_cell_has_overlap_owner() {
        let cells = vec![
            GraphCell {
                text: "x".to_string(),
                lane: 0,
                pipe_oid: Some("other".to_string()),
                pipe_oids: vec!["other".to_string(), "focus".to_string()],
            },
            GraphCell {
                text: "y".to_string(),
                lane: 1,
                pipe_oid: Some("other".to_string()),
                pipe_oids: vec!["other".to_string()],
            },
        ];
        let highlighted = HashSet::from([String::from("focus")]);
        let spans = graph_spans(&cells, Color::Green, &highlighted);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].style.fg, Some(Color::Green));
        assert_eq!(spans[1].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_author_initials_prefers_first_two_words() {
        assert_eq!(author_initials("Linus Torvalds"), "LT");
    }

    #[test]
    fn test_author_initials_single_word_uses_first_two_chars() {
        assert_eq!(author_initials("alice"), "AL");
    }

    #[test]
    fn test_author_initials_empty_falls_back() {
        assert_eq!(author_initials("   "), "--");
    }
}
