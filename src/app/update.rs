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

        Message::ListDown => {
            app.list_down();
            None
        }

        Message::ListUp => {
            app.list_up();
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
    use super::*;

    #[test]
    fn test_update_tab_next() {
        // 这个测试需要创建一个临时的 Git 仓库
        // Phase 2 再完善测试
    }
}
