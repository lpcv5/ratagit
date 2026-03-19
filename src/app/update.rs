use super::{App, Message, Command, SidePanel};

/// Documentation comment in English.
pub fn update(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::Quit => {
            app.running = false;
            None
        }

        Message::PanelNext => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::LocalBranches,
                SidePanel::LocalBranches => SidePanel::Commits,
                SidePanel::Commits => SidePanel::Stash,
                SidePanel::Stash => SidePanel::Files,
            };
            app.load_diff();
            None
        }

        Message::PanelPrev => {
            app.active_panel = match app.active_panel {
                SidePanel::Files => SidePanel::Stash,
                SidePanel::LocalBranches => SidePanel::Files,
                SidePanel::Commits => SidePanel::LocalBranches,
                SidePanel::Stash => SidePanel::Commits,
            };
            app.load_diff();
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
            app.load_diff();
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
                app.push_log(format!("refresh failed: {}", e), false);
            } else {
                app.push_log("refresh", true);
                app.load_diff();
            }
            None
        }

        Message::StartCommitInput => {
            app.start_commit_input();
            app.push_log("commit: enter message and press Enter", true);
            None
        }

        Message::StartBranchCreateInput => {
            app.start_branch_create_input();
            app.push_log("branch create: enter name and press Enter", true);
            None
        }

        Message::Commit(message) => {
            match app.commit(&message) {
                Ok(oid) => app.push_log(format!("commit {} ({})", message, oid), true),
                Err(e) => app.push_log(format!("commit failed: {}", e), false),
            }
            None
        }

        Message::CreateBranch(name) => {
            match app.create_branch(&name) {
                Ok(()) => app.push_log(format!("branch created: {}", name), true),
                Err(e) => app.push_log(format!("create branch failed: {}", e), false),
            }
            None
        }

        Message::CheckoutSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                match app.checkout_branch(&name) {
                    Ok(()) => app.push_log(format!("checked out {}", name), true),
                    Err(e) => app.push_log(format!("checkout failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
            None
        }

        Message::DeleteSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                match app.delete_branch(&name) {
                    Ok(()) => app.push_log(format!("deleted branch {}", name), true),
                    Err(e) => app.push_log(format!("delete branch failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
            None
        }

        Message::StageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.stage_file(path) {
                app.push_log(format!("stage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("staged {}", display), true);
            }
            None
        }

        Message::UnstageFile(path) => {
            let display = path.display().to_string();
            if let Err(e) = app.unstage_file(path) {
                app.push_log(format!("unstage failed {}: {}", display, e), false);
            } else {
                app.push_log(format!("unstaged {}", display), true);
            }
            None
        }

    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_update_tab_next() {
        // Comment in English.
    }
}
