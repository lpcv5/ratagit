use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::app::CachedData;
use crate::app::AppState;
use crate::app::events::{AppEvent, GitEvent, ModalEvent};
use crate::backend::git_ops::StatusEntry;
use crate::components::core::{
    build_tree_from_paths, ActionMultiplicity, GitFileStatus, SelectableList, TreeNode, TreePanel,
};
use crate::components::component_v2::ComponentV2;

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

    /// Temporary bridge method for old renderer (will be removed when renderer migrates to ComponentV2)
    pub fn render(
        &mut self,
        frame: &mut ratatui::Frame,
        area: ratatui::layout::Rect,
        is_focused: bool,
        data: &CachedData,
    ) {
        if data.files.is_empty() {
            SelectableList::render_empty(frame, area, "Files", is_focused);
            return;
        }

        self.tree.render_old(frame, area, is_focused, data);
    }
}

impl Default for FileListPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentV2 for FileListPanel {
    fn handle_key_event(&mut self, key: KeyEvent, _state: &AppState) -> AppEvent {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.tree.select_next();
                AppEvent::SelectionChanged
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.tree.select_previous();
                AppEvent::SelectionChanged
            }
            KeyCode::Char(' ') => AppEvent::Git(GitEvent::ToggleStageFile),
            KeyCode::Char('A') => AppEvent::Git(GitEvent::StageAll),
            KeyCode::Enter => AppEvent::ActivatePanel,
            KeyCode::Char('d') => AppEvent::Git(GitEvent::DiscardSelected),
            KeyCode::Char('i') => AppEvent::Git(GitEvent::IgnoreSelected),
            KeyCode::Char('r') => AppEvent::Modal(ModalEvent::ShowRenameDialog),
            KeyCode::Char('c') => AppEvent::Modal(ModalEvent::ShowCommitDialog),
            KeyCode::Char('s') => AppEvent::Git(GitEvent::StashSelected),
            KeyCode::Char('a') => AppEvent::Git(GitEvent::AmendCommit),
            KeyCode::Char('R') => AppEvent::Modal(ModalEvent::ShowResetMenu),
            _ => AppEvent::None,
        }
    }

    fn render(&self, _area: Rect, _buf: &mut Buffer, _state: &AppState) {
        // Render implementation will be added when ComponentV2 is fully integrated
        // For now, this is a stub to satisfy the trait
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_file_panel_component_v2() {
        let mut panel = FileListPanel::new();
        let state = mock_state();

        // Test navigation keys return SelectionChanged
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        // Test space key returns ToggleStageFile
        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::ToggleStageFile));

        // Test 'A' key returns StageAll
        let key = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::StageAll));

        // Test Enter key returns ActivatePanel
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::ActivatePanel);

        // Test 'd' key returns DiscardSelected
        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::DiscardSelected));

        // Test 'i' key returns IgnoreSelected
        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::IgnoreSelected));

        // Test 'r' key returns ShowRenameDialog
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Modal(ModalEvent::ShowRenameDialog));

        // Test 'c' key returns ShowCommitDialog
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Modal(ModalEvent::ShowCommitDialog));

        // Test 's' key returns StashSelected
        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::StashSelected));

        // Test 'a' key returns AmendCommit
        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::AmendCommit));

        // Test 'R' key returns ShowResetMenu
        let key = KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Modal(ModalEvent::ShowResetMenu));

        // Test unknown key returns None
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::None);
    }

    fn mock_state() -> crate::app::AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        crate::app::AppState::new(cmd_tx, event_rx)
    }

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
}
