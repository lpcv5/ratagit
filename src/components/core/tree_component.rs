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
use crate::components::core::SelectableList;
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
        let visible = get_visible_nodes(&self.all_nodes);
        self.state
            .selected()
            .and_then(|idx| visible.get(idx).copied())
    }

    /// 更新节点列表（保持当前选择位置）
    #[allow(dead_code)]
    pub fn update_nodes(&mut self, nodes: Vec<TreeNode>) {
        self.all_nodes = nodes;
        // 同步选择状态
        let visible_len = get_visible_nodes(&self.all_nodes).len();
        if visible_len == 0 {
            self.state.select(None);
        } else {
            let current = self.state.selected().unwrap_or(0);
            self.state
                .select(Some(current.min(visible_len.saturating_sub(1))));
        }
    }

    /// 切换目录的展开/折叠状态
    fn toggle_node(&mut self) {
        let visible = get_visible_nodes(&self.all_nodes);
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
    }

    /// 向前导航
    pub fn select_next(&mut self) {
        let visible_len = get_visible_nodes(&self.all_nodes).len();
        if visible_len == 0 {
            self.state.select(None);
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let next = if current + 1 >= visible_len {
            0
        } else {
            current + 1
        };
        self.state.select(Some(next));
    }

    /// 向后导航
    pub fn select_previous(&mut self) {
        let visible_len = get_visible_nodes(&self.all_nodes).len();
        if visible_len == 0 {
            self.state.select(None);
            return;
        }
        let current = self.state.selected().unwrap_or(0);
        let prev = if current == 0 {
            visible_len - 1
        } else {
            current - 1
        };
        self.state.select(Some(prev));
    }
}

impl Component for TreePanel {
    fn handle_event(&mut self, event: &Event, _data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            match key.code {
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

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, _data: &CachedData) {
        let visible = get_visible_nodes(&self.all_nodes);

        if visible.is_empty() {
            SelectableList::render_empty(frame, area, &self.title, is_focused);
            return;
        }

        let items: Vec<ListItem<'_>> = visible
            .iter()
            .map(|node| {
                let indent = "  ".repeat(node.depth);

                if node.is_dir {
                    // 目录节点：三角图标 + 目录名
                    let expand_icon = if node.is_expanded { "▼" } else { "▶" };
                    ListItem::new(Line::from(vec![
                        Span::styled(indent.clone(), Style::default()),
                        Span::styled(
                            format!("{} ", expand_icon),
                            Style::default().fg(Color::Yellow),
                        ),
                        Span::styled(node.name.clone(), Style::default().fg(Color::LightBlue)),
                    ]))
                } else {
                    // 文件节点：缩进 + 状态图标 + 文件名
                    let status_span = if let Some(status) = node.status {
                        let color = match status {
                            GitFileStatus::Added => Color::Green,
                            GitFileStatus::Modified => Color::Yellow,
                            GitFileStatus::Deleted => Color::Red,
                            GitFileStatus::Renamed => Color::LightCyan,
                            GitFileStatus::Untracked => Color::DarkGray,
                            GitFileStatus::Unmodified => Color::Reset,
                        };
                        Span::styled(
                            format!("[{}] ", status.display_label()),
                            Style::default().fg(color),
                        )
                    } else {
                        Span::raw("    ")
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(indent.clone(), Style::default()),
                        status_span,
                        Span::styled(node.name.clone(), Style::default()),
                    ]))
                }
            })
            .collect();

        let list = SelectableList::new(items, &self.title, is_focused, "> ");
        let state = &mut self.state.clone();
        list.render(frame, area, state);
    }
}
