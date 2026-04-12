use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

/// 原子组件：可选择的列表
/// 负责渲染带高亮的列表项，管理 ListState 的渲染
pub struct SelectableList<'a> {
    items: Vec<ListItem<'a>>,
    title: &'a str,
    is_focused: bool,
    highlight_symbol: &'a str,
}

impl<'a> SelectableList<'a> {
    pub fn new(
        items: Vec<ListItem<'a>>,
        title: &'a str,
        is_focused: bool,
        highlight_symbol: &'a str,
    ) -> Self {
        Self {
            items,
            title,
            is_focused,
            highlight_symbol,
        }
    }

    pub fn render(self, frame: &mut Frame, area: Rect, state: &mut ListState) {
        let border_style = if self.is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(self.title.to_string());

        let list = List::new(self.items)
            .block(block)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol(self.highlight_symbol);

        frame.render_stateful_widget(list, area, state);
    }

    pub fn render_empty(frame: &mut Frame, area: Rect, title: &'a str, is_focused: bool) {
        let border_style = if is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(title.to_string());

        let paragraph = Paragraph::new("No items")
            .block(block)
            .style(Style::default().fg(Color::DarkGray));

        frame.render_widget(paragraph, area);
    }
}

/// 原子组件：可滚动文本
/// 负责渲染文本内容，支持垂直滚动
pub struct ScrollableText<'a> {
    content: &'a str,
    title: &'a str,
    is_focused: bool,
    scroll_offset: u16,
}

impl<'a> ScrollableText<'a> {
    pub fn new(content: &'a str, title: &'a str, is_focused: bool, scroll_offset: u16) -> Self {
        Self {
            content,
            title,
            is_focused,
            scroll_offset,
        }
    }

    pub fn render(self, frame: &mut Frame, area: Rect) {
        let border_style = if self.is_focused {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(self.title.to_string());

        let paragraph = Paragraph::new(self.content)
            .block(block)
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }
}
