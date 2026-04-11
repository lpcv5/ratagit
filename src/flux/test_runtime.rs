use crate::app::{App, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::branch_backend::BranchBackend;
use crate::flux::commits_backend::CommitsBackendCommand;
use crate::flux::effects::EffectRequest;
use crate::flux::git_backend::stash::StashBackendCommand;
use crate::flux::git_backend::GitBackendCommand;
use crate::flux::stores::UiInvalidation;

/// Executes the subset of runtime effects that tests need in-process.
/// Returns `None` for effects that should stay as real async runtime work.
pub fn run_inline_effect(app: &mut App, request: EffectRequest) -> Option<Vec<DomainAction>> {
    match request {
        EffectRequest::ProcessBackgroundLoads => {
            app.process_background_refresh_tick();
            Some(vec![])
        }
        EffectRequest::FlushPendingRefresh { log_success } => {
            match app.flush_pending_refresh() {
                Ok(_) => {
                    if log_success {
                        app.push_log("refresh", true);
                    }
                }
                Err(err) => app.push_log(format!("refresh failed: {}", err), false),
            }
            Some(vec![])
        }
        EffectRequest::FlushPendingDiffReload => {
            app.flush_pending_diff_reload();
            Some(vec![])
        }
        EffectRequest::EnsureCommitsLoadedForActivePanel => {
            app.ensure_commits_loaded_for_active_panel();
            Some(vec![])
        }
        EffectRequest::ReloadDiffNow => {
            app.reload_diff_now();
            Some(vec![])
        }
        EffectRequest::RevisionOpenTreeOrToggleDir => {
            let active_panel = app.ui.active_panel;
            let branch_name = if active_panel == SidePanel::LocalBranches {
                app.selected_branch_name()
            } else {
                None
            };
            let result = match active_panel {
                SidePanel::Stash => {
                    app.apply_stash_backend_command(StashBackendCommand::OpenTreeOrToggleDir)
                }
                SidePanel::Commits => {
                    app.apply_commits_backend_command(CommitsBackendCommand::OpenTreeOrToggleDir)
                }
                SidePanel::LocalBranches => Ok(()),
                _ => Ok(()),
            };
            match result {
                Ok(()) => {
                    app.restore_search_for_active_scope();
                    if active_panel != SidePanel::LocalBranches {
                        app.reload_diff_now();
                    }
                    UiInvalidation::all().apply(app);
                }
                Err(err) => app.push_log(format!("revision files failed: {}", err), false),
            }
            if active_panel == SidePanel::LocalBranches {
                if let Some(branch) = branch_name {
                    let next = BranchBackend::open_commits_subview(
                        app.current_branches_view_state(),
                        branch.clone(),
                    );
                    app.apply_branches_backend_view(next);
                    match app.start_branch_commits_background_load(branch.clone(), 100) {
                        Ok(()) => {
                            app.push_log(format!("branch commits: {} (Esc to back)", branch), true)
                        }
                        Err(err) => {
                            let failed = BranchBackend::fail_commits_subview_load(
                                app.current_branches_view_state(),
                                &branch,
                            );
                            app.apply_branches_backend_view(failed);
                            app.push_log(format!("branch commits load failed: {}", err), false);
                        }
                    }
                }
            }
            Some(vec![])
        }
        EffectRequest::CommitsBackend(command) => {
            let opens_tree = matches!(command, CommitsBackendCommand::OpenTreeOrToggleDir);
            let closes_tree = matches!(command, CommitsBackendCommand::CloseTree);
            match app.apply_commits_backend_command(command) {
                Ok(()) => {
                    if opens_tree {
                        app.restore_search_for_active_scope();
                        app.reload_diff_now();
                        UiInvalidation::all().apply(app);
                    }
                    if closes_tree {
                        app.restore_search_for_active_scope();
                    }
                }
                Err(err) => app.push_log(format!("commits backend command failed: {}", err), false),
            }
            Some(vec![])
        }
        EffectRequest::GitBackend(command) => match command {
            GitBackendCommand::Stash(command) => match command {
                StashBackendCommand::OpenTreeOrToggleDir
                | StashBackendCommand::OpenTree { .. }
                | StashBackendCommand::CloseTree
                | StashBackendCommand::ApplyLoaded { .. } => {
                    let opens_tree = matches!(
                        command,
                        StashBackendCommand::OpenTreeOrToggleDir
                            | StashBackendCommand::OpenTree { .. }
                    );
                    let closes_tree = matches!(command, StashBackendCommand::CloseTree);
                    match app.apply_stash_backend_command(command) {
                        Ok(()) => {
                            if opens_tree || closes_tree {
                                app.restore_search_for_active_scope();
                            }
                            if opens_tree {
                                app.reload_diff_now();
                                UiInvalidation::all().apply(app);
                            }
                        }
                        Err(err) => app.push_log(format!("revision files failed: {}", err), false),
                    }
                    Some(vec![])
                }
                _ => None,
            },
        },
        EffectRequest::StagePaths(paths) => match app.stage_paths_request(paths) {
            Ok(rx) => match rx.recv() {
                Ok(Ok(())) => Some(vec![DomainAction::StagePathsFinished { result: Ok(()) }]),
                Ok(Err(e)) => Some(vec![DomainAction::StagePathsFinished {
                    result: Err(e.to_string()),
                }]),
                Err(e) => Some(vec![DomainAction::StagePathsFinished {
                    result: Err(e.to_string()),
                }]),
            },
            Err(e) => Some(vec![DomainAction::StagePathsFinished {
                result: Err(e.to_string()),
            }]),
        },
        EffectRequest::ToggleStageSelection => {
            match app.toggle_stage_visual_selection() {
                Ok((staged, unstaged)) => {
                    app.push_log(
                        format!(
                            "selection toggled: staged {}, unstaged {}",
                            staged, unstaged
                        ),
                        true,
                    );
                    UiInvalidation::all().apply(app);
                }
                Err(err) => app.push_log(format!("selection toggle failed: {}", err), false),
            }
            Some(vec![])
        }
        EffectRequest::PrepareCommitFromVisualSelection => {
            match app.prepare_commit_from_visual_selection() {
                Ok(count) => {
                    if count == 0 {
                        app.push_log("commit blocked: no selected items", false);
                        return Some(vec![]);
                    }
                    if app.start_commit_editor_guarded() {
                        app.push_log(
                            format!(
                                "commit: {} selected target(s) staged; edit message/description",
                                count
                            ),
                            true,
                        );
                        UiInvalidation::all().apply(app);
                    }
                }
                Err(err) => app.push_log(format!("prepare commit failed: {}", err), false),
            }
            Some(vec![])
        }
        _ => None,
    }
}
