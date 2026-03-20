use crate::app::{App, Command, Message, SidePanel};

pub(crate) fn handle_revision_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::RevisionOpenTreeOrToggleDir => {
            let result = match app.active_panel {
                SidePanel::Stash => app.stash_open_tree_or_toggle_dir(),
                SidePanel::Commits => app.commit_open_tree_or_toggle_dir(),
                _ => Ok(()),
            };
            match result {
                Ok(()) => {
                    app.restore_search_for_active_scope();
                    app.load_diff();
                    app.dirty.mark();
                }
                Err(e) => app.push_log(format!("revision files failed: {}", e), false),
            }
        }
        Message::RevisionCloseTree => {
            match app.active_panel {
                SidePanel::Stash => app.stash_close_tree(),
                SidePanel::Commits => app.commit_close_tree(),
                _ => {}
            }
            app.restore_search_for_active_scope();
            app.load_diff();
            app.dirty.mark();
        }
        _ => {}
    }
    None
}
