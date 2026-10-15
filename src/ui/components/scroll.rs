/// Keep context rows on both sides where space permits. Clamp the final page
/// on resize/data shrink and reduce padding for very short viewports.
pub(crate) fn scroll_offset(
    offset: usize,
    selected: Option<usize>,
    count: usize,
    height: usize,
) -> usize {
    if count == 0 || height == 0 {
        return 0;
    }
    let max = count.saturating_sub(height);
    let mut offset = offset.min(max);
    if let Some(selected) = selected.map(|index| index.min(count - 1)) {
        let padding = crate::ui::LIST_SCROLL_PADDING.min((height - 1) / 2);
        if selected < offset + padding {
            offset = selected.saturating_sub(padding);
        } else if selected >= offset + height - padding {
            offset = (selected + padding + 1).saturating_sub(height);
        }
    }
    offset.min(max)
}
