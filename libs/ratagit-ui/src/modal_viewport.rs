use ratagit_core::AppContext;
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Block, Clear, Paragraph, Wrap};

use crate::{TerminalSize, UiTheme};

const MIN_MODAL_WIDTH: usize = 40;
const MIN_MODAL_HEIGHT: usize = 14;

/// Rendering and keyboard routing share this limit so hidden dialogs cannot run.
pub fn modal_viewport_too_small(state: &AppContext, size: TerminalSize) -> bool {
    has_modal(state) && (size.width < MIN_MODAL_WIDTH || size.height < MIN_MODAL_HEIGHT)
}

fn has_modal(state: &AppContext) -> bool {
    state.ui.workflow.dialog.is_some()
        || state.ui.workflow.remote_request.is_some()
        || state.ui.details.repository_confirm.is_some()
        || state.ui.details.patch.discard.is_some()
        || state.ui.editor.is_active()
        || state.ui.branches.create.active
        || state.ui.branches.delete_menu.menu.active
        || state.ui.branches.delete_confirm.active
        || state.ui.branches.force_delete_confirm.active
        || state.ui.branches.rebase_menu.menu.active
        || state.ui.branches.auto_stash_confirm.active
        || state.ui.reset_menu.menu.active
        || state.ui.reset_menu.danger_confirm.is_some()
        || state.ui.discard_confirm.active
        || state.ui.push_force_confirm.active
        || state.ui.stage_all_confirm.active
        || state.ui.command_palette.active
}

pub(crate) fn render_modal_viewport_warning(
    frame: &mut Frame<'_>,
    state: &AppContext,
    area: Rect,
    theme: &UiTheme,
) -> bool {
    let size = TerminalSize {
        width: frame.area().width as usize,
        height: frame.area().height as usize,
    };
    if !modal_viewport_too_small(state, size) {
        return false;
    }

    frame.render_widget(Clear, frame.area());
    frame.render_widget(
        Block::default().style(theme.modal_background_style()),
        frame.area(),
    );
    let lines = if area.width >= MIN_MODAL_WIDTH as u16 {
        vec![
            Line::styled("Window too small", theme.modal_warning_style()),
            Line::from("Resize to at least 40 columns × 14 rows"),
            Line::from("Esc cancel · Ctrl+C quit"),
        ]
    } else {
        vec![
            Line::styled("Resize to 40×14", theme.modal_warning_style()),
            Line::from("Esc cancel"),
            Line::from("Ctrl+C quit"),
        ]
    };
    let height = (lines.len() as u16).min(area.height);
    let content = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(height) / 2,
        area.width,
        height,
    );
    frame.render_widget(
        Paragraph::new(lines)
            .style(theme.modal_text_style())
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: false }),
        content,
    );
    true
}
