//! Reusable, controlled hunk/diff display with renderer-owned viewport history.
//! Application adapters supply immutable data and selection; Flux owns commands.

mod ansi;
mod bindings;
mod chrome;
mod model;
mod rows;
mod viewport;

#[cfg(test)]
mod tests;

pub use bindings::{DiffBehaviorBindings, DiffIntent};
pub use model::{
    DiffPosition, DiffSelection, DiffSelectionMode, DiffStatus, DiffViewModel, DiffViewportAnchor,
};
pub use viewport::DiffViewState;

use crate::ui::{
    components::molecules::{card, EmptyState},
    theme::UiTheme,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{List, StatefulWidget, Widget},
};

/// Draw a diff into any Ratatui rectangle using an explicit theme and view model.
/// Keep one `DiffViewState` per mounted view, including across snapshot updates.
pub struct DiffView<'a> {
    model: DiffViewModel<'a>,
    theme: &'a UiTheme,
}

impl<'a> DiffView<'a> {
    pub fn new(model: DiffViewModel<'a>, theme: &'a UiTheme) -> Self {
        Self { model, theme }
    }
}

impl StatefulWidget for DiffView<'_> {
    type State = DiffViewState;

    fn render(self, area: Rect, buffer: &mut Buffer, state: &mut Self::State) {
        let model = &self.model;
        if model.lines.is_empty() {
            *state = DiffViewState::default();
            let block = card(self.theme, "Diff", model.selection.focused);
            let inner = block.inner(area);
            block.render(area, buffer);
            let hint = if model.status == DiffStatus::Loading {
                "Loading diff..."
            } else {
                model.empty_message
            };
            EmptyState::new(hint, self.theme).render(inner, buffer);
            return;
        }
        let height = usize::from(area.height.saturating_sub(2)).max(1);
        let scroll = state.offset(model, height);
        let items = rows::items(model, self.theme, scroll, height, area.width);
        let end = (scroll + items.len()).min(model.lines.len());
        let block = chrome::block(model, self.theme, scroll, end, area.width);
        Widget::render(List::new(items).block(block), area, buffer);
    }
}
