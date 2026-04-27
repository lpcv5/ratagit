use crate::{
    AppState, Command, GitResult, commit_workflow, details, operations, push_notice, snapshot,
};

pub(crate) fn update_git_result(state: &mut AppState, result: GitResult) -> Vec<Command> {
    match result {
        GitResult::Refreshed(repo_snapshot) => {
            state.work.refresh_pending = false;
            state.work.last_completed_command = Some("refresh".to_string());
            snapshot::apply_snapshot(state, repo_snapshot);
            state.status.refresh_count = state.status.refresh_count.saturating_add(1);
            state.status.last_error = None;
            details::refresh_for_focus(state)
        }
        GitResult::CommitsPage {
            offset,
            limit,
            epoch,
            result,
        } => commit_workflow::handle_commits_page_result(state, offset, limit, epoch, result),
        GitResult::FilesDetailsDiff { paths, result } => {
            details::apply_files_diff_result(state, paths, result)
        }
        GitResult::BranchDetailsLog { branch, result } => {
            details::apply_branch_log_result(state, branch, result)
        }
        GitResult::CommitDetailsDiff { commit_id, result } => {
            details::apply_commit_diff_result(state, commit_id, result)
        }
        GitResult::CommitFiles { commit_id, result } => {
            commit_workflow::handle_commit_files_result(state, commit_id, result)
        }
        GitResult::CommitFileDiff { target, result } => {
            details::apply_commit_file_diff_result(state, target, result)
        }
        GitResult::RefreshFailed { error } => {
            state.work.refresh_pending = false;
            state.work.last_completed_command = Some("refresh".to_string());
            state.status.last_error = Some(format!("Failed to refresh: {error}"));
            push_notice(state, &format!("Failed to refresh: {error}"));
            Vec::new()
        }
        GitResult::StageFiles { paths, result } => {
            operations::handle_stage_files_result(state, paths, result)
        }
        GitResult::UnstageFiles { paths, result } => {
            operations::handle_unstage_files_result(state, paths, result)
        }
        GitResult::StashFiles {
            message,
            paths,
            result,
        } => operations::handle_stash_files_result(state, message, paths, result),
        GitResult::Reset { mode, result } => operations::handle_reset_result(state, mode, result),
        GitResult::Nuke { result } => operations::handle_nuke_result(state, result),
        GitResult::DiscardFiles { paths, result } => {
            operations::handle_discard_files_result(state, paths, result)
        }
        GitResult::CreateCommit { message, result } => {
            operations::handle_create_commit_result(state, message, result)
        }
        GitResult::CreateBranch {
            name,
            start_point,
            result,
        } => operations::handle_create_branch_result(state, name, start_point, result),
        GitResult::CheckoutBranch {
            name,
            auto_stash,
            result,
        } => operations::handle_checkout_branch_result(state, name, auto_stash, result),
        GitResult::DeleteBranch {
            name,
            mode,
            force,
            result,
        } => operations::handle_delete_branch_result(state, name, mode, force, result),
        GitResult::RebaseBranch {
            target,
            interactive,
            auto_stash,
            result,
        } => {
            operations::handle_rebase_branch_result(state, target, interactive, auto_stash, result)
        }
        GitResult::SquashCommits { commit_ids, result } => {
            operations::handle_squash_commits_result(state, commit_ids, result)
        }
        GitResult::FixupCommits { commit_ids, result } => {
            operations::handle_fixup_commits_result(state, commit_ids, result)
        }
        GitResult::RewordCommit {
            commit_id,
            message,
            result,
        } => operations::handle_reword_commit_result(state, commit_id, message, result),
        GitResult::DeleteCommits { commit_ids, result } => {
            operations::handle_delete_commits_result(state, commit_ids, result)
        }
        GitResult::CheckoutCommitDetached {
            commit_id,
            auto_stash,
            result,
        } => {
            operations::handle_checkout_commit_detached_result(state, commit_id, auto_stash, result)
        }
        GitResult::StashPush { message, result } => {
            operations::handle_stash_push_result(state, message, result)
        }
        GitResult::StashPop { stash_id, result } => {
            operations::handle_stash_pop_result(state, stash_id, result)
        }
    }
}
