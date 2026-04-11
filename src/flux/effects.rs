use crate::app::{App, AppEffects, RefreshKind, SidePanel};
use crate::flux::action::{Action, DomainAction};
use crate::flux::branch_backend::{BranchBackend, BranchBackendCommand, BranchBackendEvent};
use crate::flux::files_backend::{FilesBackend, FilesBackendCommand, FilesBackendEvent};
use crate::flux::stores::UiInvalidation;
use std::path::PathBuf;
use std::rc::Rc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub enum EffectRequest {
    ProcessBackgroundLoads,
    /// Used in tests via `test_runtime::run_inline_effect` to force a refresh cycle.
    #[cfg_attr(not(test), allow(dead_code))]
    FlushPendingRefresh {
        log_success: bool,
    },
    FlushPendingDiffReload,
    EnsureCommitsLoadedForActivePanel,
    ReloadDiffNow,
    RevisionOpenTreeOrToggleDir,
    ToggleStageSelection,
    PrepareCommitFromVisualSelection,
    BranchesBackend(BranchBackendCommand),
    FilesBackend(FilesBackendCommand),
    StagePaths(Vec<PathBuf>),
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
    pub app: Rc<Mutex<dyn AppEffects>>,
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
                        app.push_log("refresh".to_string(), true);
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
            let active_panel = app.active_panel();
            let branch_name = if active_panel == SidePanel::LocalBranches {
                app.selected_branch_name()
            } else {
                None
            };
            let result = match active_panel {
                SidePanel::Stash => app.stash_open_tree_or_toggle_dir(),
                SidePanel::Commits => app.commit_open_tree_or_toggle_dir(),
                SidePanel::LocalBranches => Ok(()),
                _ => Ok(()),
            };
            match result {
                Ok(()) => {
                    app.restore_search_for_active_scope();
                    if active_panel != SidePanel::LocalBranches {
                        app.reload_diff_now();
                    }
                    UiInvalidation::all().apply(&mut *app);
                }
                Err(err) => app.push_log(format!("revision files failed: {}", err), false),
            }
            drop(app);
            if active_panel == SidePanel::LocalBranches {
                if let Some(branch) = branch_name {
                    return run_branches_backend_command(
                        BranchBackendCommand::OpenCommitsSubview { branch, limit: 100 },
                        ctx,
                    )
                    .await;
                }
            }
            vec![]
        }
        EffectRequest::ToggleStageSelection => {
            let mut app = ctx.app.lock().await;
            let result = app
                .toggle_stage_visual_selection()
                .map_err(|e| e.to_string());
            vec![Action::Domain(DomainAction::ToggleStageSelectionFinished {
                result,
            })]
        }
        EffectRequest::PrepareCommitFromVisualSelection => {
            let mut app = ctx.app.lock().await;
            let result = app
                .prepare_commit_from_visual_selection()
                .map_err(|e| e.to_string());
            vec![Action::Domain(
                DomainAction::PrepareCommitFromSelectionFinished { result },
            )]
        }
        EffectRequest::BranchesBackend(command) => run_branches_backend_command(command, ctx).await,
        EffectRequest::FilesBackend(command) => run_files_backend_command(command, ctx).await,
        EffectRequest::StagePaths(paths) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stage_paths_request(paths) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![Action::Domain(DomainAction::StagePathsFinished {
                            result: Err(err.to_string()),
                        })];
                    }
                }
            };
            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(()))) => Ok(()),
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };
            vec![Action::Domain(DomainAction::StagePathsFinished { result })]
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

