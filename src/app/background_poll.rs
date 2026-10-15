use super::app::RefreshKind;
use super::diff::DiffRefreshSource;
use super::revisions::RevisionAction;
use super::{AppState, BackgroundCompleted, BackgroundPayload, SidePanel};
use crate::background::TaskKey;
use crate::flux::action::RuntimeEvent;
use crate::flux::command_output::CommandOutput;
use tracing::debug;

impl AppState {
    pub fn process_background_refresh_tick(&mut self, output: &mut CommandOutput) {
        if let Some(kind) = self.refresh.pending_refresh.take() {
            let mut deferred_kind = None;
            if self.has_background_task(&TaskKey::Status) {
                deferred_kind = Some(kind);
            } else if !self.refresh.pending_refresh_fast_done {
                if self.start_status_load(true, output) {
                    self.refresh.pending_refresh_fast_done = true;
                    deferred_kind = Some(kind);
                } else {
                    deferred_kind = Some(RefreshKind::StatusOnly);
                }
            } else {
                if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full)
                    && (!self.start_branches_load(output) || !self.start_stashes_load(output))
                {
                    deferred_kind = Some(match deferred_kind {
                        None => RefreshKind::StatusAndRefs,
                        Some(existing) => {
                            Self::max_refresh_kind(existing, RefreshKind::StatusAndRefs)
                        }
                    });
                }
                if matches!(kind, RefreshKind::Full) {
                    self.reduce_revisions(RevisionAction::MarkCommitsDirty);
                }
                if deferred_kind.is_none() {
                    self.refresh.pending_refresh_fast_done = false;
                }
            }
            if let Some(kind) = deferred_kind {
                self.refresh.pending_refresh = Some(match self.refresh.pending_refresh {
                    None => kind,
                    Some(existing) => Self::max_refresh_kind(existing, kind),
                });
            }
        }

        if self.ui.active_panel == SidePanel::Commits && self.revisions.view().commits.dirty {
            let _ = self.start_commits_load(self.revisions.commits_requested_limit(), output);
        }
        self.maybe_schedule_commits_extend();

        self.start_pending_diff_load(output);
    }

    /// Accept results only for tasks still tracked by state. Cancellation and
    /// selection changes remove old tasks before a replacement is started.
    pub(crate) fn apply_background_completed(
        &mut self,
        completed: &BackgroundCompleted,
        output: &mut CommandOutput,
    ) {
        if !self.tasks.accept(completed) {
            return;
        }
        // Task generations and repository freshness are independent. Check both
        // before consuming a payload or reporting a now-obsolete failure.
        if !self.diff_completion_is_current(completed) {
            self.diff.reject_load();
            self.schedule_diff_reload(output);
            return;
        }
        let mut schedule_diff = false;
        let mut refresh_search = false;
        match &completed.result {
            Ok(payload) => {
                let payload = payload.as_ref();
                refresh_search |= !matches!(payload, BackgroundPayload::Diff { .. });
                match payload {
                    BackgroundPayload::Status {
                        status,
                        fast,
                        operation,
                        operation_epoch,
                    } => {
                        debug!(
                            event = "status_load_finished",
                            mode = if *fast { "fast" } else { "full" },
                            staged = status.staged.len(),
                            unstaged = status.unstaged.len(),
                            untracked = status.untracked.len(),
                            "status load finished"
                        );
                        self.git
                            .operation
                            .observe(*operation_epoch, operation.clone());
                        // A fast read populates an empty tree; only a full read replaces an existing tree.
                        let apply_status = !fast || self.files.view().tree_nodes.is_empty();
                        if apply_status {
                            self.apply_status_refresh(status.clone());
                        }
                        if *fast && self.refresh.pending_full_status_after_fast {
                            if self.start_status_load(false, output) {
                                self.refresh.pending_full_status_after_fast = false;
                            } else {
                                self.refresh.pending_refresh =
                                    Some(match self.refresh.pending_refresh {
                                        None => RefreshKind::StatusOnly,
                                        Some(existing) => Self::max_refresh_kind(
                                            existing,
                                            RefreshKind::StatusOnly,
                                        ),
                                    });
                            }
                        }
                        if apply_status
                            && self.should_schedule_diff_for_refresh(DiffRefreshSource::Status)
                        {
                            schedule_diff = true;
                        }
                        self.ui.dirty.mark_all();
                    }
                    payload @ (BackgroundPayload::Branches(_)
                    | BackgroundPayload::Stashes(_)
                    | BackgroundPayload::Commits(_)
                    | BackgroundPayload::CommitsFast(_)
                    | BackgroundPayload::BranchCommits { .. }
                    | BackgroundPayload::CommitFiles { .. }
                    | BackgroundPayload::StashFiles { .. }) => {
                        schedule_diff |= self.apply_revision_payload(payload);
                    }
                    BackgroundPayload::Diff { cache_key, diff } => {
                        let target = self.selected_diff_target();
                        if !self.diff.complete_load(
                            cache_key.clone(),
                            diff.clone(),
                            &target,
                            &self.files.view().tree_nodes,
                        ) {
                            self.schedule_diff_reload(output);
                            return;
                        }
                        self.ui.dirty.mark_diff();
                    }
                }
            }
            Err(reason) => {
                let key = &completed.request.key;
                if matches!(key, TaskKey::Status) {
                    self.refresh.pending_refresh_fast_done = false;
                }
                if matches!(key, TaskKey::BranchCommits { .. }) {
                    self.reduce_revisions(RevisionAction::BranchLoadFailed);
                }
                if matches!(key, TaskKey::Diff { .. }) {
                    self.diff.finish_load();
                }
                self.push_log(crate::app::git_error::message(reason), false);
            }
        }
        if refresh_search {
            self.restore_search_for_active_scope();
        }
        if schedule_diff {
            self.schedule_diff_reload(output);
        }
    }
    pub(crate) fn apply_runtime_event(&mut self, event: &RuntimeEvent, output: &mut CommandOutput) {
        match event {
            RuntimeEvent::NavigateDiffFile { forward } => self.navigate_diff_file(*forward, output),
            RuntimeEvent::ProcessBackgroundLoads => self.process_background_refresh_tick(output),
            RuntimeEvent::FlushPendingRefresh { log_success } => {
                self.process_background_refresh_tick(output);
                if *log_success {
                    self.push_log("refresh", true);
                }
            }
            RuntimeEvent::FlushPendingDiffReload => self.flush_pending_diff_reload(output),
            RuntimeEvent::EnsureCommitsLoadedForActivePanel => {
                self.ensure_commits_loaded_for_active_panel(output)
            }
            RuntimeEvent::ReloadDiffNow => self.reload_diff_now(output),
            RuntimeEvent::RevisionOpenTreeOrToggleDir => {
                match self.ui.active_panel {
                    SidePanel::Stash => self.stash_open_tree_or_toggle_dir(output),
                    SidePanel::Commits => self.commit_open_tree_or_toggle_dir(output),
                    SidePanel::LocalBranches => self.open_selected_branch_commits(100, output),
                    _ => {}
                }
                self.restore_search_for_active_scope();
                self.reload_diff_now(output);
                self.ui.dirty.mark_all();
            }
        }
    }
}
