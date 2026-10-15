//! Render a controlled diff model; the owner derives status and selection.
use crate::ui::components::diff::{DiffView, DiffViewModel, DiffViewState};
use crate::ui::theme::UiTheme;
use ratatui::{layout::Rect, Frame};

pub fn render_diff_panel(
    frame: &mut Frame,
    area: Rect,
    model: DiffViewModel<'_>,
    state: &mut DiffViewState,
) {
    let theme = UiTheme::default();
    frame.render_stateful_widget(DiffView::new(model, &theme), area, state);
}
