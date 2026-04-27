use crate::actions::with_pending;
use crate::selectors::repository_has_uncommitted_changes;
use crate::text_edit::{
    CursorMove, backspace_at_cursor, insert_char_at_cursor, move_cursor_in_text,
};
use crate::worktree::{close_discard_confirm, stash_scope_for_current_files_selection};
use crate::{
    AppState, Command, CommitEditorIntent, CommitField, CommitHashStatus, CommitInputMode,
    EditorKind, StashScope, branches, commit_key, push_notice, selected_commit,
};

pub(crate) fn open_commit_editor(state: &mut AppState) {
    state.reset_menu.active = false;
    close_discard_confirm(state);
    branches::close_popovers(state);
    state.editor.kind = Some(EditorKind::Commit {
        message: String::new(),
        message_cursor: 0,
        body: String::new(),
        body_cursor: 0,
        active_field: CommitField::Message,
        intent: CommitEditorIntent::Create,
    });
}

pub(crate) fn open_stash_editor(state: &mut AppState) {
    state.reset_menu.active = false;
    close_discard_confirm(state);
    branches::close_popovers(state);
    state.editor.kind = Some(EditorKind::Stash {
        title: String::new(),
        title_cursor: 0,
        scope: stash_scope_for_current_files_selection(state),
    });
}

pub(crate) fn open_commit_reword_editor(state: &mut AppState) {
    if state.commits.mode == CommitInputMode::MultiSelect {
        push_notice(state, "Reword supports one commit at a time");
        return;
    }
    if repository_has_uncommitted_changes(state) {
        push_notice(state, "Commit rewrite requires a clean working tree");
        return;
    }
    let Some(commit) = selected_commit(&state.commits) else {
        push_notice(state, "No commit selected");
        return;
    };
    if commit.is_merge {
        push_notice(state, "Commit rewrite does not support merge commits yet");
        return;
    }
    if commit.hash_status != CommitHashStatus::Unpushed {
        push_notice(state, "Commit rewrite only supports unpushed commits");
        return;
    }
    state.reset_menu.active = false;
    close_discard_confirm(state);
    branches::close_popovers(state);
    let (message, body) = split_commit_message(&commit.message);
    state.editor.kind = Some(EditorKind::Commit {
        message_cursor: message.len(),
        body_cursor: body.len(),
        message,
        body,
        active_field: CommitField::Message,
        intent: CommitEditorIntent::Reword {
            commit_id: commit_key(&commit),
        },
    });
}

pub(crate) fn input_char(state: &mut AppState, ch: char) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => match active_field {
            CommitField::Message => insert_char_at_cursor(message, message_cursor, ch),
            CommitField::Body => insert_char_at_cursor(body, body_cursor, ch),
        },
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => insert_char_at_cursor(title, title_cursor, ch),
    }
}

pub(crate) fn backspace(state: &mut AppState) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => match active_field {
            CommitField::Message => backspace_at_cursor(message, message_cursor),
            CommitField::Body => backspace_at_cursor(body, body_cursor),
        },
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => backspace_at_cursor(title, title_cursor),
    }
}

pub(crate) fn move_cursor(state: &mut AppState, movement: CursorMove) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => match active_field {
            CommitField::Message => move_cursor_in_text(message, message_cursor, movement),
            CommitField::Body => move_cursor_in_text(body, body_cursor, movement),
        },
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => move_cursor_in_text(title, title_cursor, movement),
    }
}

pub(crate) fn switch_field(state: &mut AppState, previous: bool) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    if let EditorKind::Commit { active_field, .. } = editor {
        *active_field = if previous {
            active_field.prev()
        } else {
            active_field.next()
        };
    }
}

pub(crate) fn insert_newline(state: &mut AppState) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    if let EditorKind::Commit {
        body,
        body_cursor,
        active_field: CommitField::Body,
        ..
    } = editor
    {
        insert_char_at_cursor(body, body_cursor, '\n');
    }
}

pub(crate) fn confirm(state: &mut AppState) -> Vec<Command> {
    let Some(editor) = state.editor.kind.clone() else {
        return Vec::new();
    };

    match editor {
        EditorKind::Commit {
            message,
            body,
            intent,
            ..
        } => {
            if message.trim().is_empty() {
                push_notice(state, "Commit message cannot be empty");
                return Vec::new();
            }

            let commit_message = build_commit_message(&message, &body);
            state.commits.draft_message = message.trim().to_string();
            state.editor.kind = None;
            match intent {
                CommitEditorIntent::Create => with_pending(
                    state,
                    vec![Command::CreateCommit {
                        message: commit_message,
                    }],
                ),
                CommitEditorIntent::Reword { commit_id } => with_pending(
                    state,
                    vec![Command::RewordCommit {
                        commit_id,
                        message: commit_message,
                    }],
                ),
            }
        }
        EditorKind::Stash { title, scope, .. } => match scope {
            StashScope::All => {
                state.editor.kind = None;
                with_pending(state, vec![Command::StashPush { message: title }])
            }
            StashScope::SelectedPaths(paths) => {
                if paths.is_empty() {
                    push_notice(state, "No file selected");
                    return Vec::new();
                }

                state.editor.kind = None;
                with_pending(
                    state,
                    vec![Command::StashFiles {
                        message: title,
                        paths,
                    }],
                )
            }
        },
    }
}

fn build_commit_message(subject: &str, body: &str) -> String {
    let clean_subject = subject.trim();
    let clean_body = body.trim_end();
    if clean_body.is_empty() {
        clean_subject.to_string()
    } else {
        format!("{clean_subject}\n\n{clean_body}")
    }
}

fn split_commit_message(message: &str) -> (String, String) {
    let clean = message.trim_end();
    let mut parts = clean.splitn(2, '\n');
    let subject = parts.next().unwrap_or("").trim().to_string();
    let remainder = parts.next().unwrap_or("");
    let body = remainder.strip_prefix('\n').unwrap_or(remainder);
    let body = body.trim_end().to_string();
    (subject, body)
}
