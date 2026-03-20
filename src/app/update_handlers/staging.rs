use crate::app::{App, Command, Message};

pub(crate) fn handle_staging_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::StartCommitInput => {
            if app.start_commit_editor_guarded() {
                app.push_log("commit: edit message/description then press Enter on message", true);
            }
        }
        Message::PrepareCommitFromSelection => match app.prepare_commit_from_visual_selection() {
            Ok(count) => {
                if count == 0 {
                    app.push_log("commit blocked: no selected items", false);
                    return None;
                }
                if app.start_commit_editor_guarded() {
                    app.push_log(
                        format!("commit: {} selected target(s) staged; edit message/description", count),
                        true,
                    );
                }
            }
            Err(e) => app.push_log(format!("prepare commit failed: {}", e), false),
        },
        Message::ToggleStageSelection => match app.toggle_stage_visual_selection() {
            Ok((staged, unstaged)) => {
                app.push_log(
                    format!("selection toggled: staged {}, unstaged {}", staged, unstaged),
                    true,
                );
                app.load_diff();
            }
            Err(e) => app.push_log(format!("selection toggle failed: {}", e), false),
        },
        Message::StageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.stage_file(path) {
                app.push_log(format!("stage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("staged {}", display), true);
            }
        }
        Message::UnstageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.unstage_file(path) {
                app.push_log(format!("unstage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("unstaged {}", display), true);
            }
        }
        _ => {}
    }
    None
}
