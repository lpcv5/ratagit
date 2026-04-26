use ratagit_core::{AppState, AutoStashOperation, BranchDeleteChoice, BranchRebaseChoice};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use crate::frame::TerminalCursor;
use crate::modal::{
    ModalSpec, ModalTone, modal_content_rect, render_action_footer, render_choice_list,
    render_input_block, render_modal_frame, render_muted_text, render_section_label,
    render_warning_text,
};
use crate::theme::modal_muted_style;

pub(crate) fn render_branch_modals(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    if state.branches.create.active {
        render_branch_create_modal(frame, state, area);
    }
    if state.branches.delete_menu.active {
        render_branch_delete_modal(frame, state, area);
    }
    if state.branches.force_delete_confirm.active {
        render_force_delete_modal(frame, state, area);
    }
    if state.branches.rebase_menu.active {
        render_branch_rebase_modal(frame, state, area);
    }
    if state.branches.auto_stash_confirm.active {
        render_auto_stash_modal(frame, state, area);
    }
}

pub(crate) fn branch_create_cursor_position(
    state: &AppState,
    area: Rect,
) -> Option<TerminalCursor> {
    if !state.branches.create.active {
        return None;
    }
    let content = modal_content_rect(area, branch_create_spec())?;
    let rows = branch_create_rows(content);
    single_line_cursor_position(
        rows[2],
        &state.branches.create.name,
        state.branches.create.cursor,
    )
}

fn render_branch_create_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(modal) = render_modal_frame(frame, area, branch_create_spec()) else {
        return;
    };
    let rows = branch_create_rows(modal.content);
    render_section_label(frame, rows[0], "Create branch from selected branch");
    frame.render_widget(
        Paragraph::new(format!(
            "Start point: {}",
            state.branches.create.start_point
        )),
        rows[1],
    );
    render_input_block(
        frame,
        rows[2],
        "Branch name",
        vec![Line::from(state.branches.create.name.clone())],
        true,
        ModalTone::Info,
    );
    if rows.len() > 3 {
        frame.render_widget(Paragraph::new(""), rows[3]);
    }
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Info,
            &[("Enter", "create"), ("Esc", "cancel")],
        );
    }
    if let Some(cursor) = branch_create_cursor_position(state, area) {
        frame.set_cursor_position(Position::new(cursor.x, cursor.y));
    }
}

fn render_branch_delete_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(modal) = render_modal_frame(
        frame,
        area,
        ModalSpec::new("Delete Branch", ModalTone::Danger, 72, 12, 20, 8, 1),
    ) else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(modal.content);
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}",
            state.branches.delete_menu.target_branch
        )),
        rows[0],
    );
    render_choice_list(
        frame,
        rows[1],
        "Choice",
        &[
            (
                BranchDeleteChoice::Local,
                "delete local",
                modal_muted_style(),
            ),
            (
                BranchDeleteChoice::Remote,
                "delete remote",
                modal_muted_style(),
            ),
            (
                BranchDeleteChoice::Both,
                "delete local and remote",
                modal_muted_style(),
            ),
        ],
        state.branches.delete_menu.selected,
        ModalTone::Danger,
    );
    render_section_label(frame, rows[2], "Description");
    frame.render_widget(
        Paragraph::new(branch_delete_description(
            state.branches.delete_menu.selected,
        ))
        .wrap(Wrap { trim: false }),
        rows[3],
    );
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Danger,
            &[("j/k", "select"), ("Enter", "delete"), ("Esc", "cancel")],
        );
    }
}

fn render_branch_rebase_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(modal) = render_modal_frame(
        frame,
        area,
        ModalSpec::new("Rebase", ModalTone::Warning, 74, 12, 20, 8, 1),
    ) else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(4),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(modal.content);
    frame.render_widget(
        Paragraph::new(format!(
            "Selected target: {}",
            state.branches.rebase_menu.target_branch
        )),
        rows[0],
    );
    render_choice_list(
        frame,
        rows[1],
        "Choice",
        &[
            (
                BranchRebaseChoice::Simple,
                "simple rebase",
                modal_muted_style(),
            ),
            (
                BranchRebaseChoice::Interactive,
                "interactive rebase",
                modal_muted_style(),
            ),
            (
                BranchRebaseChoice::OriginMain,
                "rebase onto origin/main",
                modal_muted_style(),
            ),
        ],
        state.branches.rebase_menu.selected,
        ModalTone::Warning,
    );
    render_section_label(frame, rows[2], "Description");
    frame.render_widget(
        Paragraph::new(branch_rebase_description(
            state.branches.rebase_menu.selected,
            &state.branches.rebase_menu.target_branch,
        ))
        .wrap(Wrap { trim: false }),
        rows[3],
    );
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Warning,
            &[("j/k", "select"), ("Enter", "rebase"), ("Esc", "cancel")],
        );
    }
}

