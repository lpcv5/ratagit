use std::collections::HashMap;

use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{ListItem, ListState},
    Frame,
};

use crate::app::CachedData;
use crate::components::core::tree::{get_visible_nodes, GitFileStatus, TreeNode};
use crate::components::core::{
    accent_primary_color, accent_secondary_color, multi_select_row_style, theme, MultiSelectState,
    MultiSelectableList, SelectableList, LIST_HIGHLIGHT_SYMBOL,
};
use crate::components::Component;
use crate::components::Intent;

/// 文件树面板组件
pub struct TreePanel {
    state: ListState,
    title: String,
    /// 所有节点（包含折叠的）
    all_nodes: Vec<TreeNode>,
    /// 是否支持空格操作（如暂存文件）
    pub enable_space_action: bool,
    multi_select: MultiSelectState<String>,
}

impl TreePanel {
    pub fn new(title: String, nodes: Vec<TreeNode>, enable_space_action: bool) -> Self {
        let mut state = ListState::default();
        state.select(Some(0));
        Self {
            state,
            title,
            all_nodes: nodes,
            enable_space_action,
            multi_select: MultiSelectState::default(),
        }
    }

    pub fn state_mut(&mut self) -> &mut ListState {
        &mut self.state
    }

    #[allow(dead_code)]
    pub fn selected_index(&self) -> Option<usize> {
        self.state.selected()
    }

    /// 获取当前选中的节点
    #[allow(dead_code)]
    pub fn selected_node(&self) -> Option<&TreeNode> {
        let visible = self.visible_nodes();
        self.state
            .selected()
            .and_then(|idx| visible.get(idx).copied())
    }

    pub fn selected_targets(&self) -> Vec<(String, bool)> {
        if self.is_multi_active() {
            let visible_paths = self.visible_paths();
            let mut targets = Vec::new();
            for path in self.multi_selected_keys(&visible_paths) {
                if let Some(node) = self.all_nodes.iter().find(|n| n.path == path) {
                    if node.is_dir {
                        // 文件夹：收集该文件夹下所有在树中的文件
                        targets.extend(self.get_files_in_dir(&node.path));
                    } else {
                        // 文件：直接添加
                        targets.push((node.path.clone(), false));
                    }
                }
            }
            return targets;
        }

        // 单选模式
        if let Some(node) = self.selected_node() {
            if node.is_dir {
                // 文件夹：返回该文件夹下所有在树中的文件
                return self.get_files_in_dir(&node.path);
            } else {
                // 文件：直接返回
                return vec![(node.path.clone(), false)];
            }
        }

        vec![]
    }

    /// 获取指定文件夹下所有在树中的文件节点（递归包括子文件夹）
    fn get_files_in_dir(&self, dir_path: &str) -> Vec<(String, bool)> {
        let prefix = format!("{}/", dir_path);
        self.all_nodes
            .iter()
            .filter(|node| {
                !node.is_dir && node.path.starts_with(&prefix)
            })
            .map(|node| (node.path.clone(), false))
            .collect()
    }

    pub fn anchor_target(&self) -> Option<(String, bool)> {
        let visible_paths = self.visible_paths();
        let Some(path) = self.multi_anchor_key(&visible_paths) else {
            return self
                .selected_node()
                .map(|node| (node.path.clone(), node.is_dir));
        };

        self.all_nodes
            .iter()
            .find(|node| node.path == path)
            .map(|node| (node.path.clone(), node.is_dir))
    }

    /// 更新节点列表（保持展开状态和光标位置）
    #[allow(dead_code)]
    pub fn update_nodes(&mut self, nodes: Vec<TreeNode>) {
        // 在更新前，从旧的可见列表中获取当前选中的路径
        let old_visible = self.visible_nodes();
        let old_selected_idx = self.state.selected().unwrap_or(0);
        let selected_path = old_visible
            .get(old_selected_idx)
            .map(|node| node.path.clone());

        // 保存旧节点的展开状态
        let old_expanded_state: HashMap<String, bool> = self
            .all_nodes
            .iter()
            .filter(|n| n.is_dir)
            .map(|n| (n.path.clone(), n.is_expanded))
            .collect();

        // 更新节点，恢复展开状态
        self.all_nodes = nodes
            .into_iter()
            .map(|mut node| {
                if node.is_dir {
                    if let Some(&expanded) = old_expanded_state.get(&node.path) {
                        node.is_expanded = expanded;
                    }
                }
                node
            })
            .collect();

        self.exit_multi_select();

        // 在新的可见列表中查找原路径
        let visible = self.visible_nodes();
        let visible_len = visible.len();

        if visible_len == 0 {
            self.state.select(None);
            return;
        }

        // 尝试找回原路径
        if let Some(path) = selected_path {
            if let Some(idx) = visible.iter().position(|node| node.path == path) {
                self.state.select(Some(idx));
                return;
            }
        }

        // 找不到原路径，保持索引位置
        let current = self.state.selected().unwrap_or(0);
        self.state.select(Some(current.min(visible_len.saturating_sub(1))));
    }

