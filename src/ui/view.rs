use crate::app::App;
use crossterm::event::KeyEvent;
use ratatui::{Frame, layout::Rect};

/// 视图 trait（Component 思想）
pub trait View {
    /// 渲染视图
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);

    /// 处理键盘输入
    fn handle_key(&self, key: KeyEvent, app: &App) -> Option<crate::app::Message>;
}
