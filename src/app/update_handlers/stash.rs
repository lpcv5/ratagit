use crate::app::{App, Command, Message};

pub(crate) fn handle_stash_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::StartStashInput => {
            let targets = app.prepare_stash_targets_from_selection();
            if targets.is_empty() {
                app.push_log("stash blocked: no selected items", false);
                return None;
            }
            app.start_stash_editor(targets);
            app.push_log("stash: enter title and press Enter", true);
        }
        Message::StashPush { message, paths } => match app.stash_push(&paths, &message) {
            Ok(index) => app.push_log(
                format!("stash created stash@{{{}}}: {}", index, message),
                true,
            ),
            Err(e) => app.push_log(format!("stash create failed: {}", e), false),
        },
        Message::StashApplySelected => {
            if let Some(index) = app.selected_stash_index() {
                match app.stash_apply(index) {
                    Ok(()) => app.push_log(format!("stash applied stash@{{{}}}", index), true),
                    Err(e) => app.push_log(
                        format!("stash apply failed stash@{{{}}}: {}", index, e),
                        false,
                    ),
                }
            } else {
                app.push_log("no stash selected", false);
            }
        }
        Message::StashPopSelected => {
            if let Some(index) = app.selected_stash_index() {
                match app.stash_pop(index) {
                    Ok(()) => app.push_log(format!("stash popped stash@{{{}}}", index), true),
                    Err(e) => app.push_log(
                        format!("stash pop failed stash@{{{}}}: {}", index, e),
                        false,
                    ),
                }
            } else {
                app.push_log("no stash selected", false);
            }
        }
        Message::StashDropSelected => {
            if let Some(index) = app.selected_stash_index() {
                match app.stash_drop(index) {
                    Ok(()) => app.push_log(format!("stash dropped stash@{{{}}}", index), true),
                    Err(e) => app.push_log(
                        format!("stash drop failed stash@{{{}}}: {}", index, e),
                        false,
                    ),
                }
            } else {
                app.push_log("no stash selected", false);
            }
        }
        _ => {}
    }
    None
}
