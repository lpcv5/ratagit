use ratagit_core::{AppState, ResetChoice};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::{
    Block, Borders, HighlightSpacing, List, ListItem, ListState, Paragraph, Wrap,
};

use crate::modal::{
    ModalSpec, ModalTone, modal_tone_style, render_action_footer, render_modal_frame,
    render_section_label,
};
use crate::theme::{modal_danger_style, modal_muted_style, selected_row_style};

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

    let items = ResetChoice::ALL
        .iter()
        .map(|choice| {
            let style = if *choice == ResetChoice::Nuke {
                modal_danger_style()
            } else {
                modal_muted_style()
            };
            ListItem::new(Line::styled(
                format!("  {}", reset_choice_label(*choice)),
                style,
            ))
        })
        .collect::<Vec<_>>();
    let selected = ResetChoice::ALL
        .iter()
        .position(|choice| *choice == state.reset_menu.selected)
        .unwrap_or(0);
    let mut list_state = ListState::default();
    list_state.select(Some(selected));
    let list = List::new(items)
        .highlight_style(selected_row_style())
        .highlight_spacing(HighlightSpacing::Never)
        .block(
            Block::default()
                .title(Line::styled(" Mode ", modal_tone_style(ModalTone::Warning)))
                .borders(Borders::ALL)
                .border_style(modal_muted_style()),
        );
    frame.render_stateful_widget(list, rows[1], &mut list_state);

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
