use crate::app::{App, Command, Message};

pub(crate) fn handle_staging_message(app: &mut App, msg: Message) -> Option<Command> {
    if let Message::StartCommitInput = msg {
        if app.start_commit_editor_guarded() {
            app.push_log(
                "commit: edit message/description then press Enter on message",
                true,
            );
            app.dirty.mark();
        }
    }

    None
}
