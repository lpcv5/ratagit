use crate::app::{App, Command};

pub(crate) fn handle_commit_message(app: &mut App, message: String) -> Option<Command> {
    match app.commit(&message) {
        Ok(oid) => app.push_log(format!("commit {} ({})", message, oid), true),
        Err(e) => app.push_log(format!("commit failed: {}", e), false),
    }
    None
}
