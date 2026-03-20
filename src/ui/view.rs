#![allow(dead_code)]

use crate::app::App;
use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, Frame};

/// Documentation comment in English.
pub trait View {
    /// Documentation comment in English.
    fn render(&self, frame: &mut Frame, area: Rect, app: &App);

    /// Documentation comment in English.
    fn handle_key(&self, key: KeyEvent, app: &App) -> Option<crate::app::Message>;
}
