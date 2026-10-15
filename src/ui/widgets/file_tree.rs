use crate::git::FileStatus;
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};
use std::{collections::HashSet, sync::OnceLock};

fn empty_selected_indices() -> &'static HashSet<usize> {
    static EMPTY: OnceLock<HashSet<usize>> = OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};

fn status_icon(status: &FileTreeNodeStatus) -> &'static str {
    match status {
        FileTreeNodeStatus::Staged(s) => match s {
            FileStatus::New => "✚",
            FileStatus::Modified => "●",
            FileStatus::Deleted => "✖",
            FileStatus::Renamed => "➜",
            FileStatus::TypeChange => "◆",
        },
        FileTreeNodeStatus::Unstaged(s) => match s {
            FileStatus::New => "✚",
            FileStatus::Modified => "✎",
            FileStatus::Deleted => "✖",
            FileStatus::Renamed => "➜",
            FileStatus::TypeChange => "◆",
        },
        FileTreeNodeStatus::Untracked => "?",
        FileTreeNodeStatus::Directory => " ",
    }
}

fn status_color(status: &FileTreeNodeStatus, theme: &UiTheme) -> Color {
    match status {
        FileTreeNodeStatus::Staged(_) => theme.success,
        FileTreeNodeStatus::Unstaged(s) => match s {
            FileStatus::Modified => theme.warning,
            FileStatus::Deleted => theme.error,
            _ => theme.text_primary,
        },
        FileTreeNodeStatus::Untracked => theme.text_muted,
        FileTreeNodeStatus::Directory => theme.info,
    }
}

pub struct FileTree<'a> {
    nodes: &'a [FileTreeNode],
    block: Option<Block<'a>>,
    highlight_style: Style,
    selected_indices: &'a HashSet<usize>,
    search_query: Option<&'a str>,
    theme: UiTheme,
}

impl<'a> FileTree<'a> {
    pub fn new(nodes: &'a [FileTreeNode]) -> Self {
        Self {
            nodes,
            block: None,
            highlight_style: UiTheme::default().active_highlight(),
            selected_indices: empty_selected_indices(),
            search_query: None,
            theme: UiTheme::default(),
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    pub fn highlight_style(mut self, style: Style) -> Self {
        self.highlight_style = style;
        self
    }

    pub fn selected_indices(mut self, selected_indices: &'a HashSet<usize>) -> Self {
        self.selected_indices = selected_indices;
        self
    }

    pub fn search_query(mut self, query: Option<&'a str>) -> Self {
        self.search_query = query;
        self
    }

    fn render_node(
        node: &FileTreeNode,
        search_query: Option<&str>,
        theme: &UiTheme,
    ) -> ListItem<'static> {
        let indent = "  ".repeat(node.depth);
        let color = status_color(&node.status, theme);
        let base = Style::default().fg(color);

        let name = node
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| node.path.display().to_string());

        let line = if node.is_dir {
            let arrow = if node.is_expanded { "▼ " } else { "▶ " };
            let mut spans = vec![Span::raw(indent), Span::styled(arrow.to_string(), base)];
            let dir_name = format!("{}/", name);
            spans.extend(highlighted_spans(&dir_name, search_query, base));
            Line::from(spans)
        } else {
            let icon = status_icon(&node.status);
            let mut spans = vec![
                Span::raw(indent),
                Span::styled(icon.to_string(), base),
                Span::raw(" "),
            ];
            spans.extend(highlighted_spans(&name, search_query, base));
            Line::from(spans)
        };

        ListItem::new(line)
    }
}

impl<'a> StatefulWidget for FileTree<'a> {
    type State = ListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let inner = self.block.as_ref().map_or(area, |block| block.inner(area));
        let range = visible_range(self.nodes.len(), state, usize::from(inner.height));
        let items: Vec<ListItem> = self.nodes[range.clone()]
            .iter()
            .enumerate()
            .map(|(idx, node)| {
                let item = Self::render_node(node, self.search_query, &self.theme);
                if self.selected_indices.contains(&(range.start + idx)) {
                    item.style(Style::default().bg(self.theme.visual_selection_bg))
                } else {
                    item
                }
            })
            .collect();

        let mut list = List::new(items).highlight_style(self.highlight_style);

        if let Some(block) = self.block {
            list = list.block(block);
        }

        let mut viewport = ListState::default().with_selected(
            state
                .selected()
                .and_then(|index| index.checked_sub(range.start)),
        );
        StatefulWidget::render(list, area, buf, &mut viewport);
        if inner.height > 0 {
            *state.offset_mut() = range.start;
        }
    }
}

/// All tree rows have height one, so viewport bounds need no full-list scan.
fn visible_range(count: usize, state: &ListState, height: usize) -> std::ops::Range<usize> {
    if count == 0 || height == 0 {
        return 0..0;
    }
    let start = crate::ui::panels::selection::scroll_offset(
        state.offset(),
        state.selected(),
        count,
        height,
    );
    start..(start + height).min(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::LIST_SCROLL_PADDING;
    use ratatui::widgets::Borders;

    #[test]
    fn test_file_tree_viewport_matches_full_list_rendering() {
        let nodes: Vec<_> = (0..80)
            .map(|index| FileTreeNode {
                path: format!("file{index:03}.txt").into(),
                status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
                depth: index % 3,
                is_dir: false,
                is_expanded: false,
            })
            .collect();
        let visual = HashSet::from([0, 2, 35, 36, 79]);
        for height in [0, 1, 2, 3, 5, 15] {
            for offset in [0, 2, 36, 79, 100] {
                for selected in [None, Some(0), Some(1), Some(35), Some(79)] {
                    let area = Rect::new(0, 0, 45, height + 2);
                    let block = Block::default().borders(Borders::ALL).title("Files");
                    let mut expected = Buffer::empty(area);
                    let mut actual = Buffer::empty(area);
                    let mut expected_state = ListState::default()
                        .with_selected(selected)
                        .with_offset(offset);
                    let mut actual_state = expected_state;
                    if height > 0 {
                        *expected_state.offset_mut() = crate::ui::panels::selection::scroll_offset(
                            expected_state.offset(),
                            selected,
                            nodes.len(),
                            usize::from(height),
                        );
                    }
                    let widget = FileTree::new(&nodes)
                        .block(block.clone())
                        .search_query(Some("035"))
                        .selected_indices(&visual);
                    let items: Vec<_> = nodes
                        .iter()
                        .enumerate()
                        .map(|(index, node)| {
                            let item = FileTree::render_node(node, Some("035"), &widget.theme);
                            if visual.contains(&index) {
                                item.style(Style::default().bg(widget.theme.visual_selection_bg))
                            } else {
                                item
                            }
                        })
                        .collect();
                    StatefulWidget::render(
                        List::new(items)
                            .block(block)
                            .highlight_style(widget.highlight_style)
                            .scroll_padding(LIST_SCROLL_PADDING),
                        area,
                        &mut expected,
                        &mut expected_state,
                    );
                    StatefulWidget::render(widget, area, &mut actual, &mut actual_state);
                    assert_eq!(
                        actual, expected,
                        "height={height}, offset={offset}, selected={selected:?}"
                    );
                    assert_eq!(actual_state.offset(), expected_state.offset());
                }
            }
        }
    }

    #[test]
    fn test_file_tree_viewport_is_bounded_for_large_lists() {
        let state = ListState::default().with_selected(Some(99_999));
        let range = visible_range(100_000, &state, 20);
        assert_eq!(range, 99_980..100_000);
    }
}
