use crate::app::{App, RefreshKind, SidePanel};
use crate::flux::action::DomainAction;
use crate::flux::effects::EffectRequest;
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
            let result = match app.active_panel {
                SidePanel::Stash => app.stash_open_tree_or_toggle_dir(),
                SidePanel::Commits => app.commit_open_tree_or_toggle_dir(),
                SidePanel::LocalBranches => app.open_selected_branch_commits(100),
                _ => Ok(()),
            };
            match result {
                Ok(()) => {
                    app.restore_search_for_active_scope();
                    app.reload_diff_now();
                    UiInvalidation::all().apply(app);
                }
                Err(err) => app.push_log(format!("revision files failed: {}", err), false),
            }
            Some(vec![])
        }
        EffectRequest::StartCommitEditorGuarded => {
            if app.start_commit_editor_guarded() {
                app.push_log(
                    "commit: edit message/description then press Enter on message",
                    true,
                );
                UiInvalidation::all().apply(app);
            }
            Some(vec![])
        }
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
        EffectRequest::CheckoutSelectedBranch => {
            let Some(name) = app.selected_branch_name() else {
                app.push_log("no branch selected", false);
                return Some(vec![]);
            };
            app.request_refresh(RefreshKind::StatusOnly);
            if let Err(err) = app.flush_pending_refresh() {
                app.push_log(format!("refresh failed: {}", err), false);
                return Some(vec![]);
            }
            if app.has_uncommitted_changes() {
                app.start_branch_switch_confirm(name);
                UiInvalidation::overlay().apply(app);
                return Some(vec![]);
            }
            let result = app.checkout_branch(&name).map_err(|err| err.to_string());
            Some(vec![DomainAction::CheckoutBranchFinished {
                name,
                auto_stash: false,
                result,
            }])
        }
        _ => None,
    }
}
