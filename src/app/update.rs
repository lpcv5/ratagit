use super::update_handlers::{
    handle_branch_message, handle_commit_message, handle_files_message, handle_navigation_message,
    handle_quit, handle_revision_message, handle_search_message, handle_staging_message,
    handle_stash_message,
};
use super::{App, Command, GlobalMessage, Message};

/// Documentation comment in English.
pub fn update(app: &mut App, msg: Message) -> Option<Command> {
    if let Some(global) = msg.as_global() {
        return match global {
            GlobalMessage::Quit => handle_quit(app),
            GlobalMessage::PanelNext
            | GlobalMessage::PanelPrev
            | GlobalMessage::ListDown
            | GlobalMessage::ListUp
            | GlobalMessage::DiffScrollUp
            | GlobalMessage::DiffScrollDown
            | GlobalMessage::RefreshStatus => handle_navigation_message(app, msg),
            GlobalMessage::PanelGoto(panel_index) => {
                let _ = panel_index;
                handle_navigation_message(app, msg)
            }
        };
    }

    match msg {
        Message::ToggleDir
        | Message::ToggleVisualSelectMode
        | Message::CollapseAll
        | Message::ExpandAll
        | Message::StageFile(_)
        | Message::UnstageFile(_)
        | Message::DiscardPaths(_)
        | Message::ToggleStageSelection
        | Message::DiscardSelection
        | Message::PrepareCommitFromSelection => handle_files_message(app, msg),
        Message::RevisionOpenTreeOrToggleDir | Message::RevisionCloseTree => {
            handle_revision_message(app, msg)
        }
        Message::StartBranchCreateInput
        | Message::CreateBranch(_)
        | Message::CheckoutSelectedBranch
        | Message::BranchSwitchConfirm(_)
        | Message::DeleteSelectedBranch
        | Message::FetchRemote
        | Message::FetchRemoteFinished(_) => handle_branch_message(app, msg),
        Message::StartStashInput
        | Message::StashPush { .. }
        | Message::StashApplySelected
        | Message::StashPopSelected
        | Message::StashDropSelected => handle_stash_message(app, msg),
        Message::StartCommitInput => handle_staging_message(app, msg),
        Message::StartSearchInput
        | Message::SearchSetQuery(_)
        | Message::SearchConfirm
        | Message::SearchClear
        | Message::SearchNext
        | Message::SearchPrev => handle_search_message(app, msg),
        Message::Commit(message) => handle_commit_message(app, message),
        Message::Quit
        | Message::PanelNext
        | Message::PanelPrev
        | Message::PanelGoto(_)
        | Message::ListDown
        | Message::ListUp
        | Message::DiffScrollUp
        | Message::DiffScrollDown
        | Message::RefreshStatus => None,
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