    /// 切换目录的展开/折叠状态
    fn toggle_node(&mut self) {
        let visible = self.visible_nodes();
        if let Some(selected_idx) = self.state.selected() {
            if let Some(node) = visible.get(selected_idx) {
                if node.is_dir {
                    // 在 all_nodes 中找到对应的节点并切换
                    let path = node.path.clone();
                    if let Some(actual_node) = self.all_nodes.iter_mut().find(|n| n.path == path) {
                        actual_node.toggle_expanded();
                    }
                }
            }
        }

        if self.is_multi_active() {
            let visible_paths = self.visible_paths();
            self.refresh_multi_range(self.state.selected(), &visible_paths);
        }
    }

    /// 向前导航
    pub fn select_next(&mut self) {
        let visible_len = self.visible_nodes().len();
        if visible_len == 0 {
            self.state.select(None);
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let next = current.saturating_add(1).min(visible_len.saturating_sub(1));
        self.state.select(Some(next));

        if self.is_multi_active() {
            let visible_paths = self.visible_paths();
            self.refresh_multi_range(self.state.selected(), &visible_paths);
        }
    }

    /// 向后导航
    pub fn select_previous(&mut self) {
        let visible_len = self.visible_nodes().len();
        if visible_len == 0 {
            self.state.select(None);
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let prev = current.saturating_sub(1);
        self.state.select(Some(prev));

        if self.is_multi_active() {
            let visible_paths = self.visible_paths();
            self.refresh_multi_range(self.state.selected(), &visible_paths);
        }
    }

    pub fn clear_multi_select(&mut self) {
        self.exit_multi_select();
    }

    pub fn multi_select_active(&self) -> bool {
        self.is_multi_active()
    }

    fn visible_nodes(&self) -> Vec<&TreeNode> {
        get_visible_nodes(&self.all_nodes)
    }

    fn visible_paths(&self) -> Vec<String> {
        self.visible_nodes()
            .into_iter()
            .map(|node| node.path.clone())
            .collect()
    }
}

impl MultiSelectableList for TreePanel {
    type Key = String;

    fn multi_select_state(&self) -> &MultiSelectState<Self::Key> {
        &self.multi_select
    }

    fn multi_select_state_mut(&mut self) -> &mut MultiSelectState<Self::Key> {
        &mut self.multi_select
    }
}

impl Component for TreePanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
                KeyCode::Esc if self.is_multi_active() => {
                    self.exit_multi_select();
                    return Intent::None;
                }
                KeyCode::Char('v') if key.modifiers.is_empty() => {
                    let visible_paths = self.visible_paths();
                    self.toggle_multi_select(self.state.selected(), &visible_paths);
                    return Intent::None;
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.select_next();
                    return Intent::None;
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.select_previous();
                    return Intent::None;
                }
                KeyCode::Enter => {
                    self.toggle_node();
                    return Intent::None;
                }
                KeyCode::Char(' ') if key.modifiers.is_empty() && self.enable_space_action => {
                    return Intent::ToggleStageFile;
                }
                _ => {}
            }
        }

        Intent::None
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, is_focused: bool, _data: &CachedData) {
        let visible = self.visible_nodes();
        let multi_active = self.is_multi_active();
        let title = if multi_active {
            format!("{} · MULTI:{}", self.title, self.multi_selected_count())
        } else {
            self.title.clone()
        };

        if visible.is_empty() {
            SelectableList::render_empty(frame, area, &title, is_focused);
            return;
        }

        let items: Vec<ListItem<'_>> = visible
            .iter()
            .map(|node| {
                let indent = "  ".repeat(node.depth);

                let mut item = if node.is_dir {
                    // 目录节点：三角图标 + 目录名
                    let expand_icon = if node.is_expanded { "▼" } else { "▶" };
                    ListItem::new(Line::from(vec![
                        Span::styled(indent.clone(), Style::default()),
                        Span::styled(
                            format!("{} ", expand_icon),
                            Style::default().fg(accent_secondary_color()),
                        ),
                        Span::styled(
                            node.name.clone(),
                            Style::default().fg(accent_primary_color()),
                        ),
                    ]))
                } else {
                    // 文件节点：缩进 + 状态图标 + 文件名
                    // 如果文件是 staged，文件名显示为绿色
                    let status_span = if let Some(status) = node.status {
                        let color = match status {
                            GitFileStatus::Added => theme().git_added,
                            GitFileStatus::Modified => theme().git_modified,
                            GitFileStatus::Deleted => theme().git_deleted,
                            GitFileStatus::Renamed => theme().git_renamed,
                            GitFileStatus::Untracked => theme().git_untracked,
                            GitFileStatus::Unmodified => Color::Reset,
                        };
                        Span::styled(
                            format!("[{}] ", status.display_label()),
                            Style::default().fg(color),
                        )
                    } else {
                        Span::raw("    ")
                    };

                    // 如果文件是 staged，文件名显示为绿色
                    let name_span = if node.is_staged {
                        Span::styled(node.name.clone(), Style::default().fg(theme().git_added))
                    } else {
                        Span::styled(node.name.clone(), Style::default())
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(indent.clone(), Style::default()),
                        status_span,
                        name_span,
                    ]))
                };

                if multi_active && self.is_multi_selected_key(&node.path) {
                    item = item.style(multi_select_row_style());
                }

                item
            })
            .collect();

        let list = SelectableList::new(items, &title, is_focused, LIST_HIGHLIGHT_SYMBOL);
        let state = &mut self.state.clone();
        list.render(frame, area, state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_navigation_does_not_wrap_forward() {
        let nodes = vec![
            TreeNode::new("a".to_string(), "a".to_string(), false, 0, None),
            TreeNode::new("b".to_string(), "b".to_string(), false, 0, None),
            TreeNode::new("c".to_string(), "c".to_string(), false, 0, None),
        ];
        let mut panel = TreePanel::new("Files".to_string(), nodes, false);
        panel.state.select(Some(2));

        panel.select_next();
        assert_eq!(panel.state.selected(), Some(2));
    }

    #[test]
    fn tree_navigation_does_not_wrap_backward() {
        let nodes = vec![
            TreeNode::new("a".to_string(), "a".to_string(), false, 0, None),
            TreeNode::new("b".to_string(), "b".to_string(), false, 0, None),
        ];
        let mut panel = TreePanel::new("Files".to_string(), nodes, false);
        panel.state.select(Some(0));

        panel.select_previous();
        assert_eq!(panel.state.selected(), Some(0));
    }

    #[test]
    fn v_toggles_contiguous_multi_selection() {
        let nodes = vec![
            TreeNode::new("a".to_string(), "a".to_string(), false, 0, None),
            TreeNode::new("b".to_string(), "b".to_string(), false, 0, None),
            TreeNode::new("c".to_string(), "c".to_string(), false, 0, None),
        ];
        let mut panel = TreePanel::new("Files".to_string(), nodes, false);
        panel.state.select(Some(0));

        let enter_multi = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('v'),
            crossterm::event::KeyModifiers::NONE,
        ));
        panel.handle_event(&enter_multi, &CachedData::default());
        panel.select_next();
        panel.select_next();

        let selected = panel.selected_targets();
        assert_eq!(selected.len(), 3);
        assert!(selected.iter().any(|(path, _)| path == "a"));
        assert!(selected.iter().any(|(path, _)| path == "b"));
        assert!(selected.iter().any(|(path, _)| path == "c"));
    }

    #[test]
    fn esc_clears_multi_selection_only() {
        let nodes = vec![
            TreeNode::new("a".to_string(), "a".to_string(), false, 0, None),
            TreeNode::new("b".to_string(), "b".to_string(), false, 0, None),
        ];
        let mut panel = TreePanel::new("Files".to_string(), nodes, false);

        let enter_multi = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('v'),
            crossterm::event::KeyModifiers::NONE,
        ));
        panel.handle_event(&enter_multi, &CachedData::default());
        assert!(panel.is_multi_active());

        let esc = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Esc,
            crossterm::event::KeyModifiers::NONE,
        ));
        panel.handle_event(&esc, &CachedData::default());
        assert!(!panel.is_multi_active());
        assert_eq!(panel.selected_targets().len(), 1);
    }
}
