use super::{App, Message, Command, Tab, SidePanel};

/// TEA 架构的更新函数（纯函数）
pub fn update(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::Quit => {
            app.running = false;
            None
        }

        Message::TabNext => {
            app.current_tab = match app.current_tab {
                Tab::Status => Tab::Commits,
                Tab::Commits => Tab::Branches,
                Tab::Branches => Tab::Stash,
                Tab::Stash => Tab::Status,
            };
            None
        }

        Message::TabPrev => {
            app.current_tab = match app.current_tab {
                Tab::Status => Tab::Stash,
                Tab::Commits => Tab::Status,
                Tab::Branches => Tab::Commits,
                Tab::Stash => Tab::Branches,
            };
            None
        }

        Message::PanelNext => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::LocalBranches,
                SidePanel::LocalBranches => SidePanel::Commits,
                SidePanel::Commits => SidePanel::Stash,
                SidePanel::Stash => SidePanel::Files,
            };
            None
        }

        Message::PanelPrev => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::Stash,
                SidePanel::LocalBranches => SidePanel::Files,
                SidePanel::Commits => SidePanel::LocalBranches,
                SidePanel::Stash => SidePanel::Commits,
            };
            None
        }

        Message::PanelGoto(n) => {
            app.active_panel = match n {
                1 => SidePanel::Files,
                2 => SidePanel::LocalBranches,
                3 => SidePanel::Commits,
                4 => SidePanel::Stash,
                _ => app.active_panel,
            };
            None
        }

        Message::ListDown => {
            app.list_down();
            app.load_diff();
            None
        }

        Message::ListUp => {
            app.list_up();
            app.load_diff();
            None
        }

        Message::ToggleDir => {
            app.toggle_selected_dir();
            app.load_diff();
            None
        }

        Message::CollapseAll => {
            app.collapse_all();
            app.load_diff();
            None
        }

        Message::ExpandAll => {
            app.expand_all();
            app.load_diff();
            None
        }

        Message::DiffScrollUp => {
            app.diff_scroll_up();
            None
        }

        Message::DiffScrollDown => {
            app.diff_scroll_down();
            None
        }

        Message::RefreshStatus => {
            if let Err(e) = app.refresh_status() {
                eprintln!("Failed to refresh status: {}", e);
            }
            None
        }

        Message::StageFile(path) => {
            if let Err(e) = app.stage_file(path) {
                eprintln!("Failed to stage file: {}", e);
            }
            None
        }

        Message::UnstageFile(path) => {
            if let Err(e) = app.unstage_file(path) {
                eprintln!("Failed to unstage file: {}", e);
            }
            None
        }

        Message::GitStatusLoaded(status) => {
            app.status = status;
            None
        }

        Message::GitError(e) => {
            eprintln!("Git error: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_update_tab_next() {
        // Phase 2 再完善测试
    }
}
