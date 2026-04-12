use crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::app::CachedData;
use crate::components::core::TreePanel;
use crate::components::Component;
use crate::components::Intent;

/// Commit 文件树面板（包装 TreePanel）
pub struct CommitFilesPanel {
    tree: Option<TreePanel>,
    commit_id: String,
    commit_summary: String,
}

impl CommitFilesPanel {
    pub fn new() -> Self {
        Self {
            tree: None,
            commit_id: String::new(),
            commit_summary: String::new(),
        }
    }

    /// 更新文件树数据
    pub fn update_tree(&mut self, commit_id: String, commit_summary: String, tree: TreePanel) {
        self.commit_id = commit_id;
        self.commit_summary = commit_summary;
        self.tree = Some(tree);
    }

    #[allow(dead_code)]
    pub fn state_mut(&mut self) -> Option<&mut ratatui::widgets::ListState> {
        self.tree.as_mut().map(|t| t.state_mut())
    }

    #[allow(dead_code)]
    pub fn selected_index(&self) -> Option<usize> {
        self.tree.as_ref().and_then(|t| t.selected_index())
    }

    /// 清空树（当切换回 commit 列表时）
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.tree = None;
        self.commit_id.clear();
        self.commit_summary.clear();
    }
}

impl Default for CommitFilesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for CommitFilesPanel {
    fn handle_event(&mut self, event: &Event, data: &CachedData) -> Intent {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Intent::None;
            }

            if key.code == KeyCode::Esc {
                // ESC 返回 commits 面板
                return Intent::SwitchFocus(crate::app::Panel::Commits);
            }
        }

        // 委派给 tree 处理
        if let Some(ref mut tree) = self.tree {
            return tree.handle_event(event, data);
        }

        Intent::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData) {
        if let Some(ref tree) = self.tree {
            tree.render(frame, area, is_focused, data);
        } else {
            // 空状态
            use ratatui::style::{Color, Style};
            use ratatui::text::Line;
            use ratatui::widgets::{Block, Borders, Paragraph};

            let border_style = if is_focused {
                Style::default().fg(ratatui::style::Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(format!("Commit Files · {}", self.commit_summary));

            let paragraph = Paragraph::new(Line::from(vec![])).block(block);
            frame.render_widget(paragraph, area);
        }
    }
}
