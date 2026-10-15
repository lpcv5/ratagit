use crate::app::PanelView;
use ratatui::widgets::ListState;

/// Viewport history belongs to the renderer, never to published domain snapshots.
#[derive(Default)]
pub struct RenderState {
    pub files: ListState,
    pub branches: BranchesViewport,
    pub commits: RevisionViewport,
    pub stash: RevisionViewport,
    pub help: HelpViewport,
    pub diff: super::diff_panel::DiffViewport,
}

#[derive(Default)]
pub struct BranchesViewport {
    pub list: ListState,
    pub commits: RevisionViewport,
    source: Option<String>,
}

impl BranchesViewport {
    pub fn commits(&mut self, source: Option<&str>) -> &mut RevisionViewport {
        if self.source.as_deref() != source {
            self.commits = RevisionViewport::default();
            self.source = source.map(str::to_owned);
        }
        &mut self.commits
    }
}

#[derive(Default)]
pub struct RevisionViewport {
    list: ListState,
    tree: ListState,
    source: Option<String>,
}

impl RevisionViewport {
    pub fn state(&mut self, tree_mode: bool, source: Option<String>) -> &mut ListState {
        if tree_mode {
            if self.source != source {
                self.tree = ListState::default();
                self.source = source;
            }
            &mut self.tree
        } else {
            &mut self.list
        }
    }
}

#[derive(Default)]
pub struct HelpViewport {
    pub list: ListState,
    pub context: Option<(String, String)>,
}

pub fn prepare(state: &mut ListState, panel: PanelView, count: usize, height: usize) {
    state.select(panel.selected());
    *state.offset_mut() = scroll_offset(state.offset(), state.selected(), count, height);
}

/// Keep context rows on both sides where space permits. Clamp the final page
/// on resize/data shrink and reduce padding for very short viewports.
pub fn scroll_offset(offset: usize, selected: Option<usize>, count: usize, height: usize) -> usize {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrolls_before_cursor_reaches_either_edge() {
        assert_eq!(scroll_offset(0, Some(6), 30, 10), 0);
        assert_eq!(scroll_offset(0, Some(8), 30, 10), 1);
        assert_eq!(scroll_offset(10, Some(12), 30, 10), 10);
        assert_eq!(scroll_offset(10, Some(11), 30, 10), 9);
        assert_eq!(scroll_offset(9, Some(12), 30, 10), 9);
    }

    #[test]
    fn test_scroll_clamps_for_resize_shrink_and_short_viewports() {
        for count in [0, 1, 3, 30] {
            for height in [0, 1, 2, 3, 4, 5, 10, 40] {
                for selected in 0..=count {
                    let offset = scroll_offset(usize::MAX, Some(selected), count, height);
                    assert!(offset <= count.saturating_sub(height));
                    if count > 0 && height > 0 {
                        let selected = selected.min(count - 1);
                        assert!((offset..offset + height).contains(&selected));
                    }
                }
            }
        }
        assert_eq!(scroll_offset(20, Some(28), 30, 20), 10);
        assert_eq!(scroll_offset(20, Some(28), 3, 10), 0);
    }

    #[test]
    fn test_revision_views_retain_parent_offset_and_reset_new_tree_source() {
        let mut viewport = RevisionViewport::default();
        *viewport.state(false, None).offset_mut() = 10;
        *viewport.state(true, Some("a".into())).offset_mut() = 20;
        assert_eq!(viewport.state(false, None).offset(), 10);
        assert_eq!(viewport.state(true, Some("a".into())).offset(), 20);
        assert_eq!(viewport.state(true, Some("b".into())).offset(), 0);
    }
}
