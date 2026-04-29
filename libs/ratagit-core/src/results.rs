use crate::{
    AppContext, Command, GitResult, PanelFocus, RefreshTarget, commit_workflow, details,
    operations, push_notice, snapshot,
};

pub(crate) fn update_git_result(state: &mut AppContext, result: GitResult) -> Vec<Command> {
    match result {
        GitResult::Refreshed(repo_snapshot) => {
            state.work.clear_refresh();
            snapshot::apply_snapshot(state, repo_snapshot);
            state.repo.status.refresh_count = state.repo.status.refresh_count.saturating_add(1);
            state.repo.status.last_error = None;
            details::refresh_for_focus(state)
        }
        GitResult::SplitRefreshed {
            files,
            branches,
            commits,
            stashes,
        } => {
            state.work.clear_refresh();
            snapshot::apply_split_snapshot(state, files, branches, commits, stashes);
            state.repo.status.refresh_count = state.repo.status.refresh_count.saturating_add(1);
            state.repo.status.last_error = None;
            details::refresh_for_focus(state)
        }
        GitResult::FilesRefreshed(files_snapshot) => {
            snapshot::apply_files_snapshot(state, files_snapshot);
            finish_refresh_target(state, RefreshTarget::Files);
            state.repo.status.last_error = None;
            refresh_details_if_focus(state, PanelFocus::Files)
        }
        GitResult::BranchesRefreshed(branches) => {
            snapshot::apply_branches_snapshot(state, branches);
            finish_refresh_target(state, RefreshTarget::Branches);
            state.repo.status.last_error = None;
            refresh_details_if_focus(state, PanelFocus::Branches)
        }
        GitResult::CommitsRefreshed(commits) => {
            snapshot::apply_commits_snapshot(state, commits);
            finish_refresh_target(state, RefreshTarget::Commits);
            state.repo.status.last_error = None;
            refresh_details_if_focus(state, PanelFocus::Commits)
        }
        GitResult::StashesRefreshed(stashes) => {
            snapshot::apply_stashes_snapshot(state, stashes);
            finish_refresh_target(state, RefreshTarget::Stash);
            state.repo.status.last_error = None;
            refresh_details_if_focus(state, PanelFocus::Stash)
        }
        GitResult::CommitsPage {
            offset,
            limit,
            epoch,
            result,
        } => commit_workflow::handle_commits_page_result(state, offset, limit, epoch, result),
        GitResult::FilesDetailsDiff {
            request_id,
            targets,
            truncated_from,
            result,
        } => details::apply_files_diff_result(state, request_id, targets, truncated_from, result),
        GitResult::BranchDetailsLog {
            request_id,
            branch,
            result,
        } => details::apply_branch_log_result(state, request_id, branch, result),
        GitResult::CommitDetailsDiff {
            request_id,
            commit_id,
            result,
        } => details::apply_commit_diff_result(state, request_id, commit_id, result),
        GitResult::BranchCommits { branch, result } => {
            crate::branches::handle_branch_commits_result(state, branch, result)
        }
        GitResult::BranchCommitFiles {
            branch,
            commit_id,
            result,
        } => crate::branches::handle_branch_commit_files_result(state, branch, commit_id, result),
        GitResult::CommitFiles { commit_id, result } => {
            commit_workflow::handle_commit_files_result(state, commit_id, result)
        }
        GitResult::CommitFileDiff {
            request_id,
            target,
            result,
        } => details::apply_commit_file_diff_result(state, request_id, target, result),
        GitResult::RefreshFailed { target, error } => {
            if let Some(target) = target {
                finish_refresh_target(state, target);
            } else {
                state.work.clear_refresh();
            }
            state.repo.status.last_error = Some(format!("Failed to refresh: {error}"));
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
        GitResult::AmendStagedChanges { commit_id, result } => {
            operations::handle_amend_staged_changes_result(state, commit_id, result)
        }
        GitResult::Pull { result } => operations::handle_pull_result(state, result),
        GitResult::Push { force, result } => operations::handle_push_result(state, force, result),
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

fn finish_refresh_target(state: &mut AppContext, target: RefreshTarget) {
    state
        .work
        .finish_refresh_target(target, refresh_target_command_label(target));
    if !state.work.refresh.refresh_pending {
        state.repo.status.refresh_count = state.repo.status.refresh_count.saturating_add(1);
    }
}

fn refresh_details_if_focus(state: &mut AppContext, panel: PanelFocus) -> Vec<Command> {
    if state.ui.focus == panel {
        details::refresh_for_focus(state)
    } else {
        Vec::new()
    }
}

fn refresh_target_command_label(target: RefreshTarget) -> &'static str {
    match target {
        RefreshTarget::Files => "refresh_files",
        RefreshTarget::Branches => "refresh_branches",
        RefreshTarget::Commits => "refresh_commits",
        RefreshTarget::Stash => "refresh_stash",
    }
}
