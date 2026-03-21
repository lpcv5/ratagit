use crate::app::{App, Command, Message, RefreshKind};

pub(crate) fn handle_branch_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::StartBranchCreateInput => {
            app.start_branch_create_input();
            app.push_log("branch create: enter name and press Enter", true);
            app.dirty.mark();
        }
        Message::CreateBranch(name) => match app.create_branch(&name) {
            Ok(()) => {
                app.push_log(format!("branch created: {}", name), true);
                app.dirty.mark();
            }
            Err(e) => app.push_log(format!("create branch failed: {}", e), false),
        },
        Message::CheckoutSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                app.request_refresh(RefreshKind::StatusOnly);
                if let Err(e) = app.flush_pending_refresh() {
                    app.push_log(format!("refresh failed: {}", e), false);
                    return None;
                }

                if app.has_uncommitted_changes() {
                    app.start_branch_switch_confirm(name);
                    app.dirty.mark_overlay();
                    return None;
                }

                match app.checkout_branch(&name) {
                    Ok(()) => {
                        app.push_log(format!("switched to {}", name), true);
                        app.dirty.mark();
                    }
                    Err(e) => app.push_log(format!("switch failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
        }
        Message::BranchSwitchConfirm(auto_stash) => {
            let Some(target) = app.take_branch_switch_target() else {
                app.cancel_input();
                app.dirty.mark_overlay();
                return None;
            };
            app.cancel_input();
            if !auto_stash {
                app.push_log(format!("switch canceled: {}", target), false);
                app.dirty.mark();
                return None;
            }

            match app.checkout_branch_with_auto_stash(&target) {
                Ok(()) => {
                    app.push_log(format!("switched with auto stash: {}", target), true);
                    app.dirty.mark();
                }
                Err(e) => app.push_log(format!("auto-stash switch failed: {}", e), false),
            }
        }
        Message::DeleteSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                match app.delete_branch(&name) {
                    Ok(()) => {
                        app.push_log(format!("deleted branch {}", name), true);
                        app.dirty.mark();
                    }
                    Err(e) => app.push_log(format!("delete branch failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
        }
        Message::FetchRemote => {
            if app.branches.is_fetching_remote {
                app.push_log("fetch already running", false);
                return None;
            }
            match app.fetch_remote_async() {
                Ok(rx) => {
                    app.branches.is_fetching_remote = true;
                    app.push_log("fetch started", true);
                    app.dirty.mark();
                    return Some(Command::Async(rx));
                }
                Err(e) => app.push_log(format!("fetch start failed: {}", e), false),
            }
        }
        Message::FetchRemoteFinished(result) => {
            app.branches.is_fetching_remote = false;
            match result {
                Ok(remote) => {
                    app.request_refresh(RefreshKind::Full);
                    app.push_log(format!("fetched {}", remote), true);
                    app.dirty.mark();
                }
                Err(e) => app.push_log(format!("fetch failed: {}", e), false),
            }
        }
        _ => {}
    }
    None
}
