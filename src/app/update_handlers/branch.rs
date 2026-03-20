use crate::app::{App, Command, Message, RefreshKind};

pub(crate) fn handle_branch_message(app: &mut App, msg: Message) -> Option<Command> {
    match msg {
        Message::StartBranchCreateInput => {
            app.start_branch_create_input();
            app.push_log("branch create: enter name and press Enter", true);
        }
        Message::CreateBranch(name) => match app.create_branch(&name) {
            Ok(()) => app.push_log(format!("branch created: {}", name), true),
            Err(e) => app.push_log(format!("create branch failed: {}", e), false),
        },
        Message::CheckoutSelectedBranch => {
            if let Some(name) = app.selected_branch_name() {
                match app.checkout_branch(&name) {
                    Ok(()) => app.push_log(format!("checked out {}", name), true),
                    Err(e) => app.push_log(format!("checkout failed: {}", e), false),
                }
            } else {
                app.push_log("no branch selected", false);
            }
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
        }
        Message::FetchRemote => {
            if app.is_fetching_remote {
                app.push_log("fetch already running", false);
                return None;
            }
            match app.fetch_remote_async() {
                Ok(rx) => {
                    app.is_fetching_remote = true;
                    app.push_log("fetch started", true);
                    return Some(Command::Async(rx));
                }
                Err(e) => app.push_log(format!("fetch start failed: {}", e), false),
            }
        }
        Message::FetchRemoteFinished(result) => {
            app.is_fetching_remote = false;
            match result {
                Ok(remote) => {
                    app.request_refresh(RefreshKind::Full);
                    app.push_log(format!("fetched {}", remote), true);
                }
                Err(e) => app.push_log(format!("fetch failed: {}", e), false),
            }
        }
        _ => {}
    }
    None
}
