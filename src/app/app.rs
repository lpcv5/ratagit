use super::background_poll::{BackgroundReceiver, PendingBackgroundTask};
use super::background_task_runner::BackgroundTaskRunner;
use super::branch_panel_adapter;
use super::diff_cache_manager::DiffCacheManager;
use super::refresh_scheduler::RefreshScheduler;
use super::states::{
    BranchesPanelState, CommandLogEntry, CommitsPanelState, FilesPanelState, GitState, InputState,
    PanelState, RenderCache, SidePanel, StashPanelState, TreeModeState, UiState,
};
use super::{diff_cache, diff_loader, dirty_flags, files_panel_adapter, revision_tree};
use crate::config::keymap::Keymap;
use crate::flux::branch_backend::BranchPanelViewState;
use crate::flux::files_backend::{FilesBackend, FilesBackendCommand, FilesPanelViewState};
use crate::flux::task_manager::{TaskKey, TaskPriority, TaskRequestKind};
use crate::git::{Git2Repository, GitError, GitRepository, GitStatus};
use crate::ui::widgets::file_tree::FileTree;
use color_eyre::Result;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use tracing::debug;

const MAX_NORMAL_PRIORITY_TASKS: usize = 6;
const INITIAL_COMMITS_LOAD_LIMIT: usize = 30;
const COMMITS_LOAD_STEP: usize = 40;
const COMMITS_LOAD_AHEAD_THRESHOLD: usize = 8;
const BRANCH_LOG_DIFF_LIMIT: usize = 20;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    CommitEditor,
    CommandPalette,
    CreateBranch,
    StashEditor,
    Search,
    BranchSwitchConfirm,
    CommitAllConfirm,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CommitFieldFocus {
    Message,
    Description,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshKind {
    StatusOnly,
    StatusAndRefs,
    Full,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SearchScopeKey {
    pub panel: SidePanel,
    pub commit_tree_mode: bool,
    pub stash_tree_mode: bool,
}

/// Documentation comment in English.
pub struct App {
    pub running: bool,
    pub git: GitState,
    pub ui: UiState,
    pub input: InputState,
    pub command_log: Vec<CommandLogEntry>,

    repo: Box<dyn GitRepository>,
    pub(super) refresh: RefreshScheduler,
    pub(super) diff_mgr: DiffCacheManager,
    pub(super) tasks: BackgroundTaskRunner,
    keymap: Keymap,
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Git2Repository::discover()?;
        Self::build_with_repo(Box::new(repo), Keymap::load(), false)
    }

    #[cfg(test)]
    pub fn from_repo(repo: Box<dyn GitRepository>) -> Result<Self> {
        Self::build_with_repo(repo, Keymap::default(), true)
    }

    fn build_with_repo(
        repo: Box<dyn GitRepository>,
        keymap: Keymap,
        preload_commits_and_diff: bool,
    ) -> Result<Self> {
        let status = if preload_commits_and_diff {
            repo.status()?
        } else {
            GitStatus::default()
        };

        // Comment in English.
        let expanded_dirs = if preload_commits_and_diff {
            FilesBackend::all_dirs(&status)
        } else {
            HashSet::new()
        };
        let file_tree_nodes = FileTree::from_git_status_with_expanded(
            &status.unstaged,
            &status.untracked,
            &status.staged,
            &expanded_dirs,
        );

        let branches = if preload_commits_and_diff {
            repo.branches().unwrap_or_default()
        } else {
            Vec::new()
        };
        let commits = if preload_commits_and_diff {
            repo.commits(100).unwrap_or_default()
        } else {
            Vec::new()
        };
        let stashes = if preload_commits_and_diff {
            repo.stashes().unwrap_or_default()
        } else {
            Vec::new()
        };

        let mut app = Self {
            running: true,
            git: GitState {
                status,
                current_diff: Vec::new(),
            },
            ui: UiState {
                active_panel: SidePanel::Files,
                files: FilesPanelState {
                    panel: PanelState::new(),
                    tree_nodes: file_tree_nodes,
                    expanded_dirs,
                    visual_mode: false,
                    visual_anchor: None,
                },
                branches: BranchesPanelState {
                    panel: PanelState::new(),
                    items: branches,
                    is_fetching_remote: false,
                    commits_subview_active: false,
                    commits_subview_loading: false,
                    commits_subview_source: None,
                    commits_subview: CommitsPanelState::default(),
                },
                commits: CommitsPanelState {
                    panel: PanelState::new(),
                    items: commits,
                    dirty: !preload_commits_and_diff,
                    tree_mode: TreeModeState::default(),
                    highlighted_oids: HashSet::new(),
                },
                stash: StashPanelState {
                    panel: PanelState::new(),
                    items: stashes,
                    tree_mode: TreeModeState::default(),
                },
                diff_scroll: 0,
                dirty: dirty_flags::DirtyFlags::default(),
                render_cache: RenderCache::default(),
            },
            input: InputState::default(),
            command_log: Vec::new(),
            repo,
            refresh: RefreshScheduler::new(!preload_commits_and_diff),
            diff_mgr: DiffCacheManager::new(),
            tasks: BackgroundTaskRunner::new(INITIAL_COMMITS_LOAD_LIMIT),
            keymap,
        };
        if preload_commits_and_diff
            && (!app.git.status.staged.is_empty()
                || !app.git.status.unstaged.is_empty()
                || !app.git.status.untracked.is_empty())
        {
            app.sync_files_view_from_status();
        }
        app.refresh_render_cache();
        app.ui.dirty.mark_all();
        if preload_commits_and_diff {
            app.reload_diff_now();
        } else {
            app.request_refresh(RefreshKind::StatusAndRefs);
            app.schedule_diff_reload();
            // Load commits immediately on startup
            let _ = app.start_commits_load(app.tasks.commits_requested_limit);
        }
        Ok(app)
    }

    pub(crate) fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn apply_refresh(&mut self, kind: RefreshKind) -> Result<()> {
        self.git.status = self.repo.status()?;
        self.sync_files_view_from_status();

        if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full) {
            self.ui.branches.items = self.repo.branches().unwrap_or_default();
            self.ui.stash.items = self.repo.stashes().unwrap_or_default();
            if self.ui.stash.tree_mode.active {
                if let Some(index) = self.ui.stash.tree_mode.selected_source {
                    if self.ui.stash.items.iter().any(|s| s.index == index) {
                        self.ui.stash.tree_mode.files =
                            self.repo.stash_files(index).unwrap_or_default();
                        revision_tree::rebuild_tree_nodes(
                            &self.ui.stash.tree_mode.files,
                            &self.ui.stash.tree_mode.expanded_dirs,
                            &mut self.ui.stash.tree_mode.nodes,
                            &mut self.ui.stash.panel.list_state,
                        );
                    } else {
                        self.stash_close_tree();
                    }
                } else {
                    self.stash_close_tree();
                }
            }
        }

        if matches!(kind, RefreshKind::Full) {
            self.reload_commits_now();
        }

        Ok(())
    }

    pub fn request_refresh(&mut self, kind: RefreshKind) {
        self.refresh.pending_full_status_after_fast = true;
        self.refresh.pending_refresh_fast_done = false;
        if matches!(kind, RefreshKind::Full) {
            self.ui.commits.dirty = true;
        }
        self.diff_mgr.cache.invalidate_files();
        if matches!(
            self.diff_mgr.last_diff_key,
            Some(diff_cache::DiffCacheKey::File { .. })
                | Some(diff_cache::DiffCacheKey::Directory { .. })
        ) {
            self.diff_mgr.last_diff_key = None;
        }
        self.refresh.pending_refresh = Some(match self.refresh.pending_refresh {
            None => kind,
            Some(existing) => Self::max_refresh_kind(existing, kind),
        });
    }

    pub fn flush_pending_refresh(&mut self) -> Result<bool> {
        let Some(kind) = self.refresh.pending_refresh.take() else {
            return Ok(false);
        };
        self.apply_refresh(kind)?;
        self.schedule_diff_reload();
        self.ui.dirty.mark_all();
        Ok(true)
    }

    pub(super) fn has_background_task(&self, key: &TaskKey) -> bool {
        self.tasks
            .pending_background_tasks
            .values()
            .any(|task| &task.request.key == key)
    }

    pub(super) fn can_start_background_task(&self, priority: TaskPriority) -> bool {
        match priority {
            TaskPriority::High => true,
            TaskPriority::Normal | TaskPriority::Low => {
                self.tasks.pending_background_tasks.len() < MAX_NORMAL_PRIORITY_TASKS
            }
        }
    }

    pub(super) fn diff_task_key() -> TaskKey {
        TaskKey::Diff {
            target: "active".to_string(),
        }
    }

    pub(super) fn has_active_diff_task(&self) -> bool {
        let key = Self::diff_task_key();
        self.has_background_task(&key)
    }

    pub(super) fn cancel_pending_diff_task(&mut self) {
        let key = Self::diff_task_key();
        let _ = self.tasks.task_manager.cancel(&key);
        self.tasks
            .pending_background_tasks
            .retain(|_, task| task.request.key != key);
        self.diff_mgr.in_flight_diff_key = None;
    }

    pub(super) fn start_status_load(&mut self, fast: bool) -> bool {
        let key = TaskKey::Status;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::High;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request =
            self.tasks
                .task_manager
                .enqueue(key.clone(), priority, TaskRequestKind::LoadStatus);
        let mode = if fast { "fast" } else { "full" };
        match if fast {
            self.repo.status_fast_async()
        } else {
            self.repo.status_async()
        } {
            Ok(rx) => {
                debug!(
                    event = "start_status_load",
                    mode = mode,
                    generation = request.generation.0,
                    "scheduled status load"
                );
                self.tasks
                    .task_manager
                    .mark_started(&key, request.generation);
                self.tasks.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Status { fast, rx },
                    },
                );
                true
            }
            Err(err) => {
                self.tasks
                    .task_manager
                    .mark_finished(&key, request.generation);
                self.push_log(format!("status load failed: {}", err), false);
                true
            }
        }
    }

    pub(super) fn start_branches_load(&mut self) -> bool {
        let key = TaskKey::Branches;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request =
            self.tasks
                .task_manager
                .enqueue(key.clone(), priority, TaskRequestKind::LoadBranches);
        match self.repo.branches_async() {
            Ok(rx) => {
                self.tasks
                    .task_manager
                    .mark_started(&key, request.generation);
                self.tasks.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Branches(rx),
                    },
                );
                true
            }
            Err(err) => {
                self.tasks
                    .task_manager
                    .mark_finished(&key, request.generation);
                self.push_log(format!("branches load failed: {}", err), false);
                true
            }
        }
    }

    pub(super) fn start_stashes_load(&mut self) -> bool {
        let key = TaskKey::Stashes;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request =
            self.tasks
                .task_manager
                .enqueue(key.clone(), priority, TaskRequestKind::LoadStashes);
        match self.repo.stashes_async() {
            Ok(rx) => {
                self.tasks
                    .task_manager
                    .mark_started(&key, request.generation);
                self.tasks.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Stashes(rx),
                    },
                );
                true
            }
            Err(err) => {
                self.tasks
                    .task_manager
                    .mark_finished(&key, request.generation);
                self.push_log(format!("stashes load failed: {}", err), false);
                true
            }
        }
    }

    pub(super) fn start_commits_load(&mut self, limit: usize) -> bool {
        let key = TaskKey::Commits;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request = self.tasks.task_manager.enqueue(
            key.clone(),
            priority,
            TaskRequestKind::LoadCommits { limit },
        );
        self.tasks.commits_requested_limit = self.tasks.commits_requested_limit.max(limit);
        let fast = self.ui.commits.items.is_empty();
        let mode = if fast { "fast" } else { "full" };
        match if fast {
            self.repo.commits_fast_async(limit)
        } else {
            self.repo.commits_async(limit)
        } {
            Ok(rx) => {
                debug!(
                    event = "start_commits_load",
                    limit = limit,
                    mode = mode,
                    generation = request.generation.0,
                    "scheduled commits load"
                );
                self.tasks
                    .task_manager
                    .mark_started(&key, request.generation);
                self.tasks.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: if fast {
                            BackgroundReceiver::CommitsFast(rx)
                        } else {
                            BackgroundReceiver::Commits(rx)
                        },
                    },
                );
                true
            }
            Err(err) => {
                self.tasks
                    .task_manager
                    .mark_finished(&key, request.generation);
                self.push_log(format!("commits load failed: {}", err), false);
                true
            }
        }
    }

    pub(super) fn apply_status_refresh(&mut self, status: GitStatus) {
        self.git.status = status;
        self.sync_files_view_from_status();
    }

    pub fn schedule_diff_reload(&mut self) {
        self.refresh.pending_diff_reload = true;
        self.refresh.pending_diff_reload_at = Some(Instant::now());
        debug!(event = "schedule_diff_reload", "scheduled diff reload");
    }

    pub fn has_pending_diff_reload(&self) -> bool {
        self.refresh.pending_diff_reload
    }

    pub fn has_pending_refresh_work(&self) -> bool {
        self.refresh.pending_refresh.is_some() || !self.tasks.pending_background_tasks.is_empty()
    }

    pub fn diff_reload_debounce_elapsed(&self, debounce: std::time::Duration) -> bool {
        self.refresh
            .pending_diff_reload_at
            .is_some_and(|requested_at| requested_at.elapsed() >= debounce)
    }

    pub fn pending_diff_reload_at(&self) -> Option<Instant> {
        self.refresh.pending_diff_reload_at
    }

    pub fn flush_pending_diff_reload(&mut self) {
        if !self.refresh.pending_diff_reload && !self.has_active_diff_task() {
            return;
        }
        self.start_pending_diff_load();
    }

    pub(super) fn pending_refresh_kind(&self) -> Option<RefreshKind> {
        self.refresh.pending_refresh
    }

    pub(super) fn maybe_schedule_commits_extend(&mut self) {
        let Some(next_limit) = crate::flux::commits_backend::CommitsBackend::load_ahead_limit(
            &self.current_commits_view_state(),
            self.ui.active_panel,
            self.has_background_task(&TaskKey::Commits),
            self.ui.commits.dirty,
            self.tasks.commits_requested_limit,
            COMMITS_LOAD_AHEAD_THRESHOLD,
            COMMITS_LOAD_STEP,
        ) else {
            return;
        };
        let selected = self.ui.commits.panel.list_state.selected();
        let len = self.ui.commits.items.len();
        self.tasks.commits_requested_limit = next_limit;
        self.ui.commits.dirty = true;
        debug!(
            event = "commits_extend_requested",
            selected = selected,
            current = len,
            next_limit = self.tasks.commits_requested_limit,
            "scheduled commits extension"
        );
    }

    pub fn ensure_commits_loaded_for_active_panel(&mut self) {
        if self.ui.active_panel == SidePanel::Commits && self.ui.commits.dirty {
            let _ = self.start_commits_load(self.tasks.commits_requested_limit);
        }
    }

    pub(super) fn clear_pending_diff_reload(&mut self) {
        self.refresh.pending_diff_reload = false;
        self.refresh.pending_diff_reload_at = None;
    }

    pub(super) fn start_pending_diff_load(&mut self) {
        if !self.refresh.pending_diff_reload {
            return;
        }
        let target = self.selected_diff_target();
        let key = self.diff_target_to_cache_key(&target);

        if self.diff_mgr.in_flight_diff_key.as_ref() == Some(&key) && self.has_active_diff_task() {
            self.clear_pending_diff_reload();
            return;
        }

        if self.diff_mgr.last_diff_key.as_ref() == Some(&key) {
            self.cancel_pending_diff_task();
            self.clear_pending_diff_reload();
            return;
        }

        self.ui.diff_scroll = 0;
        if let Some(cached) = self.diff_mgr.cache.get_cloned(&key) {
            self.git.current_diff = cached;
            self.diff_mgr.last_diff_key = Some(key);
            self.clear_pending_diff_reload();
            self.ui.dirty.mark_diff();
            return;
        }

        self.cancel_pending_diff_task();

        let key_for_task = Self::diff_task_key();
        let target_label = Self::diff_cache_key_to_task_target(&key);
        let request = self.tasks.task_manager.enqueue(
            key_for_task.clone(),
            TaskPriority::High,
            TaskRequestKind::LoadDiff {
                target: target_label,
            },
        );
        let request_generation = request.generation.0;

        let rx = match target {
            diff_loader::DiffTarget::None => {
                self.tasks
                    .task_manager
                    .mark_finished(&key_for_task, request.generation);
                self.git.current_diff = Vec::new();
                self.diff_mgr.last_diff_key = Some(key);
                self.clear_pending_diff_reload();
                self.ui.dirty.mark_diff();
                return;
            }
            diff_loader::DiffTarget::Branch { name } => {
                self.repo.branch_log_async(name, BRANCH_LOG_DIFF_LIMIT)
            }
            diff_loader::DiffTarget::File { path, status } => match status {
                crate::ui::widgets::file_tree::FileTreeNodeStatus::Staged(_) => {
                    self.repo.diff_staged_async(path)
                }
                crate::ui::widgets::file_tree::FileTreeNodeStatus::Untracked => {
                    self.repo.diff_untracked_async(path)
                }
                crate::ui::widgets::file_tree::FileTreeNodeStatus::Unstaged(_) => {
                    self.repo.diff_unstaged_async(path)
                }
                crate::ui::widgets::file_tree::FileTreeNodeStatus::Directory => {
                    self.repo.diff_directory_async(path)
                }
            },
            diff_loader::DiffTarget::Directory { path } => self.repo.diff_directory_async(path),
            diff_loader::DiffTarget::Commit { oid, path } => {
                self.repo.commit_diff_scoped_async(oid, path)
            }
            diff_loader::DiffTarget::Stash { index, path } => {
                self.repo.stash_diff_async(index, path)
            }
        };

        match rx {
            Ok(rx) => {
                match rx.try_recv() {
                    Ok(Ok(diff)) => {
                        self.diff_mgr.cache.insert(key.clone(), diff.clone());
                        self.diff_mgr.last_diff_key = Some(key);
                        self.git.current_diff = diff;
                        self.clear_pending_diff_reload();
                        self.ui.dirty.mark_diff();
                        return;
                    }
                    Ok(Err(err)) => {
                        self.push_log(format!("diff load failed: {}", err), false);
                        self.clear_pending_diff_reload();
                        return;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.push_log("diff load disconnected".to_string(), false);
                        self.clear_pending_diff_reload();
                        return;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
                self.tasks
                    .task_manager
                    .mark_started(&key_for_task, request.generation);
                self.tasks.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Diff {
                            cache_key: key.clone(),
                            rx,
                        },
                    },
                );
                self.diff_mgr.in_flight_diff_key = Some(key);
                self.clear_pending_diff_reload();
                debug!(
                    event = "start_diff_load",
                    generation = request_generation,
                    target = ?self.diff_mgr.in_flight_diff_key,
                    "scheduled diff load"
                );
            }
            Err(err) => {
                self.tasks
                    .task_manager
                    .mark_finished(&key_for_task, request.generation);
                self.push_log(format!("diff load failed: {}", err), false);
                self.clear_pending_diff_reload();
            }
        }
    }

    pub(crate) fn start_branch_commits_background_load(
        &mut self,
        branch_name: String,
        limit: usize,
    ) -> Result<()> {
        let key = TaskKey::BranchCommits {
            branch: branch_name.clone(),
        };
        let request = self.tasks.task_manager.enqueue(
            key.clone(),
            TaskPriority::High,
            TaskRequestKind::LoadBranchCommits {
                branch: branch_name.clone(),
                limit,
            },
        );
        match self.repo.commits_for_branch_async(&branch_name, limit) {
            Ok(rx) => {
                self.tasks
                    .task_manager
                    .mark_started(&key, request.generation);
                self.tasks.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::BranchCommits {
                            branch: branch_name.clone(),
                            rx,
                        },
                    },
                );
            }
            Err(err) => {
                self.tasks
                    .task_manager
                    .mark_finished(&key, request.generation);
                return Err(err.into());
            }
        }
        Ok(())
    }

    pub fn close_branch_commits_subview(&mut self) {
        let view = crate::flux::branch_backend::BranchBackend::close_commits_subview(
            self.current_branches_view_state(),
        );
        self.apply_branches_backend_view(view);
    }

    pub fn toggle_visual_select_mode(&mut self) {
        if self.ui.active_panel != SidePanel::Files {
            return;
        }
        if self.ui.files.visual_mode {
            self.ui.files.visual_mode = false;
            self.ui.files.visual_anchor = None;
            return;
        }

        self.ui.files.visual_mode = true;
        self.ui.files.visual_anchor = self.ui.files.panel.list_state.selected();
    }

    pub fn push_log<S: Into<String>>(&mut self, command: S, success: bool) {
        self.command_log.push(CommandLogEntry {
            command: command.into(),
            success,
        });
        const MAX_LOG_ENTRIES: usize = 200;
        if self.command_log.len() > MAX_LOG_ENTRIES {
            let drain_count = self.command_log.len() - MAX_LOG_ENTRIES;
            self.command_log.drain(0..drain_count);
        }
        self.ui.dirty.mark_command_log();
    }

    #[cfg(test)]
    pub fn stage_file(&mut self, path: PathBuf) -> Result<()> {
        self.repo.stage(&path)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    #[cfg(test)]
    pub fn unstage_file(&mut self, path: PathBuf) -> Result<()> {
        self.repo.unstage(&path)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    #[cfg(test)]
    pub fn discard_paths(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.discard_paths(paths)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    #[allow(dead_code)]
    pub fn stage_paths(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.stage_paths(paths)?;
        self.request_refresh(RefreshKind::StatusOnly);
        Ok(())
    }

    pub(super) fn stage_paths_internal(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.stage_paths(paths)?;
        Ok(())
    }

    pub(super) fn unstage_paths_internal(&mut self, paths: &[PathBuf]) -> Result<()> {
        self.repo.unstage_paths(paths)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn commit(&mut self, message: &str) -> Result<String> {
        let oid = self.repo.commit(message)?;
        self.request_refresh(RefreshKind::Full);
        Ok(oid)
    }

    #[cfg(test)]
    pub fn create_branch(&mut self, name: &str) -> Result<()> {
        self.repo.create_branch(name)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    #[cfg(test)]
    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        self.repo.checkout_branch(name)?;
        self.request_refresh(RefreshKind::Full);
        Ok(())
    }

    pub fn has_uncommitted_changes(&self) -> bool {
        !self.git.status.staged.is_empty()
            || !self.git.status.unstaged.is_empty()
            || !self.git.status.untracked.is_empty()
    }

    pub fn start_branch_switch_confirm(&mut self, target: String) {
        self.input.mode = Some(InputMode::BranchSwitchConfirm);
        self.input.branch_switch_target = Some(target);
    }

    pub fn pending_branch_switch_target(&self) -> Option<&str> {
        self.input.branch_switch_target.as_deref()
    }

    pub fn take_branch_switch_target(&mut self) -> Option<String> {
        self.input.branch_switch_target.take()
    }

    #[cfg(test)]
    pub fn delete_branch(&mut self, name: &str) -> Result<()> {
        self.repo.delete_branch(name)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    pub fn fetch_remote_request(&self) -> Result<Receiver<Result<String, GitError>>> {
        Ok(self.repo.fetch_default_async()?)
    }

    pub fn stage_file_request(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.stage_async(path)?)
    }

    pub fn stage_paths_request(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.stage_paths_async(paths)?)
    }

    pub fn unstage_file_request(&self, path: PathBuf) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.unstage_async(path)?)
    }

    pub fn discard_paths_request(
        &self,
        paths: Vec<PathBuf>,
    ) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.discard_paths_async(paths)?)
    }

    pub fn commit_request(&self, message: String) -> Result<Receiver<Result<String, GitError>>> {
        Ok(self.repo.commit_async(message)?)
    }

    pub fn create_branch_request(&self, name: String) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.create_branch_async(name)?)
    }

    pub fn checkout_branch_request(
        &self,
        name: String,
        auto_stash: bool,
    ) -> Result<Receiver<Result<(), GitError>>> {
        if auto_stash {
            Ok(self.repo.checkout_branch_with_auto_stash_async(name)?)
        } else {
            Ok(self.repo.checkout_branch_async(name)?)
        }
    }

    pub fn delete_branch_request(&self, name: String) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.delete_branch_async(name)?)
    }

    pub fn stash_push_request(
        &self,
        paths: Vec<PathBuf>,
        message: String,
    ) -> Result<Receiver<Result<usize, GitError>>> {
        Ok(self.repo.stash_push_paths_async(paths, message)?)
    }

    pub fn stash_apply_request(&self, index: usize) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.stash_apply_async(index)?)
    }

    pub fn stash_pop_request(&self, index: usize) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.stash_pop_async(index)?)
    }

    pub fn stash_drop_request(&self, index: usize) -> Result<Receiver<Result<(), GitError>>> {
        Ok(self.repo.stash_drop_async(index)?)
    }

    pub fn git_log_graph_request(
        &self,
        branch: Option<String>,
    ) -> Result<Receiver<Result<Vec<String>, GitError>>> {
        Ok(self.repo.git_log_graph_async(branch)?)
    }

    #[cfg(test)]
    pub fn stash_apply(&mut self, index: usize) -> Result<()> {
        self.repo.stash_apply(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    #[cfg(test)]
    pub fn stash_pop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_pop(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    #[cfg(test)]
    pub fn stash_drop(&mut self, index: usize) -> Result<()> {
        self.repo.stash_drop(index)?;
        self.request_refresh(RefreshKind::StatusAndRefs);
        Ok(())
    }

    /// Documentation comment in English.
    pub fn diff_scroll_up(&mut self) {
        self.ui.diff_scroll = self.ui.diff_scroll.saturating_sub(10);
    }

    pub fn diff_scroll_down(&mut self) {
        let max = self.git.current_diff.len().saturating_sub(1);
        self.ui.diff_scroll = (self.ui.diff_scroll + 10).min(max);
    }

    pub fn stash_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.ui.active_panel != SidePanel::Stash {
            return Ok(());
        }

        if !self.ui.stash.tree_mode.active {
            let Some(index) = self.selected_stash_index() else {
                return Ok(());
            };
            let files = self.repo.stash_files(index)?;
            revision_tree::enter_tree_mode(
                index,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.ui.stash.tree_mode.active,
                    tree_nodes: &mut self.ui.stash.tree_mode.nodes,
                    tree_files: &mut self.ui.stash.tree_mode.files,
                    expanded_dirs: &mut self.ui.stash.tree_mode.expanded_dirs,
                    selected_tree_revision: &mut self.ui.stash.tree_mode.selected_source,
                    list_state: &mut self.ui.stash.panel.list_state,
                },
            );
            return Ok(());
        }

        let selected_dir_path = self.selected_stash_tree_node().and_then(|node| {
            if node.is_dir {
                Some(node.path.clone())
            } else {
                None
            }
        });
        revision_tree::toggle_tree_dir(
            selected_dir_path,
            &self.ui.stash.tree_mode.files,
            &mut self.ui.stash.tree_mode.expanded_dirs,
            &mut self.ui.stash.tree_mode.nodes,
            &mut self.ui.stash.panel.list_state,
        );
        Ok(())
    }

    pub fn stash_close_tree(&mut self) {
        let selected_source_index =
            self.ui
                .stash
                .tree_mode
                .selected_source
                .and_then(|stash_index| {
                    self.ui
                        .stash
                        .items
                        .iter()
                        .position(|s| s.index == stash_index)
                });

        let was_open = self.ui.stash.tree_mode.active;
        revision_tree::close_tree_mode(
            &mut self.ui.stash.tree_mode.active,
            &mut self.ui.stash.tree_mode.nodes,
            &mut self.ui.stash.tree_mode.files,
            &mut self.ui.stash.tree_mode.expanded_dirs,
            &mut self.ui.stash.panel.list_state,
            selected_source_index,
            self.ui.stash.items.len(),
        );
        if was_open {
            self.ui.stash.tree_mode.selected_source = None;
        }
    }

    pub fn reload_diff_now(&mut self) {
        self.schedule_diff_reload();
        self.start_pending_diff_load();
    }

    fn sync_files_view_from_status(&mut self) {
        let selection = files_panel_adapter::selection_state_from_shell(&self.ui.files);
        let mut expanded_dirs = self.ui.files.expanded_dirs.clone();
        expanded_dirs.extend(FilesBackend::all_dirs(&self.git.status));
        let view = FilesBackend::handle_command(FilesBackendCommand::RefreshFromStatus {
            status: self.git.status.clone(),
            expanded_dirs,
            selection,
        });
        self.apply_files_backend_view(view);
    }

    pub(crate) fn current_files_view_state(&self) -> FilesPanelViewState {
        files_panel_adapter::view_state_from_shell(&self.ui.files)
    }

    pub(crate) fn current_branches_view_state(&self) -> BranchPanelViewState {
        branch_panel_adapter::view_state_from_shell(&self.ui.branches)
    }

    pub(crate) fn current_commits_view_state(
        &self,
    ) -> crate::flux::commits_backend::CommitsPanelViewState {
        crate::app::commits_panel_adapter::view_state_from_shell(&self.ui.commits)
    }

    pub(crate) fn apply_files_backend_view(
        &mut self,
        event: crate::flux::files_backend::FilesBackendEvent,
    ) {
        if let crate::flux::files_backend::FilesBackendEvent::ViewStateUpdated(view) = event {
            files_panel_adapter::apply_view_state(&mut self.ui.files, view);
        }
    }

    pub(crate) fn apply_branches_backend_view(&mut self, view: BranchPanelViewState) {
        branch_panel_adapter::apply_view_state(&mut self.ui.branches, view);
    }

    pub(crate) fn apply_commits_backend_view(
        &mut self,
        event: crate::flux::commits_backend::CommitsBackendEvent,
    ) {
        let crate::flux::commits_backend::CommitsBackendEvent::ViewStateUpdated { view, dirty } =
            event;
        crate::app::commits_panel_adapter::apply_view_state(&mut self.ui.commits, view);
        if let Some(dirty) = dirty {
            self.ui.commits.dirty = dirty;
        }
    }

    pub(crate) fn apply_commits_backend_command(
        &mut self,
        command: crate::flux::commits_backend::CommitsBackendCommand,
    ) -> Result<()> {
        use crate::flux::commits_backend::{CommitsBackend, CommitsBackendCommand};

        let event = match command {
            CommitsBackendCommand::ApplyLoaded { items, mode } => {
                CommitsBackend::apply_loaded(self.current_commits_view_state(), items, mode)
            }
            CommitsBackendCommand::CloseTree => {
                CommitsBackend::close_tree(self.current_commits_view_state())
            }
            CommitsBackendCommand::OpenTreeOrToggleDir => {
                return self.commit_open_tree_or_toggle_dir();
            }
            CommitsBackendCommand::RecomputeHighlight => CommitsBackend::recompute_highlight(
                self.current_commits_view_state(),
                self.ui.active_panel,
            ),
        };
        self.apply_commits_backend_view(event);
        Ok(())
    }

    pub(super) fn diff_target_to_cache_key(
        &self,
        target: &diff_loader::DiffTarget,
    ) -> diff_cache::DiffCacheKey {
        use crate::ui::widgets::file_tree::FileTreeNodeStatus;
        use diff_loader::DiffTarget;

        match target {
            DiffTarget::Branch { name } => diff_cache::DiffCacheKey::Branch {
                name: name.clone(),
                limit: 100,
            },
            DiffTarget::File { path, status } => {
                let is_staged = matches!(status, FileTreeNodeStatus::Staged(_));
                diff_cache::DiffCacheKey::File {
                    path: path.clone(),
                    is_staged,
                }
            }
            DiffTarget::Directory { path } => {
                let hash = self
                    .current_files_view_state()
                    .nodes
                    .iter()
                    .filter(|n| n.path.starts_with(path))
                    .map(|n| n.path.to_string_lossy().to_string())
                    .collect::<Vec<_>>()
                    .join("|")
                    .bytes()
                    .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
                diff_cache::DiffCacheKey::Directory {
                    path: path.clone(),
                    files_hash: hash,
                }
            }
            DiffTarget::Commit { oid, path } => diff_cache::DiffCacheKey::Commit {
                oid: oid.clone(),
                path: path.clone(),
            },
            DiffTarget::Stash { index, path } => diff_cache::DiffCacheKey::Stash {
                index: *index,
                path: path.clone(),
            },
            DiffTarget::None => diff_cache::DiffCacheKey::None,
        }
    }

    pub(super) fn diff_cache_key_to_task_target(key: &diff_cache::DiffCacheKey) -> String {
        match key {
            diff_cache::DiffCacheKey::None => "none".to_string(),
            diff_cache::DiffCacheKey::File { path, is_staged } => {
                format!("file:{}:{}", path.display(), is_staged)
            }
            diff_cache::DiffCacheKey::Branch { name, limit } => {
                format!("branch:{}:{}", name, limit)
            }
            diff_cache::DiffCacheKey::Directory { path, files_hash } => {
                format!("dir:{}:{}", path.display(), files_hash)
            }
            diff_cache::DiffCacheKey::Commit { oid, path } => match path {
                Some(path) => format!("commit:{}:{}", oid, path.display()),
                None => format!("commit:{}:*", oid),
            },
            diff_cache::DiffCacheKey::Stash { index, path } => match path {
                Some(path) => format!("stash:{}:{}", index, path.display()),
                None => format!("stash:{}:*", index),
            },
        }
    }

    pub fn commit_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.ui.active_panel != SidePanel::Commits {
            return Ok(());
        }
        if self.ui.commits.dirty {
            let _ = self.start_commits_load(self.tasks.commits_requested_limit);
            return Ok(());
        }

        if !self.ui.commits.tree_mode.active {
            let Some(oid) = self.selected_commit_oid() else {
                return Ok(());
            };
            let files = self.repo.commit_files(&oid)?;
            let event = crate::flux::commits_backend::CommitsBackend::open_tree(
                self.current_commits_view_state(),
                oid,
                files,
            );
            self.apply_commits_backend_view(event);
            return Ok(());
        }

        let event = crate::flux::commits_backend::CommitsBackend::toggle_tree_dir(
            self.current_commits_view_state(),
        );
        self.apply_commits_backend_view(event);
        Ok(())
    }

    pub(super) fn global_key_or(&self, action: &str, fallback: &str) -> String {
        self.keymap
            .first_global_key(action)
            .unwrap_or_else(|| fallback.to_string())
    }

    pub(super) fn panel_key_or(&self, panel: &str, action: &str, fallback: &str) -> String {
        self.keymap
            .first_panel_key(panel, action)
            .unwrap_or_else(|| fallback.to_string())
    }

    pub(super) fn max_refresh_kind(a: RefreshKind, b: RefreshKind) -> RefreshKind {
        use RefreshKind::*;
        match (a, b) {
            (Full, _) | (_, Full) => Full,
            (StatusAndRefs, _) | (_, StatusAndRefs) => StatusAndRefs,
            _ => StatusOnly,
        }
    }

    pub(super) fn reload_commits_now(&mut self) {
        let items = self
            .repo
            .commits(self.tasks.commits_requested_limit)
            .unwrap_or_default();
        if let Err(err) = self.apply_commits_backend_command(
            crate::flux::commits_backend::CommitsBackendCommand::ApplyLoaded {
                items,
                mode: crate::flux::commits_backend::CommitsLoadMode::Full,
            },
        ) {
            self.push_log(format!("commits reload apply failed: {}", err), false);
        }
        if self.ui.commits.tree_mode.active {
            if let Some(ref oid) = self.ui.commits.tree_mode.selected_source {
                if self.ui.commits.items.iter().any(|c| c.oid == *oid) {
                    self.ui.commits.tree_mode.files =
                        self.repo.commit_files(oid).unwrap_or_default();
                    revision_tree::rebuild_tree_nodes(
                        &self.ui.commits.tree_mode.files,
                        &self.ui.commits.tree_mode.expanded_dirs,
                        &mut self.ui.commits.tree_mode.nodes,
                        &mut self.ui.commits.panel.list_state,
                    );
                } else {
                    let event = crate::flux::commits_backend::CommitsBackend::close_tree(
                        self.current_commits_view_state(),
                    );
                    self.apply_commits_backend_view(event);
                }
            } else {
                let event = crate::flux::commits_backend::CommitsBackend::close_tree(
                    self.current_commits_view_state(),
                );
                self.apply_commits_backend_view(event);
            }
        }
    }

    pub(crate) fn refresh_render_cache(&mut self) {
        self.ui.render_cache.files_visual_selected_indices = self.visual_selected_indices();
        self.ui.render_cache.files_search_summary =
            self.search_match_summary_for(SidePanel::Files, false, false);
        self.ui.render_cache.branches_search_summary =
            self.search_match_summary_for(SidePanel::LocalBranches, false, false);
        self.ui.render_cache.commits_search_summary = self.search_match_summary_for(
            SidePanel::Commits,
            self.ui.commits.tree_mode.active,
            false,
        );
        self.ui.render_cache.stash_search_summary =
            self.search_match_summary_for(SidePanel::Stash, false, self.ui.stash.tree_mode.active);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::stores::test_support::MockRepo;
    use pretty_assertions::assert_eq;

    fn mock_app() -> App {
        App::from_repo(Box::new(MockRepo)).expect("app")
    }

    #[test]
    fn test_from_repo_creates_app() {
        let app = mock_app();
        assert!(app.running);
        assert!(app.git.current_diff.is_empty());
    }

    #[test]
    fn test_push_log_adds_entry() {
        let mut app = mock_app();
        let before = app.command_log.len();
        app.push_log("test", true);
        assert_eq!(app.command_log.len(), before + 1);
        assert_eq!(app.command_log.last().unwrap().command, "test");
        assert!(app.command_log.last().unwrap().success);
    }

    #[test]
    fn test_stage_file_calls_repo() {
        let mut app = mock_app();
        let result = app.stage_file("foo.txt".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_unstage_file_calls_repo() {
        let mut app = mock_app();
        let result = app.unstage_file("foo.txt".into());
        assert!(result.is_ok());
    }

    #[test]
    fn test_discard_paths_calls_repo() {
        let mut app = mock_app();
        let result = app.discard_paths(&["foo.txt".into()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit_calls_repo() {
        let mut app = mock_app();
        let result = app.commit("fix: test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_branch_calls_repo() {
        let mut app = mock_app();
        let result = app.create_branch("feature");
        assert!(result.is_ok());
    }

    #[test]
    fn test_checkout_branch_calls_repo() {
        let mut app = mock_app();
        let result = app.checkout_branch("main");
        assert!(result.is_ok());
    }

    #[test]
    fn test_delete_branch_calls_repo() {
        let mut app = mock_app();
        let result = app.delete_branch("old-branch");
        assert!(result.is_ok());
    }

    #[test]
    fn test_stash_apply_calls_repo() {
        let mut app = mock_app();
        let result = app.stash_apply(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stash_pop_calls_repo() {
        let mut app = mock_app();
        let result = app.stash_pop(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_stash_drop_calls_repo() {
        let mut app = mock_app();
        let result = app.stash_drop(0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_has_uncommitted_changes_false_when_empty() {
        let app = mock_app();
        assert!(!app.has_uncommitted_changes());
    }

    #[test]
    fn test_diff_scroll_up_and_down() {
        let mut app = mock_app();
        // diff_scroll_down is capped at current_diff.len()-1; with empty diff stays at 0
        app.diff_scroll_down();
        assert_eq!(app.ui.diff_scroll, 0);
        // With some diff content, scroll can increase
        app.git.current_diff = vec![
            crate::git::DiffLine {
                kind: crate::git::DiffLineKind::Added,
                content: "line".to_string(),
            };
            20
        ];
        app.diff_scroll_down();
        assert!(app.ui.diff_scroll > 0);
        app.diff_scroll_up();
        assert_eq!(app.ui.diff_scroll, 0);
    }

    #[test]
    fn test_toggle_visual_select_mode() {
        let mut app = mock_app();
        assert!(!app.ui.files.visual_mode);
        app.toggle_visual_select_mode();
        assert!(app.ui.files.visual_mode);
        app.toggle_visual_select_mode();
        assert!(!app.ui.files.visual_mode);
    }

    #[test]
    fn test_schedule_diff_reload_marks_pending() {
        let mut app = mock_app();
        assert!(!app.has_pending_diff_reload());
        app.schedule_diff_reload();
        assert!(app.has_pending_diff_reload());
    }

    #[test]
    fn test_request_refresh_sets_pending() {
        let mut app = mock_app();
        assert!(app.pending_refresh_kind().is_none());
        app.request_refresh(RefreshKind::StatusOnly);
        assert_eq!(app.pending_refresh_kind(), Some(RefreshKind::StatusOnly));
    }

    #[test]
    fn test_flush_pending_refresh_clears_pending() {
        let mut app = mock_app();
        app.request_refresh(RefreshKind::StatusOnly);
        let result = app.flush_pending_refresh();
        assert!(result.is_ok());
        assert!(result.unwrap());
        assert!(app.pending_refresh_kind().is_none());
    }

    #[test]
    fn test_start_branch_switch_confirm() {
        let mut app = mock_app();
        app.start_branch_switch_confirm("feature".to_string());
        assert_eq!(app.input.branch_switch_target, Some("feature".to_string()));
        assert_eq!(app.input.mode, Some(InputMode::BranchSwitchConfirm));
    }

    #[test]
    fn test_take_branch_switch_target() {
        let mut app = mock_app();
        app.input.branch_switch_target = Some("feature".to_string());
        let target = app.take_branch_switch_target();
        assert_eq!(target, Some("feature".to_string()));
        assert!(app.input.branch_switch_target.is_none());
    }
}