async fn run_files_backend_command(
    command: FilesBackendCommand,
    ctx: &mut EffectCtx,
) -> Vec<Action> {
    match command {
        FilesBackendCommand::RefreshFromStatus { .. } => {
            panic!("pure files refresh commands must not go through the runtime effect adapter")
        }
        FilesBackendCommand::ToggleSelectedDir => {
            let mut app = ctx.app.lock().await;
            let app = app
                .as_any_mut()
                .downcast_mut::<App>()
                .expect("files backend dir toggle requires concrete App");
            let next =
                FilesBackend::toggle_selected_dir(&app.git.status, app.current_files_view_state());
            app.apply_files_backend_view(FilesBackendEvent::ViewStateUpdated(next));
            vec![]
        }
        FilesBackendCommand::CollapseAll => {
            let mut app = ctx.app.lock().await;
            let app = app
                .as_any_mut()
                .downcast_mut::<App>()
                .expect("files backend collapse requires concrete App");
            let next = FilesBackend::collapse_all(&app.git.status, app.current_files_view_state());
            app.apply_files_backend_view(FilesBackendEvent::ViewStateUpdated(next));
            vec![]
        }
        FilesBackendCommand::ExpandAll => {
            let mut app = ctx.app.lock().await;
            let app = app
                .as_any_mut()
                .downcast_mut::<App>()
                .expect("files backend expand requires concrete App");
            let next = FilesBackend::expand_all(&app.git.status, app.current_files_view_state());
            app.apply_files_backend_view(FilesBackendEvent::ViewStateUpdated(next));
            vec![]
        }
        FilesBackendCommand::ReloadDiff => {
            let mut app = ctx.app.lock().await;
            app.reload_diff_now();
            vec![]
        }
        FilesBackendCommand::StagePath(path) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.stage_file_request(path.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![FilesBackendEvent::StageFinished {
                            path,
                            result: Err(err.to_string()),
                        }
                        .into_action()
                        .expect("stage finished event should map to an action")];
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
            vec![FilesBackendEvent::StageFinished { path, result }
                .into_action()
                .expect("stage finished event should map to an action")]
        }
        FilesBackendCommand::UnstagePath(path) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.unstage_file_request(path.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![FilesBackendEvent::UnstageFinished {
                            path,
                            result: Err(err.to_string()),
                        }
                        .into_action()
                        .expect("unstage finished event should map to an action")];
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
            vec![FilesBackendEvent::UnstageFinished { path, result }
                .into_action()
                .expect("unstage finished event should map to an action")]
        }
        FilesBackendCommand::DiscardPaths(paths) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.discard_paths_request(paths.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![FilesBackendEvent::DiscardFinished {
                            paths,
                            result: Err(err.to_string()),
                        }
                        .into_action()
                        .expect("discard finished event should map to an action")];
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
            vec![FilesBackendEvent::DiscardFinished { paths, result }
                .into_action()
                .expect("discard finished event should map to an action")]
        }
    }
}

