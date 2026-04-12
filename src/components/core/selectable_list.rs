use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::components::core::{muted_text_style, panel_block, selected_row_style};

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
        const BOTTOM_ITEM_RESERVE: usize = 3;

        let block = panel_block(self.title, self.is_focused);
        let highlight_style = if self.is_focused {
            selected_row_style()
        } else {
            Style::default()
        };
        let highlight_symbol = if self.is_focused {
            self.highlight_symbol
        } else {
            ""
        };
        let item_count = self.items.len();
        apply_bottom_item_reserve(state, item_count, area, BOTTOM_ITEM_RESERVE);

        let list = List::new(self.items)
            .block(block)
            .highlight_style(highlight_style)
            .highlight_symbol(highlight_symbol);

        frame.render_stateful_widget(list, area, state);
    }

    pub fn render_empty(frame: &mut Frame, area: Rect, title: &'a str, is_focused: bool) {
        let block = panel_block(title, is_focused);

        let paragraph = Paragraph::new("No items")
            .block(block)
            .style(muted_text_style());

        frame.render_widget(paragraph, area);
    }
}

fn apply_bottom_item_reserve(state: &mut ListState, item_count: usize, area: Rect, reserve: usize) {
    if item_count == 0 {
        *state.offset_mut() = 0;
        return;
    }

    let Some(selected_idx) = state.selected() else {
        *state.offset_mut() = 0;
        return;
    };

    let selected = selected_idx.min(item_count.saturating_sub(1));
    if selected != selected_idx {
        state.select(Some(selected));
    }

    let viewport_height = usize::from(area.height.saturating_sub(2));
    if viewport_height == 0 {
        *state.offset_mut() = 0;
        return;
    }

    let anchor = if viewport_height > reserve {
        viewport_height.saturating_sub(1 + reserve)
    } else {
        viewport_height.saturating_sub(1)
    };
    let max_offset = item_count.saturating_sub(viewport_height);
    *state.offset_mut() = selected.saturating_sub(anchor).min(max_offset);
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
        let block = panel_block(self.title, self.is_focused);

        let paragraph = Paragraph::new(self.content)
            .block(block)
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }
}
