use ratagit_core::{AppContext, ResetChoice};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::modal::{
    ChoiceMenuBody, ConfirmBody, ModalSpec, ModalTone, choice_menu_modal_height, choice_menu_rows,
    render_choice_menu_body, render_confirm_body, render_modal,
};
use crate::theme::{modal_danger_style, modal_muted_style};

pub(crate) fn render_reset_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    if state.ui.reset_menu.danger_confirm.is_some() {
        render_reset_danger_modal(frame, state, area);
        return;
    }

    if !state.ui.reset_menu.menu.active {
        return;
    }

    let choices = reset_choices();
    render_modal(
        frame,
        area,
        ModalSpec::new(
            "Reset",
            ModalTone::Warning,
            72,
            choice_menu_modal_height(choices.len(), 1),
            20,
            8,
            1,
        ),
        &[&[("j/k", "select"), ("Enter", "confirm"), ("Esc", "cancel")]],
        |frame, content| {
            render_choice_menu_body(
                frame,
                content,
                ModalTone::Warning,
                ChoiceMenuBody {
                    intro: "Choose reset scope for the whole repo.".to_string(),
                    list_title: "Mode",
                    choices: &choices,
                    selected: state.ui.reset_menu.menu.selected,
                    list_height: 5,
                    description: reset_choice_description(state.ui.reset_menu.menu.selected)
                        .to_string(),
                },
            );
        },
    );
}

fn render_reset_danger_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    let choice = state
        .ui
        .reset_menu
        .danger_confirm
        .unwrap_or(ResetChoice::Hard);
    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Danger, 76, 12, 20, 8, 1),
        &[&[("Enter", reset_confirm_action(choice)), ("Esc", "cancel")]],
        |frame, content| {
            render_confirm_body(
                frame,
                content,
                ModalTone::Danger,
                ConfirmBody::new(reset_confirm_question(choice))
                    .secondary("This can discard uncommitted work that reflog cannot restore.")
                    .details(reset_confirm_details(choice)),
            );
        },
    );
}

fn reset_choices() -> Vec<(ResetChoice, &'static str, Style)> {
    choice_menu_rows(&ResetChoice::ALL, reset_choice_label, reset_choice_style)
}

fn reset_choice_label(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Mixed => "mixed",
        ResetChoice::Soft => "soft",
        ResetChoice::Hard => "hard",
        ResetChoice::Nuke => "Nuke",
    }
}

fn reset_choice_style(choice: ResetChoice) -> Style {
    if choice == ResetChoice::Nuke {
        modal_danger_style()
    } else {
        modal_muted_style()
    }
}

fn reset_choice_description(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Mixed => "Mixed: reset index to HEAD; keep working tree changes.",
        ResetChoice::Soft => "Soft: move HEAD/index only; keep staged and working tree changes.",
        ResetChoice::Hard => "Hard: discard tracked working tree and index changes.",
        ResetChoice::Nuke => {
            "Nuke: hard reset, then remove untracked files/directories with `git clean -fd`."
        }
    }
}

fn reset_confirm_action(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Nuke => "nuke",
        _ => "reset hard",
    }
}

fn reset_confirm_question(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Nuke => "Nuke working tree?",
        _ => "Hard reset working tree to HEAD?",
    }
}

fn reset_confirm_details(choice: ResetChoice) -> &'static str {
    match choice {
        ResetChoice::Nuke => "Runs `git reset --hard HEAD` and `git clean -fd`.",
        _ => "Runs `git reset --hard HEAD`.",
    }
}
