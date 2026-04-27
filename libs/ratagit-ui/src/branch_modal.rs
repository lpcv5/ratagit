use ratagit_core::{
    AppContext, AutoStashOperation, BranchDeleteChoice, BranchDeleteMode, BranchRebaseChoice,
};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

use crate::frame::TerminalCursor;
use crate::modal::{
    ChoiceMenuBody, ConfirmBody, ModalSpec, ModalTone, choice_menu_modal_height,
    modal_content_rect, render_choice_menu_body, render_confirm_body, render_input_block,
    render_modal, render_section_label, render_text,
};
use crate::theme::modal_muted_style;

pub(crate) fn render_branch_modals(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    if state.ui.branches.create.active {
        render_branch_create_modal(frame, state, area);
    }
    if state.ui.branches.delete_menu.active {
        render_branch_delete_modal(frame, state, area);
    }
    if state.ui.branches.delete_confirm.active {
        render_branch_delete_confirm_modal(frame, state, area);
    }
    if state.ui.branches.force_delete_confirm.active {
        render_force_delete_modal(frame, state, area);
    }
    if state.ui.branches.rebase_menu.active {
        render_branch_rebase_modal(frame, state, area);
    }
    if state.ui.branches.auto_stash_confirm.active {
        render_auto_stash_modal(frame, state, area);
    }
}

fn render_branch_delete_confirm_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    let mode = state
        .ui
        .branches
        .delete_confirm
        .mode
        .unwrap_or(BranchDeleteMode::Remote);
    let branch = &state.ui.branches.delete_confirm.target_branch;
    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Danger, 76, 12, 20, 8, 1),
        &[&[
            ("Enter", branch_delete_confirm_action(mode)),
            ("Esc", "cancel"),
        ]],
        |frame, content| {
            render_confirm_body(
                frame,
                content,
                ModalTone::Danger,
                ConfirmBody::new(branch_delete_confirm_question(mode, branch))
                    .secondary("Remote branch deletion may affect other collaborators.")
                    .details(branch_delete_confirm_details(mode, branch)),
            );
        },
    );
}

pub(crate) fn branch_create_cursor_position(
    state: &AppContext,
    area: Rect,
) -> Option<TerminalCursor> {
    if !state.ui.branches.create.active {
        return None;
    }
    let content = modal_content_rect(area, branch_create_spec())?;
    let rows = branch_create_rows(content);
    single_line_cursor_position(
        rows[2],
        &state.ui.branches.create.name,
        state.ui.branches.create.cursor,
    )
}

fn render_branch_create_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    let rendered = render_modal(
        frame,
        area,
        branch_create_spec(),
        &[&[("Enter", "create"), ("Esc", "cancel")]],
        |frame, content| {
            let rows = branch_create_rows(content);
            render_section_label(frame, rows[0], "Create branch from selected branch");
            render_text(
                frame,
                rows[1],
                format!("Start point: {}", state.ui.branches.create.start_point),
            );
            render_input_block(
                frame,
                rows[2],
                "Branch name",
                vec![Line::from(state.ui.branches.create.name.clone())],
                true,
                ModalTone::Info,
            );
            if rows.len() > 3 {
                frame.render_widget(Paragraph::new(""), rows[3]);
            }
        },
    );
    if rendered.is_some()
        && let Some(cursor) = branch_create_cursor_position(state, area)
    {
        frame.set_cursor_position(Position::new(cursor.x, cursor.y));
    }
}

fn render_branch_delete_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    let choices = branch_delete_choices();
    render_modal(
        frame,
        area,
        ModalSpec::new(
            "Delete Branch",
            ModalTone::Danger,
            72,
            choice_menu_modal_height(choices.len(), 1),
            20,
            8,
            1,
        ),
        &[&[("j/k", "select"), ("Enter", "delete"), ("Esc", "cancel")]],
        |frame, content| {
            render_choice_menu_body(
                frame,
                content,
                ModalTone::Danger,
                ChoiceMenuBody {
                    intro: format!("Target: {}", state.ui.branches.delete_menu.target_branch),
                    list_title: "Choice",
                    choices: &choices,
                    selected: state.ui.branches.delete_menu.selected,
                    list_height: 4,
                    description: branch_delete_description(state.ui.branches.delete_menu.selected)
                        .to_string(),
                },
            );
        },
    );
}

fn render_branch_rebase_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    let choices = branch_rebase_choices();
    render_modal(
        frame,
        area,
        ModalSpec::new(
            "Rebase",
            ModalTone::Warning,
            74,
            choice_menu_modal_height(choices.len(), 1),
            20,
            8,
            1,
        ),
        &[&[("j/k", "select"), ("Enter", "rebase"), ("Esc", "cancel")]],
        |frame, content| {
            render_choice_menu_body(
                frame,
                content,
                ModalTone::Warning,
                ChoiceMenuBody {
                    intro: format!(
                        "Selected target: {}",
                        state.ui.branches.rebase_menu.target_branch
                    ),
                    list_title: "Choice",
                    choices: &choices,
                    selected: state.ui.branches.rebase_menu.selected,
                    list_height: 4,
                    description: branch_rebase_description(
                        state.ui.branches.rebase_menu.selected,
                        &state.ui.branches.rebase_menu.target_branch,
                    ),
                },
            );
        },
    );
}

