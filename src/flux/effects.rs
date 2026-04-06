use crate::app::{App, RefreshKind, SidePanel};
use crate::flux::action::{Action, DomainAction};
use crate::flux::stores::UiInvalidation;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::Mutex;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum EffectRequest {
    ProcessBackgroundLoads,
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
        EffectRequest::ProcessBackgroundLoads => {
            let mut app = ctx.app.lock().await;
            app.process_background_refresh_tick();
            vec![]
        }
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
            let result = match app.ui.active_panel {
                SidePanel::Stash => app.stash_open_tree_or_toggle_dir(),
                SidePanel::Commits => app.commit_open_tree_or_toggle_dir(),
                SidePanel::LocalBranches => app.open_selected_branch_commits(100),
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
                .git
                .status
                .unstaged
                .iter()
                .map(|e| e.path.clone())
                .chain(app.git.status.untracked.iter().map(|e| e.path.clone()))
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
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stage_file_request(path.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::StageFileFinished {
                            path,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusOnly);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::StageFileFinished {
                path,
                result,
            })]
        }
        EffectRequest::UnstageFile(path) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.unstage_file_request(path.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::UnstageFileFinished {
                            path,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusOnly);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::UnstageFileFinished {
                path,
                result,
            })]
        }
        EffectRequest::DiscardPaths(paths) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.discard_paths_request(paths.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::DiscardPathsFinished {
                            paths,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusOnly);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::DiscardPathsFinished {
                paths,
                result,
            })]
        }
        EffectRequest::CreateBranch(name) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.create_branch_request(name.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::CreateBranchFinished {
                            name,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusAndRefs);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::CreateBranchFinished {
                name,
                result,
            })]
        }
        EffectRequest::CheckoutBranch { name, auto_stash } => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.checkout_branch_request(name.clone(), auto_stash) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::CheckoutBranchFinished {
                            name,
                            auto_stash,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::Full);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::CheckoutBranchFinished {
                name,
                auto_stash,
                result,
            })]
        }
        EffectRequest::DeleteBranch(name) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.delete_branch_request(name.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::DeleteBranchFinished {
                            name,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusAndRefs);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::DeleteBranchFinished {
                name,
                result,
            })]
        }
        EffectRequest::Commit(message) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.commit_request(message.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::CommitFinished {
                            message,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(oid))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::Full);
                    Ok(oid)
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::CommitFinished {
                message,
                result,
            })]
        }
        EffectRequest::StashPush { message, paths } => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stash_push_request(paths.clone(), message.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::StashPushFinished {
                            message,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(index))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusAndRefs);
                    Ok(index)
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::StashPushFinished {
                message,
                result,
            })]
        }
        EffectRequest::StashApply(index) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stash_apply_request(index) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::StashApplyFinished {
                            index,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusAndRefs);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::StashApplyFinished {
                index,
                result,
            })]
        }
        EffectRequest::StashPop(index) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stash_pop_request(index) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::StashPopFinished {
                            index,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusAndRefs);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::StashPopFinished {
                index,
                result,
            })]
        }
        EffectRequest::StashDrop(index) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stash_drop_request(index) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::StashDropFinished {
                            index,
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => {
                    let mut app = ctx.app.lock().await;
                    app.request_refresh(RefreshKind::StatusAndRefs);
                    Ok(())
                }
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::StashDropFinished {
                index,
                result,
            })]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::stores::test_support::MockRepo;
    use pretty_assertions::assert_eq;
    use std::rc::Rc;
    use tokio::sync::Mutex;

    fn make_ctx() -> EffectCtx {
        let app = App::from_repo(Box::new(MockRepo)).expect("app");
        EffectCtx {
            app: Rc::new(Mutex::new(app)),
        }
    }

    async fn run_effect(request: EffectRequest) -> Vec<Action> {
        let mut ctx = make_ctx();
        run(request, &mut ctx).await
    }

    fn assert_single_domain_action(actions: Vec<Action>) -> DomainAction {
        assert_eq!(actions.len(), 1);
        match actions.into_iter().next().expect("single action") {
            Action::Domain(action) => action,
            other => panic!("expected domain action, got: {other:?}"),
        }
    }

    fn assert_no_actions(actions: Vec<Action>) {
        assert!(actions.is_empty());
    }

    #[tokio::test]
    async fn stage_file_effect_returns_stage_file_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::StageFile("foo.txt".into())).await,
        );
        assert!(matches!(action, DomainAction::StageFileFinished { .. }));
    }

    #[tokio::test]
    async fn unstage_file_effect_returns_unstage_file_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::UnstageFile("foo.txt".into())).await,
        );
        assert!(matches!(action, DomainAction::UnstageFileFinished { .. }));
    }

    #[tokio::test]
    async fn discard_paths_effect_returns_discard_paths_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::DiscardPaths(vec!["foo.txt".into()])).await,
        );
        assert!(matches!(action, DomainAction::DiscardPathsFinished { .. }));
    }

    #[tokio::test]
    async fn create_branch_effect_returns_create_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::CreateBranch("new-branch".to_string())).await,
        );
        assert!(matches!(action, DomainAction::CreateBranchFinished { .. }));
    }

    #[tokio::test]
    async fn checkout_branch_effect_returns_checkout_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::CheckoutBranch {
                name: "main".to_string(),
                auto_stash: false,
            })
            .await,
        );
        assert!(matches!(
            action,
            DomainAction::CheckoutBranchFinished { .. }
        ));
    }

    #[tokio::test]
    async fn delete_branch_effect_returns_delete_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::DeleteBranch("old-branch".to_string())).await,
        );
        assert!(matches!(action, DomainAction::DeleteBranchFinished { .. }));
    }

    #[tokio::test]
    async fn commit_effect_returns_commit_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::Commit("test commit".to_string())).await,
        );
        assert!(matches!(action, DomainAction::CommitFinished { .. }));
    }

    #[tokio::test]
    async fn stash_push_effect_returns_stash_push_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::StashPush {
                message: "wip".to_string(),
                paths: vec!["foo.txt".into()],
            })
            .await,
        );
        assert!(matches!(action, DomainAction::StashPushFinished { .. }));
    }

    #[tokio::test]
    async fn stash_apply_effect_returns_stash_apply_finished_action() {
        let action = assert_single_domain_action(run_effect(EffectRequest::StashApply(0)).await);
        assert!(matches!(action, DomainAction::StashApplyFinished { .. }));
    }

    #[tokio::test]
    async fn stash_pop_effect_returns_stash_pop_finished_action() {
        let action = assert_single_domain_action(run_effect(EffectRequest::StashPop(0)).await);
        assert!(matches!(action, DomainAction::StashPopFinished { .. }));
    }

    #[tokio::test]
    async fn stash_drop_effect_returns_stash_drop_finished_action() {
        let action = assert_single_domain_action(run_effect(EffectRequest::StashDrop(0)).await);
        assert!(matches!(action, DomainAction::StashDropFinished { .. }));
    }

    #[tokio::test]
    async fn flush_pending_refresh_without_log_success_returns_no_actions() {
        assert_no_actions(
            run_effect(EffectRequest::FlushPendingRefresh { log_success: false }).await,
        );
    }

    #[tokio::test]
    async fn reload_diff_now_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::ReloadDiffNow).await);
    }

    #[tokio::test]
    async fn flush_pending_diff_reload_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::FlushPendingDiffReload).await);
    }

    #[tokio::test]
    async fn ensure_commits_loaded_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::EnsureCommitsLoadedForActivePanel).await);
    }

    #[tokio::test]
    async fn toggle_stage_selection_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::ToggleStageSelection).await);
    }

    #[tokio::test]
    async fn start_commit_editor_guarded_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::StartCommitEditorGuarded).await);
    }

    #[tokio::test]
    async fn checkout_selected_branch_effect_returns_checkout_branch_finished_action() {
        let mut ctx = make_ctx();
        {
            let mut app = ctx.app.lock().await;
            app.ui.active_panel = crate::app::SidePanel::LocalBranches;
            app.ui.branches.items = vec![crate::git::BranchInfo {
                name: "main".to_string(),
                is_current: true,
            }];
            app.ui.branches.panel.list_state.select(Some(0));
        }

        let action =
            assert_single_domain_action(run(EffectRequest::CheckoutSelectedBranch, &mut ctx).await);
        assert!(matches!(
            action,
            DomainAction::CheckoutBranchFinished { .. }
        ));
    }

    #[tokio::test]
    async fn revision_open_from_branches_opens_commits_subview_in_branches_panel() {
        let mut ctx = make_ctx();
        {
            let mut app = ctx.app.lock().await;
            app.ui.active_panel = crate::app::SidePanel::LocalBranches;
            app.ui.branches.items = vec![crate::git::BranchInfo {
                name: "main".to_string(),
                is_current: true,
            }];
            app.ui.branches.panel.list_state.select(Some(0));
        }

        assert_no_actions(run(EffectRequest::RevisionOpenTreeOrToggleDir, &mut ctx).await);
        assert_no_actions(run(EffectRequest::ProcessBackgroundLoads, &mut ctx).await);
        let app = ctx.app.lock().await;
        assert_eq!(app.ui.active_panel, crate::app::SidePanel::LocalBranches);
        assert!(app.ui.branches.commits_subview_active);
        assert!(!app.ui.branches.commits_subview.items.is_empty());
        assert_eq!(
            app.ui.branches.commits_subview.panel.list_state.selected(),
            Some(0)
        );
    }

    #[tokio::test]
    async fn stage_all_and_start_commit_editor_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::StageAllAndStartCommitEditor).await);
    }

    #[tokio::test]
    async fn prepare_commit_from_visual_selection_effect_returns_no_actions() {
        assert_no_actions(run_effect(EffectRequest::PrepareCommitFromVisualSelection).await);
    }

    #[tokio::test]
    async fn checkout_branch_with_auto_stash_effect_returns_checkout_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::CheckoutBranch {
                name: "feature".to_string(),
                auto_stash: true,
            })
            .await,
        );
        assert!(matches!(
            action,
            DomainAction::CheckoutBranchFinished { .. }
        ));
    }

    #[tokio::test]
    async fn flush_pending_refresh_with_log_success_returns_no_actions() {
        assert_no_actions(
            run_effect(EffectRequest::FlushPendingRefresh { log_success: true }).await,
        );
    }
}
