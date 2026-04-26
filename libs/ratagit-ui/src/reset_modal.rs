use ratagit_core::{AppState, ResetChoice};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Paragraph, Wrap};

use crate::modal::{
    ModalSpec, ModalTone, render_action_footer, render_choice_list, render_modal_frame,
    render_section_label,
};
use crate::theme::{modal_danger_style, modal_muted_style};

pub(crate) fn render_reset_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    if !state.reset_menu.active {
        return;
    }

    let Some(modal) = render_modal_frame(
        frame,
        area,
        ModalSpec::new("Reset", ModalTone::Warning, 72, 13, 20, 8, 1),
    ) else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(5),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(modal.content);

    frame.render_widget(
        Paragraph::new("Choose reset scope for the whole repo."),
        rows[0],
    );

    let choices = ResetChoice::ALL
        .iter()
        .map(|choice| {
            (
                *choice,
                reset_choice_label(*choice),
                reset_choice_style(*choice),
            )
        })
        .collect::<Vec<_>>();
    render_choice_list(
        frame,
        rows[1],
        "Mode",
        &choices,
        state.reset_menu.selected,
        ModalTone::Warning,
    );

    render_section_label(frame, rows[2], "Description");
    frame.render_widget(
        Paragraph::new(reset_choice_description(state.reset_menu.selected))
            .wrap(Wrap { trim: false }),
        rows[3],
    );
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Warning,
            &[("j/k", "select"), ("Enter", "confirm"), ("Esc", "cancel")],
        );
    }
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
