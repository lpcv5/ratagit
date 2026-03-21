use crate::app::{App, Command, Message};

pub(crate) fn handle_staging_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::StartCommitInput => {
            if app.start_commit_editor_guarded() {
                app.push_log(
                    "commit: edit message/description then press Enter on message",
                    true,
                );
                app.dirty.mark();
            }
        }
        _ => {}
    }
    None
}

