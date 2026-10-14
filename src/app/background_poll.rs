use super::app::RefreshKind;
use super::diff_loader;
use super::{AppState, BackgroundCompleted, BackgroundPayload, SidePanel};
use crate::flux::effects::RuntimeEvent;
use crate::flux::task_manager::{TaskKey, TaskResult, TaskResultKind};
use std::collections::HashMap;
use tracing::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DiffRefreshSource {
    Status,
    Branches,
    Stashes,
    Commits,
}

impl AppState {
    pub(super) fn should_schedule_diff_for_refresh(&self, source: DiffRefreshSource) -> bool {
        use diff_loader::DiffTarget;
        matches!(
            (source, self.selected_diff_target()),
            (DiffRefreshSource::Status, DiffTarget::File { .. })
                | (DiffRefreshSource::Status, DiffTarget::Directory { .. })
                | (DiffRefreshSource::Branches, DiffTarget::Branch { .. })
                | (DiffRefreshSource::Stashes, DiffTarget::Stash { .. })
                | (DiffRefreshSource::Commits, DiffTarget::Commit { .. })
        )
    }

    pub fn process_background_refresh_tick(&mut self) {
        if let Some(kind) = self.refresh.pending_refresh.take() {
            let mut deferred_kind = None;
            if self.has_background_task(&TaskKey::Status) {
                deferred_kind = Some(kind);
            } else if !self.refresh.pending_refresh_fast_done {
                if self.start_status_load(true) {
                    self.refresh.pending_refresh_fast_done = true;
                    deferred_kind = Some(kind);
                } else {
                    deferred_kind = Some(RefreshKind::StatusOnly);
                }
            } else {
                if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full)
                    && (!self.start_branches_load() || !self.start_stashes_load())
                {
                    deferred_kind = Some(match deferred_kind {
                        None => RefreshKind::StatusAndRefs,
                        Some(existing) => {
                            Self::max_refresh_kind(existing, RefreshKind::StatusAndRefs)
                        }
                    });
                }
                if matches!(kind, RefreshKind::Full) {
                    self.ui.commits.dirty = true;
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

        if self.ui.active_panel == SidePanel::Commits && self.ui.commits.dirty {
            let _ = self.start_commits_load(self.tasks.commits_requested_limit);
        }
        self.maybe_schedule_commits_extend();

        self.start_pending_diff_load();
    }

    /// Accept results only for tasks still tracked by state. Cancellation and
    /// selection changes remove old tasks before a replacement is started.
    pub(crate) fn apply_background_completed(&mut self, completed: BackgroundCompleted) {
        let request = completed.request;
        if self
            .tasks
            .pending_background_tasks
            .remove(&request.generation)
            .is_none()
        {
            return;
        }
        let mut finished_payloads = HashMap::new();
        let kind = match completed.result {
            Ok(payload) => {
                finished_payloads.insert((request.key.clone(), request.generation), payload);
                TaskResultKind::Finished
            }
            Err(reason) => TaskResultKind::Failed { reason },
        };
        self.tasks.task_manager.submit_result(TaskResult {
            key: request.key,
            generation: request.generation,
            kind,
        });
        let mut schedule_diff = false;
        let mut refresh_search = false;
        for task in self.tasks.task_manager.collect_ready() {
            match task.kind {
                TaskResultKind::Finished => {
                    let Some(payload) = finished_payloads.remove(&(task.key, task.generation))
                    else {
                        continue;
                    };
                    refresh_search |= !matches!(&payload, BackgroundPayload::Diff { .. });
                    match payload {
                        BackgroundPayload::CommitFiles { oid, files } => {
                            if self.ui.commits.tree_mode.selected_source.as_deref() == Some(&oid) {
                                super::revision_tree::enter_tree_mode(
                                    oid,
                                    files,
                                    super::revision_tree::TreeModeState {
                                        tree_mode: &mut self.ui.commits.tree_mode.active,
                                        tree_nodes: &mut self.ui.commits.tree_mode.nodes,
                                        tree_files: &mut self.ui.commits.tree_mode.files,
                                        expanded_dirs: &mut self.ui.commits.tree_mode.expanded_dirs,
                                        selected_tree_revision: &mut self
                                            .ui
                                            .commits
                                            .tree_mode
                                            .selected_source,
                                        list_state: &mut self.ui.commits.panel.list_state,
                                    },
                                );
                                self.restore_search_for_active_scope();
                                self.ui.dirty.mark_all();
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::StashFiles { index, files } => {
                            if self.ui.stash.tree_mode.selected_source == Some(index) {
                                super::revision_tree::enter_tree_mode(
                                    index,
                                    files,
                                    super::revision_tree::TreeModeState {
                                        tree_mode: &mut self.ui.stash.tree_mode.active,
                                        tree_nodes: &mut self.ui.stash.tree_mode.nodes,
                                        tree_files: &mut self.ui.stash.tree_mode.files,
                                        expanded_dirs: &mut self.ui.stash.tree_mode.expanded_dirs,
                                        selected_tree_revision: &mut self
                                            .ui
                                            .stash
                                            .tree_mode
                                            .selected_source,
                                        list_state: &mut self.ui.stash.panel.list_state,
                                    },
                                );
                                self.restore_search_for_active_scope();
                                self.ui.dirty.mark_all();
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::Status {
                            status,
                            fast,
                            operation,
                            operation_epoch,
                        } => {
                            debug!(
                                event = "status_load_finished",
                                mode = if fast { "fast" } else { "full" },
                                staged = status.staged.len(),
                                unstaged = status.unstaged.len(),
                                untracked = status.untracked.len(),
                                "status load finished"
                            );
                            self.git.operation.observe(operation_epoch, operation);
                            // Fast reads omit untracked paths. A hunk edit can turn an
                            // indexed file into an untracked file, so retain its selection
                            // until the complete status arrives.
                            if !fast || !self.ui.diff.focused {
                                self.apply_status_refresh(status);
                            }
                            if fast && self.refresh.pending_full_status_after_fast {
                                if self.start_status_load(false) {
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
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Status) {
                                schedule_diff = true;
                            }
                            self.ui.dirty.mark_all();
                        }
                        BackgroundPayload::Branches(items) => {
                            self.ui.branches.items = items;
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Branches) {
                                schedule_diff = true;
                            }
                            self.ui.dirty.mark_all();
                        }
                        BackgroundPayload::Stashes(items) => {
                            self.ui.stash.items = items;
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Stashes) {
                                schedule_diff = true;
                            }
                            self.ui.dirty.mark_all();
                        }
                        BackgroundPayload::Commits(items) => {
                            debug!(
                                event = "commits_load_finished",
                                count = items.len(),
                                requested_limit = self.tasks.commits_requested_limit,
                                "commits load finished"
                            );
                            self.ui.commits.items = items;
                            self.ui.commits.dirty = false;
                            self.ui.dirty.mark_all();
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits) {
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::CommitsFast(items) => {
                            debug!(
                                event = "commits_load_finished",
                                mode = "fast",
                                count = items.len(),
                                requested_limit = self.tasks.commits_requested_limit,
                                "commits fast load finished"
                            );
                            self.ui.commits.items = items;
                            self.ui.commits.dirty = true;
                            self.ui.dirty.mark_all();
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits) {
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::BranchCommits { branch, items } => {
                            if self.ui.branches.commits_subview_source.as_deref()
                                == Some(branch.as_str())
                                && self.ui.branches.commits_subview_active
                            {
                                self.ui.branches.commits_subview.items = items;
                                self.ui.branches.commits_subview_loading = false;
                                self.ui.branches.commits_subview.panel.list_state.select(
                                    (!self.ui.branches.commits_subview.items.is_empty())
                                        .then_some(0),
                                );
                                self.ui.dirty.mark_all();
                                if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits)
                                {
                                    schedule_diff = true;
                                }
                            }
                        }
                        BackgroundPayload::Diff { cache_key, diff } => {
                            if self.diff_target_to_cache_key(&self.selected_diff_target())
                                != cache_key
                            {
                                self.diff_mgr.in_flight_diff_key = None;
                                self.schedule_diff_reload();
                                continue;
                            }
                            self.diff_mgr.in_flight_diff_key = None;
                            self.diff_mgr.cache.insert(cache_key.clone(), diff.clone());
                            self.diff_mgr.last_diff_key = Some(cache_key);
                            self.git.current_diff = diff.lines;
                            self.git.current_patch = diff.patch;
                            self.ui.diff.reset_selection();
                            self.ui.diff.cursor = self
                                .git
                                .current_patch
                                .as_ref()
                                .and_then(|patch| patch.hunk_ids().first().copied());
                            if !self.refresh.pending_diff_reload {
                                self.refresh.pending_diff_reload_at = None;
                            }
                            self.ui.dirty.mark_diff();
                        }
                    }
                }
                TaskResultKind::Failed { reason } => {
                    if matches!(task.key, TaskKey::Status) {
                        self.refresh.pending_refresh_fast_done = false;
                    }
                    if matches!(task.key, TaskKey::BranchCommits { .. }) {
                        self.ui.branches.commits_subview_loading = false;
                    }
                    if matches!(task.key, TaskKey::Diff { .. }) {
                        self.diff_mgr.in_flight_diff_key = None;
                    }
                    if matches!(task.key, TaskKey::Diff { .. }) && !self.refresh.pending_diff_reload
                    {
                        self.refresh.pending_diff_reload_at = None;
                    }
                    self.push_log(reason, false);
                }
            }
        }

        if refresh_search {
            self.restore_search_for_active_scope();
        }
        if schedule_diff {
            self.schedule_diff_reload();
        }
    }
    pub(crate) fn apply_runtime_event(&mut self, event: &RuntimeEvent) {
        match event {
            RuntimeEvent::ProcessBackgroundLoads => self.process_background_refresh_tick(),
            RuntimeEvent::FlushPendingRefresh { log_success } => {
                self.process_background_refresh_tick();
                if *log_success {
                    self.push_log("refresh", true);
                }
            }
            RuntimeEvent::FlushPendingDiffReload => self.flush_pending_diff_reload(),
            RuntimeEvent::EnsureCommitsLoadedForActivePanel => {
                self.ensure_commits_loaded_for_active_panel()
            }
            RuntimeEvent::ReloadDiffNow => self.reload_diff_now(),
            RuntimeEvent::RevisionOpenTreeOrToggleDir => {
                let result = match self.ui.active_panel {
                    SidePanel::Stash => self.stash_open_tree_or_toggle_dir(),
                    SidePanel::Commits => self.commit_open_tree_or_toggle_dir(),
                    SidePanel::LocalBranches => self.open_selected_branch_commits(100),
                    _ => Ok(()),
                };
                if let Err(err) = result {
                    self.push_log(format!("revision files failed: {err}"), false);
                }
                self.restore_search_for_active_scope();
                self.reload_diff_now();
                self.ui.dirty.mark_all();
            }
        }
    }
}
