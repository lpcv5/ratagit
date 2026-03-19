use crate::app::{App, Message};
use crate::ui::View;
use crate::git::FileStatus;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

/// StatusView 状态
pub struct StatusViewState {
    /// 当前选中的列表
    pub current_list: CurrentList,

    /// Unstaged 文件列表状态
    pub unstaged_state: ListState,

    /// Staged 文件列表状态
    pub staged_state: ListState,

    /// Untracked 文件列表状态
    pub untracked_state: ListState,
}

/// 当前活动的列表
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CurrentList {
    Unstaged,
    Staged,
    Untracked,
}

/// StatusView 组件
pub struct StatusView {
    state: StatusViewState,
}

impl StatusView {
    pub fn new() -> Self {
        let mut unstaged_state = ListState::default();
        unstaged_state.select(Some(0));

        Self {
            state: StatusViewState {
                current_list: CurrentList::Unstaged,
                unstaged_state,
                staged_state: ListState::default(),
                untracked_state: ListState::default(),
            },
        }
    }

    /// 获取文件状态颜色
    fn get_status_color(status: &FileStatus) -> Color {
        match status {
            FileStatus::New => Color::Green,
            FileStatus::Modified => Color::Yellow,
            FileStatus::Deleted => Color::Red,
            FileStatus::Renamed => Color::Magenta,
            FileStatus::TypeChange => Color::Cyan,
        }
    }

    /// 获取文件状态文本
    fn get_status_text(status: &FileStatus) -> &'static str {
        match status {
            FileStatus::New => "new",
            FileStatus::Modified => "mod",
            FileStatus::Deleted => "del",
            FileStatus::Renamed => "ren",
            FileStatus::TypeChange => "typ",
        }
    }
}

impl View for StatusView {
    fn render(&self, frame: &mut Frame, area: Rect, app: &App) {
        // 创建布局：左侧 Unstaged，右侧 Staged
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // 渲染 Unstaged 列表（包含 Unstaged 和 Untracked）
        {
            let mut items = Vec::new();

            // Unstaged 文件
            if !app.status.unstaged.is_empty() {
                items.push(ListItem::new("=== Unstaged ===").style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ));
                for file in &app.status.unstaged {
                    let status_text = Self::get_status_text(&file.status);
                    let color = Self::get_status_color(&file.status);
                    let text = format!("{} {}", status_text, file.path.display());
                    items.push(ListItem::new(text).style(Style::default().fg(color)));
                }
            }

            // Untracked 文件
            if !app.status.untracked.is_empty() {
                items.push(ListItem::new("=== Untracked ===").style(
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::BOLD),
                ));
                for file in &app.status.untracked {
                    let text = format!("??? {}", file.path.display());
                    items.push(ListItem::new(text).style(Style::default().fg(Color::Gray)));
                }
            }

            if items.is_empty() {
                items.push(ListItem::new("No changes"));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Changes"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_stateful_widget(list, chunks[0], &mut self.state.unstaged_state.clone());
        }

        // 渲染 Staged 列表
        {
            let mut items = Vec::new();

            if !app.status.staged.is_empty() {
                for file in &app.status.staged {
                    let status_text = Self::get_status_text(&file.status);
                    let color = Self::get_status_color(&file.status);
                    let text = format!("{} {}", status_text, file.path.display());
                    items.push(ListItem::new(text).style(Style::default().fg(color)));
                }
            } else {
                items.push(ListItem::new("No staged changes"));
            }

            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Staged"))
                .highlight_style(
                    Style::default()
                        .bg(Color::DarkGray)
                        .add_modifier(Modifier::BOLD),
                );

            frame.render_stateful_widget(list, chunks[1], &mut self.state.staged_state.clone());
        }
    }

    fn handle_key(&self, key: KeyEvent, _app: &App) -> Option<Message> {
        match key.code {
            // 上下移动
            KeyCode::Char('j') | KeyCode::Down => {
                // TODO: 实现列表导航
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                // TODO: 实现列表导航
                None
            }

            // Space: Stage/Unstage 文件
            KeyCode::Char(' ') => {
                // TODO: 根据当前列表和选中项，stage 或 unstage
                None
            }

            // 刷新
            KeyCode::Char('r') => Some(Message::RefreshStatus),

            _ => None,
        }
    }
}
