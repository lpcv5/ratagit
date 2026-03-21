use crate::app::{App, Command, Message, SidePanel};

pub(crate) fn handle_files_message(app: &mut App, msg: Message) -> Option<Command> {
    if app.active_panel != SidePanel::Files {
        return None;
    }

    match msg {
        Message::ToggleDir => {
            app.toggle_selected_dir();
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::ToggleVisualSelectMode => {
            app.toggle_visual_select_mode();
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::CollapseAll => {
            app.collapse_all();
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::ExpandAll => {
            app.expand_all();
            app.reload_diff_now();
            app.dirty.mark();
        }
        Message::StageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.stage_file(path) {
                app.push_log(format!("stage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("staged {}", display), true);
                app.dirty.mark();
            }
        }
        Message::UnstageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.unstage_file(path) {
                app.push_log(format!("unstage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("unstaged {}", display), true);
                app.dirty.mark();
            }
        }
        Message::DiscardPaths(paths) => {
            if paths.is_empty() {
                app.push_log("discard blocked: no discardable selected items", false);
                return None;
            }
            if let Err(e) = app.discard_paths(&paths) {
                app.push_log(format!("discard failed: {}", e), false);
            } else if paths.len() == 1 {
                app.push_log(format!("discarded {}", paths[0].display()), true);
                app.dirty.mark();
            } else {
                app.push_log(format!("discarded {} path(s)", paths.len()), true);
                app.dirty.mark();
            }
        }
        Message::ToggleStageSelection => match app.toggle_stage_visual_selection() {
            Ok((staged, unstaged)) => {
                app.push_log(
                    format!("selection toggled: staged {}, unstaged {}", staged, unstaged),
                    true,
                );
                app.dirty.mark();
            }
            Err(e) => app.push_log(format!("selection toggle failed: {}", e), false),
        },
        Message::DiscardSelection => {
            let paths = app.prepare_discard_targets_from_selection();
            if paths.is_empty() {
                app.push_log("discard blocked: no discardable selected items", false);
                return None;
            }
            if let Err(e) = app.discard_paths(&paths) {
                app.push_log(format!("discard failed: {}", e), false);
            } else {
                app.push_log(format!("discarded {} path(s)", paths.len()), true);
                app.files.visual_mode = false;
                app.files.visual_anchor = None;
                app.dirty.mark();
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
                        format!(
                            "commit: {} selected target(s) staged; edit message/description",
                            count
                        ),
                        true,
                    );
                    app.dirty.mark();
                }
            }
            Err(e) => app.push_log(format!("prepare commit failed: {}", e), false),
        },
        _ => {}
    }

    None
}
