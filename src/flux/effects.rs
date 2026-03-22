use crate::app::{App, RefreshKind, SidePanel};
use crate::flux::action::{Action, DomainAction};
use crate::flux::stores::UiInvalidation;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum EffectRequest {
    FlushPendingRefresh {
        log_success: bool,
    },
    FlushPendingDiffReload,
    EnsureCommitsLoadedForActivePanel,
    ReloadDiffNow,
    RevisionOpenTreeOrToggleDir,
    StartCommitEditorGuarded,
    StageAllAndStartCommitEditor,
    ToggleStageSelection,
    PrepareCommitFromVisualSelection,
    CheckoutSelectedBranch,
    FetchRemote,
    StageFile(PathBuf),
    UnstageFile(PathBuf),
    DiscardPaths(Vec<PathBuf>),
    CreateBranch(String),
    CheckoutBranch {
        name: String,
        auto_stash: bool,
    },
    DeleteBranch(String),
    Commit(String),
    StashPush {
        message: String,
        paths: Vec<PathBuf>,
    },
    StashApply(usize),
    StashPop(usize),
    StashDrop(usize),
}

pub struct EffectCtx {
    pub app: Rc<Mutex<App>>,
}

pub async fn run(request: EffectRequest, ctx: &mut EffectCtx) -> Vec<Action> {
    match request {
        EffectRequest::FlushPendingRefresh { log_success } => {
            let mut app = ctx.app.lock().await;
            match app.flush_pending_refresh() {
                Ok(_) => {
                    if log_success {
                        app.push_log("refresh", true);
                    }
                }
                Err(err) => app.push_log(format!("refresh failed: {}", err), false),
            }
            vec![]
        }
        EffectRequest::FlushPendingDiffReload => {
            let mut app = ctx.app.lock().await;
            app.flush_pending_diff_reload();
            vec![]
        }
        EffectRequest::EnsureCommitsLoadedForActivePanel => {
            let mut app = ctx.app.lock().await;
            app.ensure_commits_loaded_for_active_panel();
            vec![]
        }
        EffectRequest::ReloadDiffNow => {
            let mut app = ctx.app.lock().await;
            app.reload_diff_now();
            vec![]
        }
        EffectRequest::RevisionOpenTreeOrToggleDir => {
            let mut app = ctx.app.lock().await;
            let result = match app.active_panel {
                SidePanel::Stash => app.stash_open_tree_or_toggle_dir(),
                SidePanel::Commits => app.commit_open_tree_or_toggle_dir(),
                _ => Ok(()),
            };
            match result {
                Ok(()) => {
                    app.restore_search_for_active_scope();
                    app.reload_diff_now();
                    UiInvalidation::all().apply(&mut app);
                }
                Err(err) => app.push_log(format!("revision files failed: {}", err), false),
            }
            vec![]
        }
        EffectRequest::StartCommitEditorGuarded => {
            let mut app = ctx.app.lock().await;
            if app.start_commit_editor_guarded() {
                app.push_log(
                    "commit: edit message/description then press Enter on message",
                    true,
                );
                UiInvalidation::all().apply(&mut app);
            }
            vec![]
        }
        EffectRequest::StageAllAndStartCommitEditor => {
            let mut app = ctx.app.lock().await;
            app.cancel_input();
            let paths: Vec<PathBuf> = app
                .status
                .unstaged
                .iter()
                .map(|e| e.path.clone())
                .chain(app.status.untracked.iter().map(|e| e.path.clone()))
                .collect();
            if paths.is_empty() {
                app.push_log("nothing to stage", false);
                return vec![];
            }
            if let Err(err) = app.stage_paths(&paths) {
                app.push_log(format!("stage all failed: {}", err), false);
                return vec![];
            }
            app.request_refresh(crate::app::RefreshKind::StatusOnly);
            if let Err(err) = app.flush_pending_refresh() {
                app.push_log(format!("refresh failed: {}", err), false);
                return vec![];
            }
            app.start_commit_editor();
            app.push_log(
                "commit: all files staged; edit message/description then press Enter",
                true,
            );
            UiInvalidation::all().apply(&mut app);
            vec![]
        }
        EffectRequest::ToggleStageSelection => {
            let mut app = ctx.app.lock().await;
            match app.toggle_stage_visual_selection() {
                Ok((staged, unstaged)) => {
                    app.push_log(
                        format!(
                            "selection toggled: staged {}, unstaged {}",
                            staged, unstaged
                        ),
                        true,
                    );
                    let _ = app.flush_pending_refresh();
                    UiInvalidation::all().apply(&mut app);
                }
                Err(err) => app.push_log(format!("selection toggle failed: {}", err), false),
            }
            vec![]
        }
        EffectRequest::PrepareCommitFromVisualSelection => {
            let mut app = ctx.app.lock().await;
            match app.prepare_commit_from_visual_selection() {
                Ok(count) => {
                    if count == 0 {
                        app.push_log("commit blocked: no selected items", false);
                        return vec![];
                    }
                    let _ = app.flush_pending_refresh();
                    if app.start_commit_editor_guarded() {
                        app.push_log(
                            format!(
                                "commit: {} selected target(s) staged; edit message/description",
                                count
                            ),
                            true,
                        );
                        UiInvalidation::all().apply(&mut app);
                    }
                }
                Err(err) => app.push_log(format!("prepare commit failed: {}", err), false),
            }
            vec![]
        }
        EffectRequest::CheckoutSelectedBranch => {
            let mut app = ctx.app.lock().await;
            let Some(name) = app.selected_branch_name() else {
                app.push_log("no branch selected", false);
                return vec![];
            };

            app.request_refresh(RefreshKind::StatusOnly);
            if let Err(err) = app.flush_pending_refresh() {
                app.push_log(format!("refresh failed: {}", err), false);
                return vec![];
            }

            if app.has_uncommitted_changes() {
                app.start_branch_switch_confirm(name);
                UiInvalidation::overlay().apply(&mut app);
                return vec![];
            }

            let result = app.checkout_branch(&name).map_err(|err| err.to_string());
            vec![Action::Domain(DomainAction::CheckoutBranchFinished {
                name,
                auto_stash: false,
                result,
            })]
        }
        EffectRequest::FetchRemote => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.fetch_remote_request() {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::FetchRemoteFinished(Err(
                            err.to_string()
                        )))];
                    }
                }
            };

            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(remote))) => Ok(remote),
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };

            vec![Action::Domain(DomainAction::FetchRemoteFinished(result))]
        }
        EffectRequest::StageFile(path) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.stage_file(path.clone()).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::StageFileFinished {
                path,
                result,
            })]
        }
        EffectRequest::UnstageFile(path) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.unstage_file(path.clone())
                    .map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::UnstageFileFinished {
                path,
                result,
            })]
        }
        EffectRequest::DiscardPaths(paths) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.discard_paths(&paths).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::DiscardPathsFinished {
                paths,
                result,
            })]
        }
        EffectRequest::CreateBranch(name) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.create_branch(&name).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::CreateBranchFinished {
                name,
                result,
            })]
        }
        EffectRequest::CheckoutBranch { name, auto_stash } => {
            let result = {
                let mut app = ctx.app.lock().await;
                if auto_stash {
                    app.checkout_branch_with_auto_stash(&name)
                } else {
                    app.checkout_branch(&name)
                }
                .map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::CheckoutBranchFinished {
                name,
                auto_stash,
                result,
            })]
        }
        EffectRequest::DeleteBranch(name) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.delete_branch(&name).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::DeleteBranchFinished {
                name,
                result,
            })]
        }
        EffectRequest::Commit(message) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.commit(&message).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::CommitFinished {
                message,
                result,
            })]
        }
        EffectRequest::StashPush { message, paths } => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.stash_push(&paths, &message)
                    .map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::StashPushFinished {
                message,
                result,
            })]
        }
        EffectRequest::StashApply(index) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.stash_apply(index).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::StashApplyFinished {
                index,
                result,
            })]
        }
        EffectRequest::StashPop(index) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.stash_pop(index).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::StashPopFinished {
                index,
                result,
            })]
        }
        EffectRequest::StashDrop(index) => {
            let result = {
                let mut app = ctx.app.lock().await;
                app.stash_drop(index).map_err(|err| err.to_string())
            };
            vec![Action::Domain(DomainAction::StashDropFinished {
                index,
                result,
            })]
        }
    }
}
