use ratagit_core::AppContext;
use ratatui::Frame;
use ratatui::layout::Rect;

use crate::modal::{ConfirmBody, ModalSpec, ModalTone, render_confirm_body, render_modal};

pub(crate) fn render_sync_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    if !state.ui.push_force_confirm.active {
        return;
    }

    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Danger, 76, 12, 20, 8, 1),
        &[&[("Enter", "force push"), ("Esc", "cancel")]],
        |frame, content| {
            render_confirm_body(
                frame,
                content,
                ModalTone::Danger,
                ConfirmBody::new("Force push current branch?")
                    .secondary("This can overwrite remote commits.")
                    .details(format!(
                        "Git refused normal push:\n{}",
                        state.ui.push_force_confirm.reason
                    )),
            );
        },
    );
}
