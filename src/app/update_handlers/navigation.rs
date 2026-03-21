use crate::app::{App, Command, Message, RefreshKind, SidePanel};

pub(crate) fn handle_navigation_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::PanelNext => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::LocalBranches,
                SidePanel::LocalBranches => SidePanel::Commits,
                SidePanel::Commits => SidePanel::Stash,
                SidePanel::Stash => SidePanel::Files,
            };
            app.ensure_commits_loaded_for_active_panel();
            app.restore_search_for_active_scope();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::PanelPrev => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::Stash,
                SidePanel::LocalBranches => SidePanel::Files,
                SidePanel::Commits => SidePanel::LocalBranches,
                SidePanel::Stash => SidePanel::Commits,
            };
            app.ensure_commits_loaded_for_active_panel();
            app.restore_search_for_active_scope();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::PanelGoto(n) => {
            app.active_panel = match n {
                1 => SidePanel::Files,
                2 => SidePanel::LocalBranches,
                3 => SidePanel::Commits,
                4 => SidePanel::Stash,
                _ => app.active_panel,
            };
            app.ensure_commits_loaded_for_active_panel();
            app.restore_search_for_active_scope();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::ListDown => {
            app.list_down();
            app.schedule_diff_reload();
            app.dirty.mark_main_content();
        }
        Message::ListUp => {
            app.list_up();
            app.schedule_diff_reload();
            app.dirty.mark_main_content();
        }
        Message::ToggleDir => {
            app.toggle_selected_dir();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::ToggleVisualSelectMode => {
            app.toggle_visual_select_mode();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::CollapseAll => {
            app.collapse_all();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::ExpandAll => {
            app.expand_all();
            app.reload_diff_now();
            app.dirty.mark_main_content();
        }
        Message::DiffScrollUp => {
            app.diff_scroll_up();
            app.dirty.mark_diff();
        }
        Message::DiffScrollDown => {
            app.diff_scroll_down();
            app.dirty.mark_diff();
        }
        Message::RefreshStatus => {
            app.request_refresh(RefreshKind::Full);
            if let Err(e) = app.flush_pending_refresh() {
                app.push_log(format!("refresh failed: {}", e), false);
            } else {
                app.push_log("refresh", true);
            }
            app.dirty.mark_all();
        }
        _ => {}
    }
    None
}
