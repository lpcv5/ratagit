use crate::git::{FileEntry, FileStatus};
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
};

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
            FileStatus::TypeChange => "T",
        },
        FileTreeNodeStatus::Unstaged(s) => match s {
            FileStatus::New => "✚",
            FileStatus::Modified => "✎",
            FileStatus::Deleted => "✖",
            FileStatus::Renamed => "➜",
            FileStatus::TypeChange => "T",
        },
        FileTreeNodeStatus::Untracked => "?",
        FileTreeNodeStatus::Directory => " ",
    }
}

fn status_color(status: &FileTreeNodeStatus) -> Color {
    match status {
        FileTreeNodeStatus::Staged(_) => Color::Green,
        FileTreeNodeStatus::Unstaged(s) => match s {
            FileStatus::New => Color::Green,
            FileStatus::Modified => Color::Yellow,
            FileStatus::Deleted => Color::Red,
            FileStatus::Renamed => Color::Magenta,
            FileStatus::TypeChange => Color::Cyan,
        },
        FileTreeNodeStatus::Untracked => Color::Gray,
        FileTreeNodeStatus::Directory => Color::Blue,
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
    nodes: Vec<FileTreeNode>,
    block: Option<Block<'a>>,
    highlight_style: Style,
}

impl<'a> FileTree<'a> {
    pub fn new(nodes: Vec<FileTreeNode>) -> Self {
        Self {
            nodes,
            block: None,
            highlight_style: Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD),
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
            files.push((f.path.clone(), FileTreeNodeStatus::Unstaged(f.status.clone())));
        }
        for f in untracked {
            files.push((f.path.clone(), FileTreeNodeStatus::Untracked));
        }
        for f in staged {
            files.push((f.path.clone(), FileTreeNodeStatus::Staged(f.status.clone())));
        }
        files
    }

    fn render_node(node: &FileTreeNode) -> ListItem<'static> {
        let indent = "  ".repeat(node.depth);
        let color = status_color(&node.status);

        let name = node
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| node.path.display().to_string());

        let line = if node.is_dir {
            let arrow = if node.is_expanded { "▼ " } else { "▶ " };
            Line::from(vec![
                Span::raw(indent),
                Span::styled(format!("{}{}/", arrow, name), Style::default().fg(color)),
            ])
        } else {
            let icon = status_icon(&node.status);
            Line::from(vec![
                Span::raw(indent),
                Span::styled(icon, Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(name, Style::default().fg(color)),
            ])
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
            if parent == Path::new("") { break; }
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
                &child_dir, files, all_dirs, expanded, depth + 1,
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
        let items: Vec<ListItem> = self.nodes.iter().map(Self::render_node).collect();

        let mut list = List::new(items).highlight_style(self.highlight_style);

        if let Some(block) = self.block {
            list = list.block(block);
        }

        StatefulWidget::render(list, area, buf, &mut state.list_state);
    }
}
