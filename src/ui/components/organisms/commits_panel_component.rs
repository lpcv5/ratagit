use super::empty_list_item;
use super::props::{CommitsPanelProps, GraphCellProps};
use crate::ui::components::atoms::Tone;
use crate::ui::highlight::highlighted_spans;
use crate::ui::panels::revision_tree_panel::{render_revision_tree_panel, RevisionTreePanelProps};
use crate::ui::theme::UiTheme;
use crate::ui::traits::DynamicPanel;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::ListItem,
    Frame,
};

pub fn draw_commits_panel(
    frame: &mut Frame,
    area: Rect,
    props: CommitsPanelProps<'_>,
    viewport: &mut crate::ui::panels::selection::RevisionViewport,
) {
    let theme = UiTheme::default();
    let query = props.header.search_query;
    let content = props.content.map(|rows| {
        if rows.is_empty() {
            return empty_list_item("No commits");
        }
        rows.iter()
            .map(|commit| {
                let color = hash_color(commit.tone);
                let mut spans = graph_spans(&commit.graph, color);
                spans.push(Span::raw(" "));
                spans.extend(highlighted_spans(
                    &commit.oid[..7.min(commit.oid.len())],
                    query,
                    Style::default().fg(color),
                ));
                spans.push(Span::raw(" "));
                spans.extend(highlighted_spans(
                    &author_initials(commit.author),
                    query,
                    Style::default().fg(theme.text_muted),
                ));
                spans.push(Span::raw(" "));
                spans.extend(highlighted_spans(
                    commit.message,
                    query,
                    Style::default().fg(theme.text_primary),
                ));
                ListItem::new(Line::from(spans))
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

impl DynamicPanel for CommitsPanelProps<'_> {
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

/// Apply the owner's graph emphasis using UI theme colors.
fn graph_spans(cells: &[GraphCellProps<'_>], color: Color) -> Vec<Span<'static>> {
    cells
        .iter()
        .map(|cell| {
            Span::styled(
                cell.text.to_owned(),
                Style::default().fg(if cell.dimmed {
                    UiTheme::default().text_muted
                } else {
                    color
                }),
            )
        })
        .collect()
}

fn hash_color(tone: Tone) -> Color {
    let theme = UiTheme::default();
    match tone {
        Tone::Success => theme.success,
        Tone::Warning => theme.warning,
        Tone::Danger => theme.error,
        Tone::Accent => theme.accent,
        Tone::Neutral => theme.text_muted,
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
    use pretty_assertions::assert_eq;

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
