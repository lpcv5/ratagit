use ratagit_core::scroll_offset_for_selection;

use super::panel_types::PanelLine;

pub(crate) fn render_indexed_entries_window_with<T>(
    items: &[T],
    selected: usize,
    scroll_offset: usize,
    max_lines: usize,
    render_item: impl Fn(usize, &T) -> PanelLine,
) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }
    if items.is_empty() {
        return Vec::new();
    }
    let start = scroll_window_start(items.len(), selected, scroll_offset, max_lines);
    items
        .iter()
        .enumerate()
        .skip(start)
        .take(max_lines)
        .map(|(index, item)| render_item(index, item))
        .collect()
}

pub(crate) fn scroll_window_start(
    len: usize,
    selected: usize,
    scroll_offset: usize,
    max_lines: usize,
) -> usize {
    scroll_offset_for_selection(scroll_offset, selected, len, max_lines)
}
