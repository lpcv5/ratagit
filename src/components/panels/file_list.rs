use std::collections::HashMap;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use crate::app::events::{AppEvent, GitEvent, ModalEvent};
use crate::app::AppState;
use crate::app::CachedData;
use crate::backend::git_ops::StatusEntry;
use crate::components::component_v2::ComponentV2;
use crate::components::core::{
    build_tree_from_paths, GitFileStatus, SelectableList, TreeNode, TreePanel,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FileViewMode {
    Tree,
    Flat,
}

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

/// 构建平铺文件节点（不含目录）
fn build_flat_file_list(files: &[StatusEntry]) -> Vec<TreeNode> {
    let mut nodes: Vec<TreeNode> = files
        .iter()
        .map(|entry| {
            let mut node = TreeNode::new(
                entry.path.clone(),
                entry.path.clone(),
                false,
                0,
                Some(status_entry_to_git_status(entry)),
            );
            node.is_staged = entry.is_staged;
            node
        })
        .collect();

    nodes.sort_by(|a, b| a.path.cmp(&b.path));
    nodes
}

/// 文件树面板组件（使用 Tree 组件）
pub struct FileListPanel {
    tree: TreePanel,
    view_mode: FileViewMode,
    files_cache: Vec<StatusEntry>,
}

impl FileListPanel {
    pub fn new() -> Self {
        let tree = TreePanel::new("Files".to_string(), Vec::new(), true);
        Self {
            tree,
            view_mode: FileViewMode::Tree,
            files_cache: Vec::new(),
        }
    }

    #[allow(dead_code)] // Reserved for future use
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

    pub fn handle_escape(&mut self) -> AppEvent {
        if self.tree.clear_multi_select_if_active() {
            AppEvent::SelectionChanged
        } else {
            AppEvent::None
        }
    }

    /// 获取当前选中的文件节点
    #[allow(dead_code)]
    pub fn selected_node(&self) -> Option<&TreeNode> {
        self.tree.selected_node()
    }

    /// 更新文件数据
    pub fn update_files(&mut self, files: &[StatusEntry]) {
        self.files_cache = files.to_vec();
        self.rebuild_nodes();
    }

    fn rebuild_nodes(&mut self) {
        let nodes = match self.view_mode {
            FileViewMode::Tree => build_file_tree(&self.files_cache),
            FileViewMode::Flat => build_flat_file_list(&self.files_cache),
        };
        self.tree.update_nodes(nodes);
    }

    fn toggle_view_mode(&mut self) {
        self.view_mode = match self.view_mode {
            FileViewMode::Tree => FileViewMode::Flat,
            FileViewMode::Flat => FileViewMode::Tree,
        };
        self.rebuild_nodes();
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
            KeyCode::Char('a') => AppEvent::Git(GitEvent::StageAll),
            KeyCode::Char('A') => AppEvent::Git(GitEvent::AmendCommit),
            KeyCode::Enter => {
                if self.view_mode == FileViewMode::Tree && self.tree.toggle_selected_dir() {
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::ActivatePanel
                }
            }
            KeyCode::Char('d') => AppEvent::Git(GitEvent::DiscardSelected),
            KeyCode::Char('i') => AppEvent::Git(GitEvent::IgnoreSelected),
            KeyCode::Char('c') => AppEvent::Modal(ModalEvent::ShowCommitDialog),
            KeyCode::Char('s') => AppEvent::Git(GitEvent::StashSelected),
            KeyCode::Char('D') => AppEvent::Modal(ModalEvent::ShowResetMenu),
            KeyCode::Char('`') => {
                self.toggle_view_mode();
                AppEvent::SelectionChanged
            }
            KeyCode::Char('-') => {
                self.tree.collapse_all_dirs();
                AppEvent::SelectionChanged
            }
            KeyCode::Char('=') => {
                self.tree.expand_all_dirs();
                AppEvent::SelectionChanged
            }
            KeyCode::Char('v') => {
                if self.tree.selected_node().is_some() {
                    self.tree.toggle_multi_select_at_cursor();
                    AppEvent::SelectionChanged
                } else {
                    AppEvent::None
                }
            }
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
    fn test_file_panel_keymap_core_actions() {
        let mut panel = FileListPanel::new();
        let state = mock_state();

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

        let key = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::ToggleStageFile));

        let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::StageAll));

        let key = KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::AmendCommit));

        let key = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::DiscardSelected));

        let key = KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::IgnoreSelected));

        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Modal(ModalEvent::ShowCommitDialog));

        let key = KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Git(GitEvent::StashSelected));

        let key = KeyEvent::new(KeyCode::Char('D'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::Modal(ModalEvent::ShowResetMenu));

        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::None);

        let key = KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key, &state);
        assert_eq!(event, AppEvent::None);
    }

    #[test]
    fn test_enter_toggles_directory_and_activates_file() {
        let mut panel = FileListPanel::new();
        let state = mock_state();

        panel.update_files(&[
            StatusEntry {
                path: "README.md".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
            StatusEntry {
                path: "src/main.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
        ]);

        // src directory is selected at index 1 in tree mode
        panel.state_mut().select(Some(1));
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        // collapsed directory should not advance into child
        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(panel.selected_tree_node(), Some(("src".to_string(), true)),);

        // expand again and move into child
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_j, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert_eq!(
            panel.selected_tree_node(),
            Some(("src/main.rs".to_string(), false)),
        );

        // file selection enters main panel
        panel.state_mut().select(Some(0));
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);
        assert_eq!(event, AppEvent::ActivatePanel);
    }

    #[test]
    fn test_tree_view_shortcuts_toggle_and_collapse_expand() {
        let mut panel = FileListPanel::new();
        let state = mock_state();
        panel.update_files(&[
            StatusEntry {
                path: "src/main.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
            StatusEntry {
                path: "src/lib.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
        ]);

        panel.state_mut().select(Some(0));
        let key_minus = KeyEvent::new(KeyCode::Char('-'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_minus, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let _ = panel.handle_key_event(key_j, &state);
        assert_eq!(panel.selected_tree_node(), Some(("src".to_string(), true)),);

        let key_equal = KeyEvent::new(KeyCode::Char('='), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_equal, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        let key_j = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        let _ = panel.handle_key_event(key_j, &state);
        assert_eq!(
            panel.selected_tree_node(),
            Some(("src/lib.rs".to_string(), false)),
        );

        let key_backtick = KeyEvent::new(KeyCode::Char('`'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_backtick, &state);
        assert_eq!(event, AppEvent::SelectionChanged);

        panel.state_mut().select(Some(0));
        let key_enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let event = panel.handle_key_event(key_enter, &state);
        assert_eq!(event, AppEvent::ActivatePanel);
    }

    #[test]
    fn test_file_panel_v_and_esc_toggle_multi_select() {
        let mut panel = FileListPanel::new();
        let state = mock_state();
        panel.update_files(&[StatusEntry {
            path: "src/main.rs".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        }]);
        panel.state_mut().select(Some(0));

        let key_v = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        let event = panel.handle_key_event(key_v, &state);
        assert_eq!(event, AppEvent::SelectionChanged);
        assert!(panel.is_multi_select_active());

        let event = panel.handle_escape();
        assert_eq!(event, AppEvent::SelectionChanged);
        assert!(!panel.is_multi_select_active());
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

    #[test]
    fn test_build_flat_file_list_with_statuses() {
        let files = vec![
            StatusEntry {
                path: "src/new_file.txt".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: true,
            },
            StatusEntry {
                path: "src/modified.rs".to_string(),
                is_staged: true,
                is_unstaged: false,
                is_untracked: false,
            },
        ];
        let nodes = build_flat_file_list(&files);

        assert_eq!(nodes.len(), 2);
        assert!(nodes.iter().all(|n| !n.is_dir));
        assert_eq!(nodes[0].path, "src/modified.rs");
        assert_eq!(nodes[1].path, "src/new_file.txt");
    }
}
