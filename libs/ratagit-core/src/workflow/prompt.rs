use super::*;
use crate::text_edit::{
    CursorMove, backspace_at_cursor, insert_char_at_cursor, move_cursor_in_text,
};

pub(super) fn open(state: &mut AppContext, kind: PromptKind, target: String) {
    let text = if kind == PromptKind::Rename {
        target.clone()
    } else {
        String::new()
    };
    let cursor = text.len();
    state.ui.workflow.dialog = Some(Dialog::Prompt {
        kind,
        target,
        text,
        cursor,
    });
}

pub(super) fn edit(state: &mut AppContext, action: WorkflowAction) {
    match state.ui.workflow.dialog.as_mut() {
        Some(Dialog::Menu {
            selected, choices, ..
        }) => match action {
            WorkflowAction::Up => *selected = selected.saturating_sub(1),
            WorkflowAction::Down => {
                *selected = (*selected + 1).min(choices.len().saturating_sub(1))
            }
            _ => {}
        },
        Some(Dialog::Prompt { text, cursor, .. }) => match action {
            WorkflowAction::Input(ch) if !ch.is_control() => {
                insert_char_at_cursor(text, cursor, ch)
            }
            WorkflowAction::Backspace => backspace_at_cursor(text, cursor),
            WorkflowAction::CursorLeft => move_cursor_in_text(text, cursor, CursorMove::Left),
            WorkflowAction::CursorRight => move_cursor_in_text(text, cursor, CursorMove::Right),
            WorkflowAction::CursorHome => move_cursor_in_text(text, cursor, CursorMove::Home),
            WorkflowAction::CursorEnd => move_cursor_in_text(text, cursor, CursorMove::End),
            _ => {}
        },
        _ => {}
    }
}

pub(super) fn confirm(
    state: &mut AppContext,
    kind: PromptKind,
    target: String,
    text: String,
) -> Vec<Command> {
    let value = text.trim().to_string();
    if value.is_empty() {
        crate::push_notice(state, "A value is required");
        state.ui.workflow.dialog = Some(Dialog::Prompt {
            kind,
            target,
            cursor: text.len(),
            text,
        });
        return Vec::new();
    }
    use PromptKind as P;
    let operation = match kind {
        P::Rename => Operation::Rename {
            old: target,
            new: value,
        },
        P::Upstream => Operation::Upstream {
            branch: target,
            target: Some(value),
        },
        P::Checkout => return super::open::checkout(state, value),
        P::Branch => Operation::CreateBranch {
            name: value,
            target,
        },
        P::Tag => Operation::Tag {
            name: value,
            target,
        },
        P::CommitNoVerify => Operation::Commit {
            message: value,
            skip_hooks: true,
        },
    };
    request(state, operation)
}

pub fn title(kind: PromptKind) -> &'static str {
    match kind {
        PromptKind::Rename => "Rename branch",
        PromptKind::Upstream => "Set upstream (remote/branch)",
        PromptKind::Checkout => "Checkout branch by name (- for previous)",
        PromptKind::Branch => "New branch from selected target (tracks remote)",
        PromptKind::Tag => "Tag selected commit",
        PromptKind::CommitNoVerify => "Commit without hooks",
    }
}