fn render_force_delete_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(modal) = render_modal_frame(
        frame,
        area,
        ModalSpec::new("Force Delete Branch", ModalTone::Danger, 76, 12, 20, 8, 1),
    ) else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(modal.content);
    render_warning_text(
        frame,
        rows[0],
        ModalTone::Danger,
        "Branch is not fully merged.",
    );
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}",
            state.branches.force_delete_confirm.target_branch
        )),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new(format!(
            "Git refused safe deletion:\n{}",
            state.branches.force_delete_confirm.reason
        ))
        .wrap(Wrap { trim: false }),
        rows[2],
    );
    render_warning_text(
        frame,
        rows[3],
        ModalTone::Danger,
        "Force delete removes the branch name even if commits are only reachable there.",
    );
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Danger,
            &[("Enter", "force delete"), ("Esc", "cancel")],
        );
    }
}

fn render_auto_stash_modal(frame: &mut Frame<'_>, state: &AppState, area: Rect) {
    let Some(modal) = render_modal_frame(
        frame,
        area,
        ModalSpec::new("Auto Stash", ModalTone::Warning, 72, 10, 20, 7, 1),
    ) else {
        return;
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(modal.content);
    render_warning_text(
        frame,
        rows[0],
        ModalTone::Warning,
        "Working tree has uncommitted changes.",
    );
    render_muted_text(
        frame,
        rows[1],
        "Auto stash before the operation, then restore the stash afterward?",
    );
    render_section_label(frame, rows[2], "Operation");
    frame.render_widget(
        Paragraph::new(auto_stash_operation_label(
            state.branches.auto_stash_confirm.operation.as_ref(),
        )),
        rows[3],
    );
    if let Some(footer) = modal.footer {
        render_action_footer(
            frame,
            footer,
            ModalTone::Warning,
            &[("Enter", "auto stash"), ("Esc", "cancel")],
        );
    }
}

fn branch_create_spec() -> ModalSpec {
    ModalSpec::new("Create Branch", ModalTone::Info, 72, 10, 20, 7, 1)
}

fn branch_create_rows(area: Rect) -> std::rc::Rc<[Rect]> {
    Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(1),
        ])
        .split(area)
}

fn branch_delete_description(choice: BranchDeleteChoice) -> &'static str {
    match choice {
        BranchDeleteChoice::Local => "Delete the selected local branch with `git branch -d`.",
        BranchDeleteChoice::Remote => "Delete `origin/<branch>` with `git push origin --delete`.",
        BranchDeleteChoice::Both => "Delete the local branch first, then delete `origin/<branch>`.",
    }
}

fn branch_rebase_description(choice: BranchRebaseChoice, selected_target: &str) -> String {
    match choice {
        BranchRebaseChoice::Simple => {
            format!("Rebase the current branch onto `{selected_target}`.")
        }
        BranchRebaseChoice::Interactive => {
            format!("Run interactive rebase of the current branch onto `{selected_target}`.")
        }
        BranchRebaseChoice::OriginMain => {
            "Rebase the current branch onto `origin/main`.".to_string()
        }
    }
}

fn auto_stash_operation_label(operation: Option<&AutoStashOperation>) -> String {
    match operation {
        Some(AutoStashOperation::Checkout { branch }) => format!("checkout {branch}"),
        Some(AutoStashOperation::CheckoutCommitDetached { commit_id }) => {
            format!("checkout detached {commit_id}")
        }
        Some(AutoStashOperation::Rebase {
            target,
            interactive,
        }) => {
            let mode = if *interactive {
                "interactive"
            } else {
                "simple"
            };
            format!("rebase ({mode}) onto {target}")
        }
        None => "unknown".to_string(),
    }
}

fn single_line_cursor_position(area: Rect, text: &str, cursor: usize) -> Option<TerminalCursor> {
    if area.width < 3 || area.height < 3 {
        return None;
    }
    let cursor = cursor.min(text.len());
    let before_cursor = text.get(..cursor).unwrap_or(text);
    let content_width = area.width.saturating_sub(2).max(1) as usize;
    let text_width = UnicodeWidthStr::width(before_cursor);
    let x = area
        .x
        .saturating_add(1)
        .saturating_add(text_width.min(content_width.saturating_sub(1)) as u16);
    Some(TerminalCursor {
        x,
        y: area.y.saturating_add(1),
    })
}
