pub fn move_selected_index(selected: &mut usize, len: usize, move_up: bool) -> bool {
    if len == 0 {
        *selected = 0;
        return false;
    }

    let old_selected = *selected;
    if move_up {
        *selected = selected.saturating_sub(1);
    } else {
        *selected = selected.saturating_add(1).min(len - 1);
    }
    *selected != old_selected
}

pub fn move_selected_index_with_scroll_offset(
    selected: &mut usize,
    scroll_offset: &mut usize,
    len: usize,
    move_up: bool,
    visible_lines: usize,
) -> bool {
    if len == 0 {
        *selected = 0;
        *scroll_offset = 0;
        return false;
    }

    let old_selected = *selected;
    move_selected_index(selected, len, move_up);
    let next_offset = scroll_offset_for_selection(*scroll_offset, *selected, len, visible_lines);
    *scroll_offset = next_offset;
    *selected != old_selected
}

pub fn scroll_offset_for_selection(
    scroll_offset: usize,
    selected: usize,
    len: usize,
    visible_lines: usize,
) -> usize {
    if visible_lines == 0 || len <= visible_lines {
        return 0;
    }

    let max_start = len - visible_lines;
    let selected = selected.min(len - 1);
    let mut start = scroll_offset.min(max_start);
    let reserve = 3.min(visible_lines.saturating_sub(1) / 2);
    let top_threshold = start.saturating_add(reserve);
    let bottom_threshold = start
        .saturating_add(visible_lines.saturating_sub(1))
        .saturating_sub(reserve);

    if selected < top_threshold {
        start = selected.saturating_sub(reserve);
    } else if selected > bottom_threshold {
        start = selected
            .saturating_add(reserve)
            .saturating_add(1)
            .saturating_sub(visible_lines);
    }

    start.min(max_start)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_offset_stays_fixed_for_middle_jk_motion() {
        let mut selected = 19;
        let mut offset = 16;

        move_selected_index_with_scroll_offset(&mut selected, &mut offset, 30, false, 8);
        assert_eq!((selected, offset), (20, 16));

        move_selected_index_with_scroll_offset(&mut selected, &mut offset, 30, true, 8);
        assert_eq!((selected, offset), (19, 16));
    }

    #[test]
    fn scroll_offset_moves_only_after_three_row_threshold() {
        assert_eq!(scroll_offset_for_selection(16, 19, 30, 8), 16);
        assert_eq!(scroll_offset_for_selection(16, 18, 30, 8), 15);
        assert_eq!(scroll_offset_for_selection(16, 20, 30, 8), 16);
        assert_eq!(scroll_offset_for_selection(16, 21, 30, 8), 17);
    }
}
