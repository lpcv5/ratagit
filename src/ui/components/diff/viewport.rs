use super::DiffViewModel;
use std::ops::Range;

#[derive(Default)]
pub struct DiffViewState {
    request: Option<crate::git::patch::PatchRequest>,
    selected: Option<Range<usize>>,
    requested_scroll: usize,
    offset: usize,
    height: usize,
    manual: bool,
    anchor_revision: Option<u64>,
}

impl DiffViewState {
    pub(super) fn offset(&mut self, model: &DiffViewModel<'_>, height: usize) -> usize {
        let selected = model
            .patch
            .filter(|_| model.ready() && model.selection.focused)
            .and_then(|patch| {
                if model.selection.is_line_mode() {
                    model
                        .selection
                        .line_cursor
                        .and_then(|id| patch.line_row(id))
                        .map(|row| row..row + 1)
                } else {
                    patch
                        .change_blocks()
                        .into_iter()
                        .find(|span| Some(span.id) == model.selection.hunk_cursor)
                        .map(|span| span.range)
                }
            });
        let anchor = model.viewport_anchor.filter(|anchor| {
            model.ready()
                && selected.is_some()
                && self
                    .selected
                    .as_ref()
                    .is_some_and(|range| range.contains(&anchor.previous_row))
                && self.anchor_revision != Some(anchor.revision)
        });
        let request = model.patch.map(|patch| &patch.request);
        if self.request.as_ref() != request && anchor.is_none() {
            *self = Self::default();
        }
        self.request = request.cloned();
        if !model.ready() {
            // Debounce and in-flight reads must not consume selection history or
            // anchor requests before the replacement document becomes visible.
            return self.offset.min(model.lines.len().saturating_sub(1));
        }
        let selection_changed = selected.is_some() && selected != self.selected;
        if let Some(anchor) = anchor {
            // Preserve the same content on screen, even when hunk merging or
            // file headers move it. Allow trailing blank space on a shorter
            // final page instead of pulling the content up to fill the panel.
            self.offset = if anchor.row >= anchor.previous_row {
                self.offset.saturating_add(anchor.row - anchor.previous_row)
            } else {
                self.offset.saturating_sub(anchor.previous_row - anchor.row)
            };
        } else {
            if selection_changed {
                self.manual = false;
            } else if model.scroll != self.requested_scroll {
                self.offset = model.scroll;
                self.manual = true;
            }
            if !self.manual && (selection_changed || height != self.height) {
                if let Some(range) = &selected {
                    self.offset = if model.selection.is_line_mode() {
                        crate::ui::components::scroll::scroll_offset(
                            self.offset,
                            Some(range.start),
                            model.lines.len(),
                            height,
                        )
                    } else {
                        change_offset(self.offset, range, model.lines.len(), height)
                    };
                }
            }
        }
        self.offset = self.offset.min(model.lines.len().saturating_sub(1));
        self.selected = selected;
        self.requested_scroll = model.scroll;
        self.height = height;
        self.anchor_revision = model.viewport_anchor.map(|anchor| anchor.revision);
        self.offset
    }
}

/// Fit the entire change before reserving context. Oversized changes start at the
/// top so the available height shows as much of their content as possible.
fn change_offset(offset: usize, range: &Range<usize>, count: usize, height: usize) -> usize {
    if height == 0 || count == 0 {
        return 0;
    }
    let max = count.saturating_sub(height);
    if range.len() > height {
        return range.start.min(max);
    }
    let padding = crate::ui::LIST_SCROLL_PADDING.min((height - range.len()) / 2);
    let offset = offset.min(max);
    if range.start < offset + padding {
        range.start.saturating_sub(padding).min(max)
    } else if range.end + padding > offset + height {
        (range.end + padding).saturating_sub(height).min(max)
    } else {
        offset
    }
}
