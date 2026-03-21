use crate::git::{FileEntry, FileStatus};
use crate::ui::highlight::highlighted_spans;
use crate::ui::theme::UiTheme;
use crate::ui::LIST_SCROLL_PADDING;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::OnceLock,
};

fn empty_selected_indices() -> &'static HashSet<usize> {
    static EMPTY: OnceLock<HashSet<usize>> = OnceLock::new();
    EMPTY.get_or_init(HashSet::new)
}

/// Documentation comment in English.
#[derive(Debug, Clone, PartialEq)]
pub enum FileTreeNodeStatus {
    Unstaged(FileStatus),
    Staged(FileStatus),
    Untracked,
    Directory,
}

/// Documentation comment in English.
#[derive(Debug, Clone)]
pub struct FileTreeNode {
    pub path: PathBuf,
    pub status: FileTreeNodeStatus,
    pub depth: usize,
    pub is_dir: bool,
    pub is_expanded: bool,
}

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

/// Documentation comment in English.
#[derive(Debug, Default)]
pub struct FileTreeState {
    pub list_state: ListState,
    #[allow(dead_code)]
    pub expanded_dirs: HashSet<PathBuf>,
}

#[allow(dead_code)]
impl FileTreeState {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.list_state.select(Some(0));
        s
    }

    pub fn toggle_dir(&mut self, path: &Path) {
        if self.expanded_dirs.contains(path) {
            self.expanded_dirs.remove(path);
        } else {
            self.expanded_dirs.insert(path.to_path_buf());
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.list_state.selected()
    }

    pub fn select(&mut self, idx: Option<usize>) {
        self.list_state.select(idx);
    }
}

/// Documentation comment in English.
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
            highlight_style: Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
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

    /// Documentation comment in English.
    #[allow(dead_code)]
    pub fn from_git_status(
        unstaged: &[FileEntry],
        untracked: &[FileEntry],
        staged: &[FileEntry],
    ) -> Vec<FileTreeNode> {
        let files = Self::collect_files(unstaged, untracked, staged);
        let all_dirs = collect_dirs(&files);
        let expanded = all_dirs.clone();
        build_subtree(Path::new(""), &files, &all_dirs, &expanded, 0)
    }

    /// Documentation comment in English.
    pub fn from_git_status_with_expanded(
        unstaged: &[FileEntry],
        untracked: &[FileEntry],
        staged: &[FileEntry],
        expanded: &HashSet<PathBuf>,
    ) -> Vec<FileTreeNode> {
        let files = Self::collect_files(unstaged, untracked, staged);
        let all_dirs = collect_dirs(&files);
        build_subtree(Path::new(""), &files, &all_dirs, expanded, 0)
    }

    fn collect_files(
        unstaged: &[FileEntry],
        untracked: &[FileEntry],
        staged: &[FileEntry],
    ) -> Vec<(PathBuf, FileTreeNodeStatus)> {
        let mut files = Vec::new();
        for f in unstaged {
            files.push((
                f.path.clone(),
                FileTreeNodeStatus::Unstaged(f.status.clone()),
            ));
        }
        for f in untracked {
            files.push((f.path.clone(), FileTreeNodeStatus::Untracked));
        }
        for f in staged {
            files.push((f.path.clone(), FileTreeNodeStatus::Staged(f.status.clone())));
        }
        files
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

/// Documentation comment in English.
fn collect_dirs(files: &[(PathBuf, FileTreeNodeStatus)]) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for (path, _) in files {
        let mut p = path.as_path();
        while let Some(parent) = p.parent() {
            if parent == Path::new("") {
                break;
            }
            dirs.insert(parent.to_path_buf());
            p = parent;
        }
    }
    dirs
}

/// Documentation comment in English.
fn build_subtree(
    dir: &Path,
    files: &[(PathBuf, FileTreeNodeStatus)],
    all_dirs: &HashSet<PathBuf>,
    expanded: &HashSet<PathBuf>,
    depth: usize,
) -> Vec<FileTreeNode> {
    let mut nodes = Vec::new();

    // Comment in English.
    let mut child_dirs: Vec<PathBuf> = all_dirs
        .iter()
        .filter(|d| d.parent() == Some(dir))
        .cloned()
        .collect();
    child_dirs.sort();

    // Comment in English.
    let mut child_files: Vec<&(PathBuf, FileTreeNodeStatus)> = files
        .iter()
        .filter(|(p, _)| p.parent() == Some(dir))
        .collect();
    child_files.sort_by(|a, b| a.0.cmp(&b.0));

    // Comment in English.
    for child_dir in child_dirs {
        let is_expanded = expanded.contains(&child_dir);
        nodes.push(FileTreeNode {
            path: child_dir.clone(),
            status: FileTreeNodeStatus::Directory,
            depth,
            is_dir: true,
            is_expanded,
        });
        if is_expanded {
            nodes.extend(build_subtree(
                &child_dir,
                files,
                all_dirs,
                expanded,
                depth + 1,
            ));
        }
    }

    // Comment in English.
    for (path, status) in child_files {
        nodes.push(FileTreeNode {
            path: path.clone(),
            status: status.clone(),
            depth,
            is_dir: false,
            is_expanded: false,
        });
    }

    nodes
}

impl<'a> StatefulWidget for FileTree<'a> {
    type State = FileTreeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| {
                let item = Self::render_node(node, self.search_query, &self.theme);
                if self.selected_indices.contains(&idx) {
                    item.style(Style::default().bg(self.theme.visual_selection_bg))
                } else {
                    item
                }
            })
            .collect();

        let mut list = List::new(items)
            .scroll_padding(LIST_SCROLL_PADDING)
            .highlight_style(self.highlight_style);

        if let Some(block) = self.block {
            list = list.block(block);
        }

        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }
}
