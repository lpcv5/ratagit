use crossterm::event::Event;
use ratatui::{layout::Rect, Frame};

use crate::app::CachedData;

use crate::components::Intent;

/// Component trait：所有 UI 组件的通用接口
/// 组件持有自身状态，通过 `is_focused` 参数知道是否获得焦点，
/// 返回 Intent 而非直接执行操作。
pub trait Component {
    /// 处理输入事件，返回 Intent
    /// 组件只接收与自身相关的事件，无需检查 active_panel
    fn handle_event(&mut self, event: &Event, data: &CachedData) -> Intent;

    /// 渲染组件
    fn render(&self, frame: &mut Frame, area: Rect, is_focused: bool, data: &CachedData);
}
