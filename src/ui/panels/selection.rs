use crate::app::PanelView;
use ratatui::widgets::ListState;

/// Each frame starts with the same zero-offset viewport as before. Widget
/// scrolling stays local and never mutates a published feature model.
pub fn list_state(panel: PanelView) -> ListState {
    ListState::default().with_selected(panel.selected())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::FilesState;
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        widgets::{List, ListItem, StatefulWidget},
    };

    #[test]
    fn test_list_viewports_match_legacy_scrolling_without_writing_back_selection() {
        for count in [0, 1, 30] {
            for selected in [None, Some(0), Some(1), Some(15), Some(29), Some(99)] {
                let mut files = FilesState::default();
                files.test_view_mut().panel.select(selected);
                let panel = files.view().render_view().panel;
                for height in [0, 1, 2, 8, 40] {
                    let area = Rect::new(0, 0, 24, height);
                    let widget = || {
                        List::new((0..count).map(|index| ListItem::new(format!("row {index}"))))
                            .scroll_padding(crate::ui::LIST_SCROLL_PADDING)
                    };
                    let mut actual = Buffer::empty(area);
                    let mut expected = Buffer::empty(area);
                    let mut actual_state = list_state(panel);
                    // Original PanelState constructed a default widget state and
                    // applied selection before the renderer copied it per frame.
                    let mut legacy = ListState::default();
                    legacy.select(selected);
                    StatefulWidget::render(widget(), area, &mut actual, &mut actual_state);
                    StatefulWidget::render(widget(), area, &mut expected, &mut legacy);
                    assert_eq!(
                        actual, expected,
                        "count={count} selected={selected:?} height={height}"
                    );
                    assert_eq!(actual_state, legacy);
                    assert_eq!(panel.selected(), selected);
                    assert_eq!(files.view().panel.selected(), selected);
                }
            }
        }
    }
}
