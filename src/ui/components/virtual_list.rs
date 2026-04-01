use std::ops::Range;

/// A virtual scrolling list that tracks only the visible window of items.
///
/// For most lists in ratagit the full item set fits comfortably in memory, so
/// this struct does not stream data. It simply limits *rendering* to the
/// visible viewport, avoiding the cost of building `ListItem`s for thousands
/// of off-screen entries.
///
/// # Usage
///
/// ```rust,ignore
/// let mut vlist = VirtualList::new(commits.len(), area.height as usize);
/// vlist.ensure_visible(selected_index);
/// let visible = vlist.visible_range();
/// // Render only commits[visible] instead of all commits.
/// ```
#[derive(Debug, Clone)]
pub struct VirtualList {
    /// Total number of items in the backing collection.
    total_items: usize,
    /// Height of the viewport in terminal rows.
    viewport_height: usize,
    /// Index of the first visible item.
    scroll_offset: usize,
    /// Extra rows rendered above and below the visible area to reduce flicker
    /// during fast scrolling.
    overscan: usize,
}

impl VirtualList {
    /// Create a new `VirtualList`.
    ///
    /// - `total_items`: number of items in the full list.
    /// - `viewport_height`: visible rows available for rendering.
    pub fn new(total_items: usize, viewport_height: usize) -> Self {
        Self {
            total_items,
            viewport_height,
            scroll_offset: 0,
            overscan: 3,
        }
    }

    /// Set how many extra items are rendered outside the visible area.
    /// Default is 3.
    pub fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Update viewport height (call when the terminal is resized).
    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
        // Clamp offset so the last page doesn't show empty rows.
        self.clamp_offset();
    }

    /// Update total item count (call after list data changes).
    pub fn set_total_items(&mut self, total: usize) {
        self.total_items = total;
        self.clamp_offset();
    }

    /// Scroll so that `index` is within the visible viewport.
    /// No-op if `index` is already visible.
    pub fn ensure_visible(&mut self, index: usize) {
        if index < self.scroll_offset {
            self.scroll_offset = index;
        } else if index >= self.scroll_offset + self.viewport_height {
            self.scroll_offset = index.saturating_sub(self.viewport_height.saturating_sub(1));
        }
        self.clamp_offset();
    }

    /// Scroll down by `delta` rows.
    pub fn scroll_down(&mut self, delta: usize) {
        self.scroll_offset = self.scroll_offset.saturating_add(delta);
        self.clamp_offset();
    }

    /// Scroll up by `delta` rows.
    pub fn scroll_up(&mut self, delta: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(delta);
    }

    /// The range of item indices that should be rendered, including overscan.
    pub fn visible_range(&self) -> Range<usize> {
        let start = self.scroll_offset.saturating_sub(self.overscan);
        let end = (self.scroll_offset + self.viewport_height + self.overscan).min(self.total_items);
        start..end
    }

    /// The strictly visible range (no overscan) — useful for scroll indicator.
    pub fn viewport_range(&self) -> Range<usize> {
        let end = (self.scroll_offset + self.viewport_height).min(self.total_items);
        self.scroll_offset..end
    }

    /// Current scroll offset (index of first visible item).
    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    /// Total number of tracked items.
    pub fn total_items(&self) -> usize {
        self.total_items
    }

    /// Viewport height in rows.
    pub fn viewport_height(&self) -> usize {
        self.viewport_height
    }

    /// True when all items fit within the viewport (no scrolling needed).
    pub fn fits_in_viewport(&self) -> bool {
        self.total_items <= self.viewport_height
    }

    fn clamp_offset(&mut self) {
        if self.total_items == 0 || self.viewport_height == 0 {
            self.scroll_offset = 0;
            return;
        }
        let max_offset = self.total_items.saturating_sub(self.viewport_height);
        if self.scroll_offset > max_offset {
            self.scroll_offset = max_offset;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_range_includes_overscan() {
        let vl = VirtualList::new(100, 10).overscan(3);
        // At offset 0, overscan below: 0..13
        let r = vl.visible_range();
        assert_eq!(r.start, 0);
        assert_eq!(r.end, 13);
    }

    #[test]
    fn ensure_visible_scrolls_down_when_below_viewport() {
        let mut vl = VirtualList::new(100, 10);
        vl.ensure_visible(20);
        assert!(vl.scroll_offset() <= 20);
        assert!(vl.visible_range().contains(&20));
    }

    #[test]
    fn ensure_visible_scrolls_up_when_above_viewport() {
        let mut vl = VirtualList::new(100, 10);
        vl.scroll_down(50);
        vl.ensure_visible(5);
        assert_eq!(vl.scroll_offset(), 5);
    }

    #[test]
    fn clamp_prevents_over_scroll() {
        let mut vl = VirtualList::new(10, 5);
        vl.scroll_down(100);
        assert_eq!(vl.scroll_offset(), 5); // max = 10 - 5
    }

    #[test]
    fn fits_in_viewport_when_small() {
        let vl = VirtualList::new(3, 10);
        assert!(vl.fits_in_viewport());
    }

    #[test]
    fn viewport_range_does_not_exceed_total() {
        let vl = VirtualList::new(5, 10);
        assert_eq!(vl.viewport_range(), 0..5);
    }
}
