use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::CachedData;
use crate::backend::git_ops::StatusEntry;
use crate::components::core::{
    build_tree_from_paths, ActionMultiplicity, GitFileStatus, SelectableList, TreeNode, TreePanel,
};
use crate::components::Component;
use crate::components::Intent;

/// 将 StatusEntry 转换为 GitFileStatus
/// 优先显示 unstaged 状态，如果没有则显示 staged 状态
fn status_entry_to_git_status(entry: &StatusEntry) -> GitFileStatus {
    if entry.is_untracked {
        GitFileStatus::Untracked
    } else if entry.is_unstaged {
        // 有 unstaged 修改，显示 Modified（即使也被 staged 了）
        GitFileStatus::Modified
    } else if entry.is_staged {
        // 只有 staged 修改，显示 Modified（而不是 Added）
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
    let staged_map: HashMap<String, bool> = files
        .iter()
        .map(|f| (f.path.clone(), f.is_staged))
        .collect();

    let mut nodes = build_tree_from_paths(&paths, Some(&status_map));

    // 设置 is_staged 标志
    for node in &mut nodes {
        if let Some(&is_staged) = staged_map.get(&node.path) {
            node.is_staged = is_staged;
        }
    }

    nodes
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

    pub fn selected_tree_node(&self) -> Option<(String, bool)> {
        self.tree
            .selected_node()
            .map(|node| (node.path.clone(), node.is_dir))
    }

    pub fn selected_tree_targets(&self) -> Vec<(String, bool)> {
        self.tree.selected_targets()
    }

    pub fn anchor_tree_target(&self) -> Option<(String, bool)> {
        self.tree.anchor_target()
    }

    pub fn is_multi_select_active(&self) -> bool {
        self.tree.multi_select_active()
    }

    pub fn clear_multi_select(&mut self) {
        self.tree.clear_multi_select();
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

            // New keybindings
            match key.code {
                KeyCode::Char('a') => return Intent::StageAll,
                KeyCode::Char('A') => return Intent::AmendCommit,
                KeyCode::Char('d') => return Intent::DiscardSelected,
                KeyCode::Char('D') => return Intent::ShowResetMenu,
                KeyCode::Char('s') => return Intent::StashSelected,
                _ => {}
            }

            // Enter 键：目录展开/折叠后也刷新右侧详情；文件上 Enter 仅作为手动刷新。
            if key.code == KeyCode::Enter {
                if let Some(node) = self.tree.selected_node() {
                    if node.is_dir {
                        self.tree.handle_event(event, data);
                        return Intent::RefreshPanelDetail;
                    } else {
                        return Intent::RefreshPanelDetail;
                    }
                }
                return Intent::None;
            }

            // 空格键：仅文件支持暂存操作
            if key.code == KeyCode::Char(' ') && key.modifiers.is_empty() {
                let stage_action = ActionMultiplicity::BatchCapable;
                let has_file_target = self
                    .tree
                    .selected_targets()
                    .iter()
                    .any(|(_, is_dir)| !*is_dir);
                if has_file_target && stage_action == ActionMultiplicity::BatchCapable {
                    return Intent::ToggleStageFile;
                }
                return Intent::None;
            }
        }

        // 其他按键委派给 tree 处理；若光标变化则刷新详情
        let before_targets = self.tree.selected_targets();
        let before_multi = self.tree.multi_select_active();
        let intent = self.tree.handle_event(event, data);
        if !matches!(intent, Intent::None) {
            return intent;
        }

        let after_targets = self.tree.selected_targets();
        let after_multi = self.tree.multi_select_active();
        if before_targets != after_targets || before_multi != after_multi {
            return Intent::RefreshPanelDetail;
        }

        Intent::None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
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
        // Staged 文件显示为 Modified（保持 M 图标）
        assert_eq!(status_entry_to_git_status(&entry), GitFileStatus::Modified);
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

    #[test]
    fn tree_navigation_refreshes_panel_detail() {
        let files = vec![
            StatusEntry {
                path: "a.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
            StatusEntry {
                path: "b.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
        ];

        let mut panel = FileListPanel::new();
        panel.update_files(&files);

        let event = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        let intent = panel.handle_event(&event, &CachedData::default());
        assert!(matches!(intent, Intent::RefreshPanelDetail));
    }
}
