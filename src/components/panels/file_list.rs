use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::CachedData;
use crate::backend::git_ops::StatusEntry;
use crate::components::core::{
    build_tree_from_paths, GitFileStatus, SelectableList, TreeNode, TreePanel,
};
use crate::components::Component;
use crate::components::Intent;

/// 将 StatusEntry 转换为 GitFileStatus
fn status_entry_to_git_status(entry: &StatusEntry) -> GitFileStatus {
    if entry.is_untracked {
        GitFileStatus::Untracked
    } else if entry.is_staged {
        GitFileStatus::Added
    } else if entry.is_unstaged {
        GitFileStatus::Modified
    } else {
        GitFileStatus::Unmodified
    }
}

/// 构建文件树节点
fn build_file_tree(files: &[StatusEntry]) -> Vec<TreeNode> {
    let paths: Vec<String> = files.iter().map(|f| f.path.clone()).collect();
    let status_map: HashMap<String, GitFileStatus> = files
        .iter()
        .map(|f| (f.path.clone(), status_entry_to_git_status(f)))
        .collect();
    build_tree_from_paths(&paths, Some(&status_map))
}

/// 文件树面板组件（使用 Tree 组件）
pub struct FileListPanel {
    tree: TreePanel,
}

impl FileListPanel {
    pub fn new() -> Self {
        let tree = TreePanel::new("Files".to_string(), Vec::new(), true);
        Self { tree }
    }

    pub fn state_mut(&mut self) -> &mut ratatui::widgets::ListState {
        self.tree.state_mut()
    }

    pub fn selected_index(&self) -> Option<usize> {
        self.tree.selected_index()
    }

    /// 获取当前选中的文件节点
    #[allow(dead_code)]
    pub fn selected_node(&self) -> Option<&TreeNode> {
        self.tree.selected_node()
    }

    /// 更新文件数据
    pub fn update_files(&mut self, files: &[StatusEntry]) {
        let nodes = build_file_tree(files);
        self.tree.update_nodes(nodes);
    }
}

impl Default for FileListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for FileListPanel {
    fn handle_event(&mut self, event: &Event, data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            // Enter 键：如果是目录则展开/折叠，如果是文件则激活
            if key.code == KeyCode::Enter {
                if let Some(node) = self.tree.selected_node() {
                    if node.is_dir {
                        // 目录：切换展开/折叠
                        self.tree.handle_event(event, data);
                        return Intent::None;
                    } else {
                        // 文件：激活
                        return Intent::ActivatePanel;
                    }
                }
                return Intent::None;
            }

            // 空格键：暂存文件
            if key.code == KeyCode::Char(' ') && key.modifiers.is_empty() {
                return Intent::ToggleStageFile;
            }
        }

        // 其他按键委派给 tree 处理
        self.tree.handle_event(event, data)
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        if data.files.is_empty() {
            SelectableList::render_empty(frame, area, "Files", is_focused);
            return;
        }

        self.tree.render(frame, area, is_focused, data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_untracked_file_status() {
        let entry = StatusEntry {
            path: "new_file.txt".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: true,
        };
        assert_eq!(status_entry_to_git_status(&entry), GitFileStatus::Untracked);
    }

    #[test]
    fn test_modified_file_status() {
        let entry = StatusEntry {
            path: "modified.rs".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        };
        assert_eq!(status_entry_to_git_status(&entry), GitFileStatus::Modified);
    }

    #[test]
    fn test_staged_file_status() {
        let entry = StatusEntry {
            path: "staged.rs".to_string(),
            is_staged: true,
            is_unstaged: false,
            is_untracked: false,
        };
        assert_eq!(status_entry_to_git_status(&entry), GitFileStatus::Added);
    }

    #[test]
    fn test_build_file_tree_with_statuses() {
        let files = vec![
            StatusEntry {
                path: "src/new_file.txt".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: true,
            },
            StatusEntry {
                path: "src/modified.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
        ];
        let nodes = build_file_tree(&files);

        // 找到 untracked 文件节点
        let untracked = nodes.iter().find(|n| n.path == "src/new_file.txt").unwrap();
        assert_eq!(untracked.status, Some(GitFileStatus::Untracked));

        // 找到 modified 文件节点
        let modified = nodes.iter().find(|n| n.path == "src/modified.rs").unwrap();
        assert_eq!(modified.status, Some(GitFileStatus::Modified));
    }
}