fn render_force_delete_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Danger, 76, 12, 20, 8, 1),
        &[&[("Enter", "force delete"), ("Esc", "cancel")]],
        |frame, content| {
            render_confirm_body(
                frame,
                content,
                ModalTone::Danger,
                ConfirmBody::new(format!(
                    "Force delete branch: {}?",
                    state.ui.branches.force_delete_confirm.target_branch
                ))
                .secondary("This action cannot be undone.")
                .details(format!(
                    "Git refused safe deletion:\n{}",
                    state.ui.branches.force_delete_confirm.reason
                )),
            );
        },
    );
}

fn render_auto_stash_modal(frame: &mut Frame<'_>, state: &AppContext, area: Rect) {
    render_modal(
        frame,
        area,
        ModalSpec::new("Confirm", ModalTone::Warning, 72, 10, 20, 7, 1),
        &[&[("Enter", "auto stash"), ("Esc", "cancel")]],
        |frame, content| {
            let operation = state.ui.branches.auto_stash_confirm.operation.as_ref();
            render_confirm_body(
                frame,
                content,
                ModalTone::Warning,
                ConfirmBody::new(auto_stash_confirm_question(operation))
                    .secondary("The stash will be restored after the operation.")
                    .details(format!(
                        "Operation: {}",
                        auto_stash_operation_label(operation)
                    )),
            );
        },
    );
}

fn branch_create_spec() -> ModalSpec {
    ModalSpec::new("Create Branch", ModalTone::Info, 72, 10, 20, 7, 1).with_icon("")
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

fn branch_delete_choices() -> Vec<(BranchDeleteChoice, &'static str, Style)> {
    BranchDeleteChoice::ALL
        .iter()
        .map(|choice| (*choice, branch_delete_label(*choice), modal_muted_style()))
        .collect()
}

fn branch_delete_label(choice: BranchDeleteChoice) -> &'static str {
    match choice {
        BranchDeleteChoice::Local => "delete local",
        BranchDeleteChoice::Remote => "delete remote",
        BranchDeleteChoice::Both => "delete local and remote",
    }
}

fn branch_delete_description(choice: BranchDeleteChoice) -> &'static str {
    match choice {
        BranchDeleteChoice::Local => "Delete the selected local branch with `git branch -d`.",
        BranchDeleteChoice::Remote => "Delete `origin/<branch>` with `git push origin --delete`.",
        BranchDeleteChoice::Both => "Delete the local branch first, then delete `origin/<branch>`.",
    }
}

fn branch_delete_confirm_action(mode: BranchDeleteMode) -> &'static str {
    match mode {
        BranchDeleteMode::Remote => "delete remote",
        BranchDeleteMode::Both => "delete both",
        BranchDeleteMode::Local => "delete local",
    }
}

fn branch_delete_confirm_question(mode: BranchDeleteMode, branch: &str) -> String {
    match mode {
        BranchDeleteMode::Remote => format!("Delete remote branch origin/{branch}?"),
        BranchDeleteMode::Both => format!("Delete local and remote branch {branch}?"),
        BranchDeleteMode::Local => format!("Delete local branch {branch}?"),
    }
}

fn branch_delete_confirm_details(mode: BranchDeleteMode, branch: &str) -> String {
    match mode {
        BranchDeleteMode::Remote => format!("Runs `git push origin --delete {branch}`."),
        BranchDeleteMode::Both => format!("Deletes local branch, then origin/{branch}."),
        BranchDeleteMode::Local => "Runs `git branch -d`.".to_string(),
    }
}

fn branch_rebase_choices() -> Vec<(BranchRebaseChoice, &'static str, Style)> {
    BranchRebaseChoice::ALL
        .iter()
        .map(|choice| (*choice, branch_rebase_label(*choice), modal_muted_style()))
        .collect()
}

fn branch_rebase_label(choice: BranchRebaseChoice) -> &'static str {
    match choice {
        BranchRebaseChoice::Simple => "simple rebase",
        BranchRebaseChoice::Interactive => "interactive rebase",
        BranchRebaseChoice::OriginMain => "rebase onto origin/main",
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

fn auto_stash_confirm_question(operation: Option<&AutoStashOperation>) -> String {
    match operation {
        Some(AutoStashOperation::Checkout { branch }) => {
            format!("Auto stash before checkout {branch}?")
        }
        Some(AutoStashOperation::CheckoutCommitDetached { .. }) => {
            "Auto stash before detached checkout?".to_string()
        }
        Some(AutoStashOperation::Rebase { .. }) => "Auto stash before rebase?".to_string(),
        None => "Auto stash before this operation?".to_string(),
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