async fn run_branches_backend_command(
    command: BranchBackendCommand,
    ctx: &mut EffectCtx,
) -> Vec<Action> {
    match command {
        BranchBackendCommand::CreateBranch(name) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.create_branch_request(name.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![BranchBackendEvent::CreateFinished {
                            name,
                            result: Err(err.to_string()),
                        }
                        .into_action()];
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
            vec![BranchBackendEvent::CreateFinished { name, result }.into_action()]
        }
        BranchBackendCommand::CheckoutBranch { name, auto_stash } => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.checkout_branch_request(name.clone(), auto_stash) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![BranchBackendEvent::CheckoutFinished {
                            name,
                            auto_stash,
                            result: Err(err.to_string()),
                        }
                        .into_action()];
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
            vec![BranchBackendEvent::CheckoutFinished {
                name,
                auto_stash,
                result,
            }
            .into_action()]
        }
        BranchBackendCommand::DeleteBranch(name) => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.delete_branch_request(name.clone()) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![BranchBackendEvent::DeleteFinished {
                            name,
                            result: Err(err.to_string()),
                        }
                        .into_action()];
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
            vec![BranchBackendEvent::DeleteFinished { name, result }.into_action()]
        }
        BranchBackendCommand::FetchRemote => {
            {
                let mut app = ctx.app.lock().await;
                let app = app
                    .as_any_mut()
                    .downcast_mut::<App>()
                    .expect("branches backend fetch requires concrete App");
                let next =
                    BranchBackend::set_fetching_remote(app.current_branches_view_state(), true);
                app.apply_branches_backend_view(next);
            }

            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.fetch_remote_request() {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![
                            BranchBackendEvent::FetchFinished(Err(err.to_string())).into_action()
                        ];
                    }
                }
            };

            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(remote))) => Ok(remote),
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };

            vec![BranchBackendEvent::FetchFinished(result).into_action()]
        }
        BranchBackendCommand::LoadBranchGraph { branch_name } => {
            let repo_rx = {
                let app = ctx.app.lock().await;
                match app.git_log_graph_request(branch_name) {
                    Ok(rx) => rx,
                    Err(err) => {
                        return vec![
                            BranchBackendEvent::GraphLoaded(Err(err.to_string())).into_action()
                        ];
                    }
                }
            };

            let result = match tokio::task::spawn_blocking(move || repo_rx.recv()).await {
                Ok(Ok(Ok(lines))) => Ok(lines),
                Ok(Ok(Err(err))) => Err(err.to_string()),
                Ok(Err(err)) => Err(err.to_string()),
                Err(err) => Err(err.to_string()),
            };

            vec![BranchBackendEvent::GraphLoaded(result).into_action()]
        }
        BranchBackendCommand::OpenCommitsSubview { branch, limit } => {
            let mut app = ctx.app.lock().await;
            let app = app
                .as_any_mut()
                .downcast_mut::<App>()
                .expect("branches backend subview requires concrete App");
            let next = BranchBackend::open_commits_subview(
                app.current_branches_view_state(),
                branch.clone(),
            );
            app.apply_branches_backend_view(next);
            let _ = app.open_selected_branch_commits(limit);
            vec![]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::flux::files_backend::FilesBackendCommand;
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
    async fn files_backend_stage_path_returns_stage_file_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::FilesBackend(FilesBackendCommand::StagePath(
                "foo.txt".into(),
            )))
            .await,
        );
        assert!(matches!(action, DomainAction::StageFileFinished { .. }));
    }

    #[tokio::test]
    async fn files_backend_unstage_path_returns_unstage_file_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::FilesBackend(
                FilesBackendCommand::UnstagePath("foo.txt".into()),
            ))
            .await,
        );
        assert!(matches!(action, DomainAction::UnstageFileFinished { .. }));
    }

    #[tokio::test]
    async fn files_backend_discard_paths_returns_discard_paths_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::FilesBackend(
                FilesBackendCommand::DiscardPaths(vec!["foo.txt".into()]),
            ))
            .await,
        );
        assert!(matches!(action, DomainAction::DiscardPathsFinished { .. }));
    }

    #[tokio::test]
    async fn files_backend_reload_diff_returns_no_actions() {
        assert_no_actions(
            run_effect(EffectRequest::FilesBackend(FilesBackendCommand::ReloadDiff)).await,
        );
    }

    #[tokio::test]
    async fn create_branch_effect_returns_create_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::BranchesBackend(
                BranchBackendCommand::CreateBranch("new-branch".to_string()),
            ))
            .await,
        );
        assert!(matches!(action, DomainAction::CreateBranchFinished { .. }));
    }

    #[tokio::test]
    async fn checkout_branch_effect_returns_checkout_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::BranchesBackend(
                BranchBackendCommand::CheckoutBranch {
                    name: "main".to_string(),
                    auto_stash: false,
                },
            ))
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
            run_effect(EffectRequest::BranchesBackend(
                BranchBackendCommand::DeleteBranch("old-branch".to_string()),
            ))
            .await,
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
    async fn toggle_stage_selection_effect_returns_toggle_stage_selection_finished_action() {
        let action =
            assert_single_domain_action(run_effect(EffectRequest::ToggleStageSelection).await);
        assert!(matches!(
            action,
            DomainAction::ToggleStageSelectionFinished { .. }
        ));
    }

    #[tokio::test]
    async fn revision_open_from_branches_opens_commits_subview_in_branches_panel() {
        let mut ctx = make_ctx();
        {
            let mut app_guard = ctx.app.lock().await;
            let app: &mut App = (*app_guard).as_any_mut().downcast_mut().unwrap();
            app.ui.active_panel = crate::app::SidePanel::LocalBranches;
            app.ui.branches.items = vec![crate::git::BranchInfo {
                name: "main".to_string(),
                is_current: true,
            }];
            app.ui.branches.panel.list_state.select(Some(0));
        }

        assert_no_actions(run(EffectRequest::RevisionOpenTreeOrToggleDir, &mut ctx).await);
        assert_no_actions(run(EffectRequest::ProcessBackgroundLoads, &mut ctx).await);
        let app_guard = ctx.app.lock().await;
        let app: &App = (*app_guard).as_any().downcast_ref().unwrap();
        assert_eq!(app.ui.active_panel, crate::app::SidePanel::LocalBranches);
        assert!(app.ui.branches.commits_subview_active);
        assert!(!app.ui.branches.commits_subview.items.is_empty());
        assert_eq!(
            app.ui.branches.commits_subview.panel.list_state.selected(),
            Some(0)
        );
    }

    #[tokio::test]
    async fn prepare_commit_from_visual_selection_effect_returns_prepare_commit_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::PrepareCommitFromVisualSelection).await,
        );
        assert!(matches!(
            action,
            DomainAction::PrepareCommitFromSelectionFinished { .. }
        ));
    }

    #[tokio::test]
    async fn checkout_branch_with_auto_stash_effect_returns_checkout_branch_finished_action() {
        let action = assert_single_domain_action(
            run_effect(EffectRequest::BranchesBackend(
                BranchBackendCommand::CheckoutBranch {
                    name: "feature".to_string(),
                    auto_stash: true,
                },
            ))
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
