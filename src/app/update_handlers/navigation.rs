use crate::app::{App, Command, Message, SidePanel};

pub(crate) fn handle_navigation_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::PanelNext => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::LocalBranches,
                SidePanel::LocalBranches => SidePanel::Commits,
                SidePanel::Commits => SidePanel::Stash,
                SidePanel::Stash => SidePanel::Files,
            };
            app.load_diff();
        }
        Message::PanelPrev => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::Stash,
                SidePanel::LocalBranches => SidePanel::Files,
                SidePanel::Commits => SidePanel::LocalBranches,
                SidePanel::Stash => SidePanel::Commits,
            };
            app.load_diff();
        }
        Message::PanelGoto(n) => {
            app.active_panel = match n {
                1 => SidePanel::Files,
                2 => SidePanel::LocalBranches,
                3 => SidePanel::Commits,
                4 => SidePanel::Stash,
                _ => app.active_panel,
            };
            app.load_diff();
        }
        Message::ListDown => {
            app.list_down();
            app.load_diff();
        }
        Message::ListUp => {
            app.list_up();
            app.load_diff();
        }
        Message::ToggleDir => {
            app.toggle_selected_dir();
            app.load_diff();
        }
        Message::ToggleVisualSelectMode => {
            app.toggle_visual_select_mode();
            app.load_diff();
        }
        Message::CollapseAll => {
            app.collapse_all();
            app.load_diff();
        }
        Message::ExpandAll => {
            app.expand_all();
            app.load_diff();
        }
        Message::DiffScrollUp => app.diff_scroll_up(),
        Message::DiffScrollDown => app.diff_scroll_down(),
        Message::RefreshStatus => {
            if let Err(e) = app.refresh_status() {
                app.push_log(format!("refresh failed: {}", e), false);
            } else {
                app.push_log("refresh", true);
                app.load_diff();
            }
        }
        _ => {}
    }
    None
}
