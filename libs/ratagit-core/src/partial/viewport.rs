//! Keep a complete change block visible before reserving context.
use super::*;

pub(super) fn scroll_to_cursor(state: &mut AppContext, visible_lines: usize) {
    let Some(patch) = &state.repo.details.patch else {
        return;
    };
    let cursor: std::collections::HashSet<_> = cursor_ids(state).into_iter().collect();
    let rows = patch_rows(patch);
    let positions: Vec<_> = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.id.is_some_and(|id| cursor.contains(&id)))
        .map(|(row, _)| row)
        .collect();
    let Some(&start) = positions.first() else {
        return;
    };
    let height = visible_lines;
    let offset = state.ui.details.scroll_offset;
    state.ui.details.scroll_offset = if state.ui.details.patch.mode == SelectionMode::Line {
        crate::scroll_offset_for_selection(offset, start, rows.len(), height)
    } else {
        let mut end = positions.last().copied().unwrap() + 1;
        while rows
            .get(end)
            .is_some_and(|row| row.kind == PatchLineKind::NoNewline)
        {
            end += 1;
        }
        change_offset(offset, start..end, rows.len(), height)
    };
}

fn change_offset(
    offset: usize,
    range: std::ops::Range<usize>,
    count: usize,
    height: usize,
) -> usize {
    if height == 0 || count == 0 {
        return 0;
    }
    let max = count.saturating_sub(height);
    if range.len() > height {
        return range.start.min(max);
    }
    let padding = 3.min((height - range.len()) / 2);
    let offset = offset.min(max);
    if range.start < offset + padding {
        range.start.saturating_sub(padding).min(max)
    } else if range.end + padding > offset + height {
        (range.end + padding).saturating_sub(height).min(max)
    } else {
        offset
    }
}

#[cfg(test)]
mod tests {
    use super::change_offset;
    #[test]
    fn test_whole_block_fits_before_context_padding() {
        assert_eq!(change_offset(0, 10..15, 30, 6), 9);
        assert_eq!(change_offset(0, 10..25, 30, 6), 10);
        assert_eq!(change_offset(12, 10..15, 30, 6), 10);
    }
}
