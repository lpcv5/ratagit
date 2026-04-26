#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

pub fn reset_scroll_origin(
    selected: usize,
    len: usize,
    scroll_direction: &mut Option<ScrollDirection>,
    scroll_direction_origin: &mut usize,
) {
    if len == 0 {
        *scroll_direction = None;
        *scroll_direction_origin = 0;
    } else {
        *scroll_direction_origin = selected.min(len - 1);
    }
}

pub fn move_selected_index_with_scroll(
    selected: &mut usize,
    len: usize,
    move_up: bool,
    scroll_direction: &mut Option<ScrollDirection>,
    scroll_direction_origin: &mut usize,
) -> bool {
    if len == 0 {
        *selected = 0;
        *scroll_direction = None;
        *scroll_direction_origin = 0;
        return false;
    }

    let old_selected = *selected;
    let next_direction = if move_up {
        ScrollDirection::Up
    } else {
        ScrollDirection::Down
    };
    if move_up {
        *selected = selected.saturating_sub(1);
    } else {
        *selected = selected.saturating_add(1).min(len - 1);
    }
    if *selected != old_selected && *scroll_direction != Some(next_direction) {
        *scroll_direction = Some(next_direction);
        *scroll_direction_origin = old_selected;
    }
    *selected != old_selected
}
