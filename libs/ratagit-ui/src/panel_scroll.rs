use ratagit_core::ScrollDirection;

use super::panel_types::PanelLine;

pub(crate) fn render_indexed_entries_window_with<T>(
    items: &[T],
    selected: usize,
    scroll_direction: Option<ScrollDirection>,
    scroll_direction_origin: usize,
    max_lines: usize,
    render_item: impl Fn(usize, &T) -> PanelLine,
) -> Vec<PanelLine> {
    const SCROLL_RESERVE: usize = 3;

    if max_lines == 0 {
        return Vec::new();
    }
    if items.is_empty() {
        return Vec::new();
    }
    let start = scroll_window_start(
        items.len(),
        selected,
        scroll_direction,
        scroll_direction_origin,
        max_lines,
        SCROLL_RESERVE,
    );
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
    scroll_direction: Option<ScrollDirection>,
    scroll_direction_origin: usize,
    max_lines: usize,
    reserve: usize,
) -> usize {
    if max_lines == 0 || len <= max_lines {
        return 0;
    }
    let max_start = len.saturating_sub(max_lines);
    let selected = selected.min(len - 1);
    match scroll_direction {
        Some(ScrollDirection::Up) => {
            let previous_start =
                bottom_reserve_start(scroll_direction_origin, max_lines, max_start, reserve);
            if selected >= previous_start.saturating_add(reserve) {
                previous_start
            } else {
                top_reserve_start(selected, max_start, reserve)
            }
        }
        Some(ScrollDirection::Down) => {
            let previous_start = top_reserve_start(scroll_direction_origin, max_start, reserve);
            let bottom_threshold = previous_start
                .saturating_add(max_lines.saturating_sub(1))
                .saturating_sub(reserve);
            if selected <= bottom_threshold {
                previous_start
            } else {
                bottom_reserve_start(selected, max_lines, max_start, reserve)
            }
        }
        None => bottom_reserve_start(selected, max_lines, max_start, reserve),
    }
}

fn top_reserve_start(selected: usize, max_start: usize, reserve: usize) -> usize {
    selected.saturating_sub(reserve).min(max_start)
}

fn bottom_reserve_start(
    selected: usize,
    max_lines: usize,
    max_start: usize,
    reserve: usize,
) -> usize {
    selected
        .saturating_add(1 + reserve)
        .saturating_sub(max_lines)
        .min(max_start)
}
