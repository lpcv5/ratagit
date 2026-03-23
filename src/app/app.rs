use super::{diff_cache, diff_loader, dirty_flags, refresh, revision_tree};
use crate::config::keymap::Keymap;
use crate::flux::task_manager::{
    TaskGeneration, TaskKey, TaskManager, TaskPriority, TaskRequest, TaskRequestKind,
    TaskResult, TaskResultKind,
};
use crate::git::{
    BranchInfo, CommitInfo, DiffLine, FileEntry, Git2Repository, GitError, GitRepository,
    GitStatus, StashInfo,
};
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode};
use color_eyre::Result;
use ratatui::widgets::ListState;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::time::Instant;
use tracing::debug;

/// Documentation comment in English.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SidePanel {
    #[default]
    Files,
    LocalBranches,
    Commits,
    Stash,
}

/// Documentation comment in English.
#[derive(Clone)]
pub struct PanelState {
    pub list_state: ListState,
}

impl PanelState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }
}

impl Default for PanelState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone)]
pub struct TreeModeState<T> {
    pub active: bool,
    pub nodes: Vec<FileTreeNode>,
    pub files: Vec<FileEntry>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub selected_source: Option<T>,
}

impl<T> Default for TreeModeState<T> {
    fn default() -> Self {
        Self {
            active: false,
            nodes: Vec::new(),
            files: Vec::new(),
            expanded_dirs: HashSet::new(),
            selected_source: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct FilesPanelState {
    pub panel: PanelState,
    pub tree_nodes: Vec<FileTreeNode>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,
}

#[derive(Default, Clone)]
pub struct BranchesPanelState {
    pub panel: PanelState,
    pub items: Vec<BranchInfo>,
    pub is_fetching_remote: bool,
    pub commits_subview_active: bool,
    pub commits_subview_loading: bool,
    pub commits_subview_source: Option<String>,
    pub commits_subview: CommitsPanelState,
}

enum BackgroundReceiver {
    Status {
        fast: bool,
        rx: Receiver<Result<GitStatus, GitError>>,
    },
    Branches(Receiver<Result<Vec<BranchInfo>, GitError>>),
    Stashes(Receiver<Result<Vec<StashInfo>, GitError>>),
    Commits(Receiver<Result<Vec<CommitInfo>, GitError>>),
    CommitsFast(Receiver<Result<Vec<CommitInfo>, GitError>>),
    BranchCommits {
        branch: String,
        rx: Receiver<Result<Vec<CommitInfo>, GitError>>,
    },
    Diff {
        cache_key: diff_cache::DiffCacheKey,
        rx: Receiver<Result<Vec<DiffLine>, GitError>>,
    },
}

enum BackgroundPayload {
    Status {
        status: GitStatus,
        fast: bool,
    },
    Branches(Vec<BranchInfo>),
    Stashes(Vec<StashInfo>),
    Commits(Vec<CommitInfo>),
    CommitsFast(Vec<CommitInfo>),
    BranchCommits {
        branch: String,
        items: Vec<CommitInfo>,
    },
    Diff {
        cache_key: diff_cache::DiffCacheKey,
        diff: Vec<DiffLine>,
    },
}

struct PendingBackgroundTask {
    request: TaskRequest,
    receiver: BackgroundReceiver,
}

const MAX_NORMAL_PRIORITY_TASKS: usize = 6;
const INITIAL_COMMITS_LOAD_LIMIT: usize = 30;
const COMMITS_LOAD_STEP: usize = 40;
const COMMITS_LOAD_AHEAD_THRESHOLD: usize = 8;
const BRANCH_LOG_DIFF_LIMIT: usize = 20;

#[derive(Default, Clone)]
pub struct CommitsPanelState {
    pub panel: PanelState,
    pub items: Vec<CommitInfo>,
    pub dirty: bool,
    pub tree_mode: TreeModeState<String>,
    pub highlighted_oids: HashSet<String>,
}

#[derive(Default, Clone)]
pub struct StashPanelState {
    pub panel: PanelState,
    pub items: Vec<StashInfo>,
    pub tree_mode: TreeModeState<usize>,
}

/// Documentation comment in English.
#[derive(Clone)]
pub struct CommandLogEntry {
    pub command: String,
    pub success: bool,
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiffRefreshSource {
    Status,
    Branches,
    Stashes,
    Commits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SearchScopeKey {
    pub panel: SidePanel,
    pub commit_tree_mode: bool,
    pub stash_tree_mode: bool,
}

#[derive(Default, Clone)]
pub struct RenderCache {
    pub files_visual_selected_indices: HashSet<usize>,
    pub files_search_summary: Option<String>,
    pub branches_search_summary: Option<String>,
    pub commits_search_summary: Option<String>,
    pub stash_search_summary: Option<String>,
}

/// Documentation comment in English.
pub struct App {
    pub running: bool,
    pub active_panel: SidePanel,

    repo: Box<dyn GitRepository>,
    pub status: GitStatus,

    pub files: FilesPanelState,
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,

    pub command_log: Vec<CommandLogEntry>,
    pub current_diff: Vec<DiffLine>,
    /// Documentation comment in English.
    pub diff_scroll: usize,
    pub input_mode: Option<InputMode>,
    pub input_buffer: String,
    pub commit_message_buffer: String,
    pub commit_description_buffer: String,
    pub commit_focus: CommitFieldFocus,
    pub stash_message_buffer: String,
    pub stash_targets: Vec<PathBuf>,
    pub branch_switch_target: Option<String>,
    pub search_query: String,
    pub(super) search_matches: Vec<usize>,
    pub(super) search_scope: SearchScopeKey,
    pub(super) search_queries: HashMap<SearchScopeKey, String>,
    pending_refresh: Option<RefreshKind>,
    pending_diff_reload: bool,
    pending_diff_reload_at: Option<Instant>,
    task_manager: TaskManager,
    pending_background_tasks: HashMap<TaskGeneration, PendingBackgroundTask>,
    pending_full_status_after_fast: bool,
    pending_refresh_fast_done: bool,
    commits_requested_limit: usize,

    diff_cache: diff_cache::DiffCache,
    last_diff_key: Option<diff_cache::DiffCacheKey>,
    in_flight_diff_key: Option<diff_cache::DiffCacheKey>,
    pub dirty: dirty_flags::DirtyFlags,
    pub render_cache: RenderCache,

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
            refresh::collect_all_dirs(&status)
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
            active_panel: SidePanel::Files,
            repo,
            status,
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
            command_log: Vec::new(),
            current_diff: Vec::new(),
            diff_scroll: 0,
            input_mode: None,
            input_buffer: String::new(),
            commit_message_buffer: String::new(),
            commit_description_buffer: String::new(),
            commit_focus: CommitFieldFocus::Message,
            stash_message_buffer: String::new(),
            stash_targets: Vec::new(),
            branch_switch_target: None,
            search_query: String::new(),
            search_matches: Vec::new(),
            search_scope: SearchScopeKey {
                panel: SidePanel::Files,
                commit_tree_mode: false,
                stash_tree_mode: false,
            },
            search_queries: HashMap::new(),
            pending_refresh: None,
            pending_diff_reload: false,
            pending_diff_reload_at: None,
            task_manager: TaskManager::new(),
            pending_background_tasks: HashMap::new(),
            pending_full_status_after_fast: !preload_commits_and_diff,
            pending_refresh_fast_done: false,
            commits_requested_limit: INITIAL_COMMITS_LOAD_LIMIT,
            diff_cache: diff_cache::DiffCache::new(),
            last_diff_key: None,
            in_flight_diff_key: None,
            dirty: dirty_flags::DirtyFlags::default(),
            render_cache: RenderCache::default(),
            keymap,
        };
        app.refresh_render_cache();
        app.dirty.mark_all();
        if preload_commits_and_diff {
            app.reload_diff_now();
        } else {
            app.request_refresh(RefreshKind::StatusAndRefs);
            app.schedule_diff_reload();
            // Load commits immediately on startup
            let _ = app.start_commits_load(app.commits_requested_limit);
        }
        Ok(app)
    }

    pub(crate) fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    fn apply_refresh(&mut self, kind: RefreshKind) -> Result<()> {
        self.status = self.repo.status()?;
        let new_dirs = refresh::collect_all_dirs(&self.status);
        for d in new_dirs {
            self.files.expanded_dirs.insert(d);
        }
        self.rebuild_tree();

        if matches!(kind, RefreshKind::StatusAndRefs | RefreshKind::Full) {
            self.branches.items = self.repo.branches().unwrap_or_default();
            self.stash.items = self.repo.stashes().unwrap_or_default();
            if self.stash.tree_mode.active {
                if let Some(index) = self.stash.tree_mode.selected_source {
                    if self.stash.items.iter().any(|s| s.index == index) {
                        self.stash.tree_mode.files =
                            self.repo.stash_files(index).unwrap_or_default();
                        revision_tree::rebuild_tree_nodes(
                            &self.stash.tree_mode.files,
                            &self.stash.tree_mode.expanded_dirs,
                            &mut self.stash.tree_mode.nodes,
                            &mut self.stash.panel.list_state,
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
        self.pending_full_status_after_fast = true;
        self.pending_refresh_fast_done = false;
        if matches!(kind, RefreshKind::Full) {
            self.commits.dirty = true;
        }
        self.diff_cache.invalidate_files();
        if matches!(
            self.last_diff_key,
            Some(diff_cache::DiffCacheKey::File { .. })
                | Some(diff_cache::DiffCacheKey::Directory { .. })
        ) {
            self.last_diff_key = None;
        }
        self.pending_refresh = Some(match self.pending_refresh {
            None => kind,
            Some(existing) => Self::max_refresh_kind(existing, kind),
        });
    }

    pub fn flush_pending_refresh(&mut self) -> Result<bool> {
        let Some(kind) = self.pending_refresh.take() else {
            return Ok(false);
        };
        self.apply_refresh(kind)?;
        self.schedule_diff_reload();
        self.dirty.mark_all();
        Ok(true)
    }

    fn has_background_task(&self, key: &TaskKey) -> bool {
        self.pending_background_tasks
            .values()
            .any(|task| &task.request.key == key)
    }

    fn can_start_background_task(&self, priority: TaskPriority) -> bool {
        match priority {
            TaskPriority::High => true,
            TaskPriority::Normal | TaskPriority::Low => {
                self.pending_background_tasks.len() < MAX_NORMAL_PRIORITY_TASKS
            }
        }
    }

    fn diff_task_key() -> TaskKey {
        TaskKey::Diff {
            target: "active".to_string(),
        }
    }

    fn has_active_diff_task(&self) -> bool {
        let key = Self::diff_task_key();
        self.has_background_task(&key)
    }

    fn cancel_pending_diff_task(&mut self) {
        let key = Self::diff_task_key();
        let _ = self.task_manager.cancel(&key);
        self.pending_background_tasks
            .retain(|_, task| task.request.key != key);
        self.in_flight_diff_key = None;
    }

    fn start_status_load(&mut self, fast: bool) -> bool {
        let key = TaskKey::Status;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::High;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request = self
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
                self.task_manager.mark_started(&key, request.generation);
                self.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Status { fast, rx },
                    },
                );
                true
            }
            Err(err) => {
                self.task_manager.mark_finished(&key, request.generation);
                self.push_log(format!("status load failed: {}", err), false);
                true
            }
        }
    }

    fn start_branches_load(&mut self) -> bool {
        let key = TaskKey::Branches;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request =
            self.task_manager
                .enqueue(key.clone(), priority, TaskRequestKind::LoadBranches);
        match self.repo.branches_async() {
            Ok(rx) => {
                self.task_manager.mark_started(&key, request.generation);
                self.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Branches(rx),
                    },
                );
                true
            }
            Err(err) => {
                self.task_manager.mark_finished(&key, request.generation);
                self.push_log(format!("branches load failed: {}", err), false);
                true
            }
        }
    }

    fn start_stashes_load(&mut self) -> bool {
        let key = TaskKey::Stashes;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request =
            self.task_manager
                .enqueue(key.clone(), priority, TaskRequestKind::LoadStashes);
        match self.repo.stashes_async() {
            Ok(rx) => {
                self.task_manager.mark_started(&key, request.generation);
                self.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Stashes(rx),
                    },
                );
                true
            }
            Err(err) => {
                self.task_manager.mark_finished(&key, request.generation);
                self.push_log(format!("stashes load failed: {}", err), false);
                true
            }
        }
    }

    fn start_commits_load(&mut self, limit: usize) -> bool {
        let key = TaskKey::Commits;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::Normal;
        if !self.can_start_background_task(priority) {
            return false;
        }
        let request = self.task_manager.enqueue(
            key.clone(),
            priority,
            TaskRequestKind::LoadCommits { limit },
        );
        self.commits_requested_limit = self.commits_requested_limit.max(limit);
        let fast = self.commits.items.is_empty();
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
                self.task_manager.mark_started(&key, request.generation);
                self.pending_background_tasks.insert(
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
                self.task_manager.mark_finished(&key, request.generation);
                self.push_log(format!("commits load failed: {}", err), false);
                true
            }
        }
    }

    fn apply_status_refresh(&mut self, status: GitStatus) {
        self.status = status;
        let new_dirs = refresh::collect_all_dirs(&self.status);
        for d in new_dirs {
            self.files.expanded_dirs.insert(d);
        }
        self.rebuild_tree();
    }

    fn should_schedule_diff_for_refresh(&self, source: DiffRefreshSource) -> bool {
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
        if let Some(kind) = self.pending_refresh.take() {
            let mut deferred_kind = None;
            if self.has_background_task(&TaskKey::Status) {
                deferred_kind = Some(kind);
            } else if !self.pending_refresh_fast_done {
                if self.start_status_load(true) {
                    self.pending_refresh_fast_done = true;
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
                    self.commits.dirty = true;
                }
                if deferred_kind.is_none() {
                    self.pending_refresh_fast_done = false;
                }
            }
            if let Some(kind) = deferred_kind {
                self.pending_refresh = Some(match self.pending_refresh {
                    None => kind,
                    Some(existing) => Self::max_refresh_kind(existing, kind),
                });
            }
        }

        if self.active_panel == SidePanel::Commits && self.commits.dirty {
            let _ = self.start_commits_load(self.commits_requested_limit);
        }
        self.maybe_schedule_commits_extend();

        self.start_pending_diff_load();

        let mut schedule_diff = false;

        let mut next_pending_background = HashMap::new();
        let mut finished_payloads: HashMap<(TaskKey, TaskGeneration), BackgroundPayload> =
            HashMap::new();
        for (generation, task) in self.pending_background_tasks.drain() {
            let PendingBackgroundTask { request, receiver } = task;
            match receiver {
                BackgroundReceiver::Status { fast, rx } => match rx.try_recv() {
                    Ok(Ok(status)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Status { status, fast },
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("status load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Status { fast, rx },
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "status load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Branches(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Branches(items),
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("branches load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Branches(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "branches load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Stashes(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Stashes(items),
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("stashes load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Stashes(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "stashes load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Commits(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Commits(items),
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("commits load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Commits(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "commits load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::CommitsFast(rx) => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::CommitsFast(items),
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("commits load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::CommitsFast(rx),
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "commits load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::BranchCommits { branch, rx } => match rx.try_recv() {
                    Ok(Ok(items)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::BranchCommits { branch, items },
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("branch commits load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::BranchCommits { branch, rx },
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "branch commits load disconnected".to_string(),
                            },
                        });
                    }
                },
                BackgroundReceiver::Diff { cache_key, rx } => match rx.try_recv() {
                    Ok(Ok(diff)) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Finished,
                        });
                        finished_payloads.insert(
                            (request.key.clone(), request.generation),
                            BackgroundPayload::Diff { cache_key, diff },
                        );
                    }
                    Ok(Err(err)) => self.task_manager.submit_result(TaskResult {
                        key: request.key.clone(),
                        generation: request.generation,
                        kind: TaskResultKind::Failed {
                            reason: format!("diff load failed: {}", err),
                        },
                    }),
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        next_pending_background.insert(
                            generation,
                            PendingBackgroundTask {
                                request,
                                receiver: BackgroundReceiver::Diff { cache_key, rx },
                            },
                        );
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.task_manager.submit_result(TaskResult {
                            key: request.key.clone(),
                            generation: request.generation,
                            kind: TaskResultKind::Failed {
                                reason: "diff load disconnected".to_string(),
                            },
                        });
                    }
                },
            }
        }
        self.pending_background_tasks = next_pending_background;

        for task in self.task_manager.collect_ready() {
            match task.kind {
                TaskResultKind::Finished => {
                    let Some(payload) = finished_payloads.remove(&(task.key, task.generation))
                    else {
                        continue;
                    };
                    match payload {
                        BackgroundPayload::Status { status, fast } => {
                            debug!(
                                event = "status_load_finished",
                                mode = if fast { "fast" } else { "full" },
                                staged = status.staged.len(),
                                unstaged = status.unstaged.len(),
                                untracked = status.untracked.len(),
                                "status load finished"
                            );
                            self.apply_status_refresh(status);
                            if fast && self.pending_full_status_after_fast {
                                if self.start_status_load(false) {
                                    self.pending_full_status_after_fast = false;
                                } else {
                                    self.pending_refresh = Some(match self.pending_refresh {
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
                            self.dirty.mark_all();
                        }
                        BackgroundPayload::Branches(items) => {
                            self.branches.items = items;
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Branches) {
                                schedule_diff = true;
                            }
                            self.dirty.mark_all();
                        }
                        BackgroundPayload::Stashes(items) => {
                            self.stash.items = items;
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Stashes) {
                                schedule_diff = true;
                            }
                            self.dirty.mark_all();
                        }
                        BackgroundPayload::Commits(items) => {
                            debug!(
                                event = "commits_load_finished",
                                count = items.len(),
                                requested_limit = self.commits_requested_limit,
                                "commits load finished"
                            );
                            self.commits.items = items;
                            self.commits.dirty = false;
                            self.dirty.mark_all();
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits) {
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::CommitsFast(items) => {
                            debug!(
                                event = "commits_load_finished",
                                mode = "fast",
                                count = items.len(),
                                requested_limit = self.commits_requested_limit,
                                "commits fast load finished"
                            );
                            self.commits.items = items;
                            self.commits.dirty = true;
                            self.dirty.mark_all();
                            if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits) {
                                schedule_diff = true;
                            }
                        }
                        BackgroundPayload::BranchCommits { branch, items } => {
                            if self.branches.commits_subview_source.as_deref()
                                == Some(branch.as_str())
                                && self.branches.commits_subview_active
                            {
                                self.branches.commits_subview.items = items;
                                self.branches.commits_subview_loading = false;
                                self.branches.commits_subview.panel.list_state.select(
                                    (!self.branches.commits_subview.items.is_empty()).then_some(0),
                                );
                                self.dirty.mark_all();
                                if self.should_schedule_diff_for_refresh(DiffRefreshSource::Commits)
                                {
                                    schedule_diff = true;
                                }
                            }
                        }
                        BackgroundPayload::Diff { cache_key, diff } => {
                            self.in_flight_diff_key = None;
                            self.diff_cache.insert(cache_key.clone(), diff.clone());
                            self.last_diff_key = Some(cache_key);
                            self.current_diff = diff;
                            if !self.pending_diff_reload {
                                self.pending_diff_reload_at = None;
                            }
                            self.dirty.mark_diff();
                        }
                    }
                }
                TaskResultKind::Failed { reason } => {
                    if matches!(task.key, TaskKey::Status) {
                        self.pending_refresh_fast_done = false;
                    }
                    if matches!(task.key, TaskKey::BranchCommits { .. }) {
                        self.branches.commits_subview_loading = false;
                    }
                    if matches!(task.key, TaskKey::Diff { .. }) {
                        self.in_flight_diff_key = None;
                    }
                    if matches!(task.key, TaskKey::Diff { .. }) && !self.pending_diff_reload {
                        self.pending_diff_reload_at = None;
                    }
                    self.push_log(reason, false);
                }
            }
        }

        if schedule_diff {
            self.schedule_diff_reload();
        }
    }

    pub fn schedule_diff_reload(&mut self) {
        self.pending_diff_reload = true;
        self.pending_diff_reload_at = Some(Instant::now());
        debug!(event = "schedule_diff_reload", "scheduled diff reload");
    }

    pub fn has_pending_diff_reload(&self) -> bool {
        self.pending_diff_reload
    }

    pub fn has_pending_refresh_work(&self) -> bool {
        self.pending_refresh.is_some() || !self.pending_background_tasks.is_empty()
    }

    pub fn pending_background_task_count(&self) -> usize {
        self.pending_background_tasks.len()
    }

    pub fn diff_reload_debounce_elapsed(&self, debounce: std::time::Duration) -> bool {
        self.pending_diff_reload_at
            .is_some_and(|requested_at| requested_at.elapsed() >= debounce)
    }

    pub fn flush_pending_diff_reload(&mut self) {
        if !self.pending_diff_reload && !self.has_active_diff_task() {
            return;
        }
        self.start_pending_diff_load();
    }

    pub(super) fn pending_refresh_kind(&self) -> Option<RefreshKind> {
        self.pending_refresh
    }

    fn maybe_schedule_commits_extend(&mut self) {
        if self.active_panel != SidePanel::Commits || self.commits.tree_mode.active {
            return;
        }
        if self.has_background_task(&TaskKey::Commits) || self.commits.dirty {
            return;
        }
        let len = self.commits.items.len();
        if len == 0 {
            return;
        }
        let Some(selected) = self.commits.panel.list_state.selected() else {
            return;
        };
        if selected + COMMITS_LOAD_AHEAD_THRESHOLD < len {
            return;
        }
        self.commits_requested_limit = self
            .commits_requested_limit
            .saturating_add(COMMITS_LOAD_STEP);
        self.commits.dirty = true;
        debug!(
            event = "commits_extend_requested",
            selected = selected,
            current = len,
            next_limit = self.commits_requested_limit,
            "scheduled commits extension"
        );
    }

    pub fn ensure_commits_loaded_for_active_panel(&mut self) {
        if self.active_panel == SidePanel::Commits && self.commits.dirty {
            let _ = self.start_commits_load(self.commits_requested_limit);
        }
    }

    fn clear_pending_diff_reload(&mut self) {
        self.pending_diff_reload = false;
        self.pending_diff_reload_at = None;
    }

    fn start_pending_diff_load(&mut self) {
        if !self.pending_diff_reload {
            return;
        }
        let target = self.selected_diff_target();
        let key = self.diff_target_to_cache_key(&target);

        if self.in_flight_diff_key.as_ref() == Some(&key) && self.has_active_diff_task() {
            self.clear_pending_diff_reload();
            return;
        }

        if self.last_diff_key.as_ref() == Some(&key) {
            self.cancel_pending_diff_task();
            self.clear_pending_diff_reload();
            return;
        }

        self.diff_scroll = 0;
        if let Some(cached) = self.diff_cache.get_cloned(&key) {
            self.current_diff = cached;
            self.last_diff_key = Some(key);
            self.clear_pending_diff_reload();
            self.dirty.mark_diff();
            return;
        }

        self.cancel_pending_diff_task();

        let key_for_task = Self::diff_task_key();
        let target_label = Self::diff_cache_key_to_task_target(&key);
        let request = self.task_manager.enqueue(
            key_for_task.clone(),
            TaskPriority::High,
            TaskRequestKind::LoadDiff {
                target: target_label,
            },
        );
        let request_generation = request.generation.0;

        let rx = match target {
            diff_loader::DiffTarget::None => {
                self.current_diff = Vec::new();
                self.last_diff_key = Some(key);
                self.clear_pending_diff_reload();
                self.dirty.mark_diff();
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
                        self.diff_cache.insert(key.clone(), diff.clone());
                        self.last_diff_key = Some(key);
                        self.current_diff = diff;
                        self.clear_pending_diff_reload();
                        self.dirty.mark_diff();
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
                self.task_manager
                    .mark_started(&key_for_task, request.generation);
                self.pending_background_tasks.insert(
                    request.generation,
                    PendingBackgroundTask {
                        request,
                        receiver: BackgroundReceiver::Diff {
                            cache_key: key.clone(),
                            rx,
                        },
                    },
                );
                self.in_flight_diff_key = Some(key);
                self.clear_pending_diff_reload();
                debug!(
                    event = "start_diff_load",
                    generation = request_generation,
                    target = ?self.in_flight_diff_key,
                    "scheduled diff load"
                );
            }
            Err(err) => {
                self.task_manager
                    .mark_finished(&key_for_task, request.generation);
                self.push_log(format!("diff load failed: {}", err), false);
                self.clear_pending_diff_reload();
            }
        }
    }

    pub fn open_selected_branch_commits(&mut self, limit: usize) -> Result<()> {
        if self.active_panel != SidePanel::LocalBranches {
            return Ok(());
        }
        let Some(branch_name) = self.selected_branch_name() else {
            return Ok(());
        };

        self.branches.commits_subview_active = true;
        self.branches.commits_subview_loading = true;
        self.branches.commits_subview_source = Some(branch_name.clone());
        self.branches.commits_subview.items.clear();
        self.branches.commits_subview.dirty = false;
        self.branches.commits_subview.highlighted_oids.clear();
        self.branches.commits_subview.tree_mode = TreeModeState::default();
        self.branches.commits_subview.panel.list_state.select(None);
        let key = TaskKey::BranchCommits {
            branch: branch_name.clone(),
        };
        let request = self.task_manager.enqueue(
            key.clone(),
            TaskPriority::High,
            TaskRequestKind::LoadBranchCommits {
                branch: branch_name.clone(),
                limit,
            },
        );
        match self.repo.commits_for_branch_async(&branch_name, limit) {
            Ok(rx) => {
                self.task_manager.mark_started(&key, request.generation);
                self.pending_background_tasks.insert(
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
                self.task_manager.mark_finished(&key, request.generation);
                self.branches.commits_subview_loading = false;
                self.push_log(format!("branch commits load failed: {}", err), false);
            }
        }
        self.push_log(
            format!("branch commits: {} (Esc to back)", branch_name),
            true,
        );
        Ok(())
    }

    pub fn close_branch_commits_subview(&mut self) {
        if !self.branches.commits_subview_active {
            return;
        }

        let source_branch = self.branches.commits_subview_source.take();
        self.branches.commits_subview_active = false;
        self.branches.commits_subview_loading = false;
        self.branches.commits_subview = CommitsPanelState::default();

        let selected_index = source_branch.and_then(|name| {
            self.branches
                .items
                .iter()
                .position(|branch| branch.name == name)
        });
        if self.branches.items.is_empty() {
            self.branches.panel.list_state.select(None);
        } else {
            self.branches
                .panel
                .list_state
                .select(selected_index.or(Some(0)));
        }
    }

    pub fn toggle_visual_select_mode(&mut self) {
        if self.active_panel != SidePanel::Files {
            return;
        }
        if self.files.visual_mode {
            self.files.visual_mode = false;
            self.files.visual_anchor = None;
            return;
        }

        self.files.visual_mode = true;
        self.files.visual_anchor = self.files.panel.list_state.selected();
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
        self.dirty.mark_command_log();
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

    pub fn checkout_branch(&mut self, name: &str) -> Result<()> {
        self.repo.checkout_branch(name)?;
        self.request_refresh(RefreshKind::Full);
        Ok(())
    }

    pub fn has_uncommitted_changes(&self) -> bool {
        !self.status.staged.is_empty()
            || !self.status.unstaged.is_empty()
            || !self.status.untracked.is_empty()
    }

    pub fn start_branch_switch_confirm(&mut self, target: String) {
        self.input_mode = Some(InputMode::BranchSwitchConfirm);
        self.branch_switch_target = Some(target);
    }

    pub fn pending_branch_switch_target(&self) -> Option<&str> {
        self.branch_switch_target.as_deref()
    }

    pub fn take_branch_switch_target(&mut self) -> Option<String> {
        self.branch_switch_target.take()
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
    pub fn toggle_selected_dir(&mut self) {
        let selected_dir_path = self.selected_tree_node().and_then(|node| {
            if node.is_dir {
                Some(node.path.clone())
            } else {
                None
            }
        });
        refresh::toggle_selected_dir(&mut self.files.expanded_dirs, selected_dir_path);
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn collapse_all(&mut self) {
        refresh::collapse_all(&mut self.files.expanded_dirs);
        self.rebuild_tree();
    }

    /// Documentation comment in English.
    pub fn expand_all(&mut self) {
        refresh::expand_all(&mut self.files.expanded_dirs, &self.status);
        self.rebuild_tree();
    }

    pub fn diff_scroll_up(&mut self) {
        self.diff_scroll = self.diff_scroll.saturating_sub(10);
    }

    pub fn diff_scroll_down(&mut self) {
        let max = self.current_diff.len().saturating_sub(1);
        self.diff_scroll = (self.diff_scroll + 10).min(max);
    }

    fn rebuild_tree(&mut self) {
        refresh::rebuild_tree(
            &self.status,
            &self.files.expanded_dirs,
            &mut self.files.tree_nodes,
            &mut self.files.panel,
            &mut self.files.visual_anchor,
        );
    }

    pub fn stash_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.active_panel != SidePanel::Stash {
            return Ok(());
        }

        if !self.stash.tree_mode.active {
            let Some(index) = self.selected_stash_index() else {
                return Ok(());
            };
            let files = self.repo.stash_files(index)?;
            revision_tree::enter_tree_mode(
                index,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.stash.tree_mode.active,
                    tree_nodes: &mut self.stash.tree_mode.nodes,
                    tree_files: &mut self.stash.tree_mode.files,
                    expanded_dirs: &mut self.stash.tree_mode.expanded_dirs,
                    selected_tree_revision: &mut self.stash.tree_mode.selected_source,
                    list_state: &mut self.stash.panel.list_state,
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
            &self.stash.tree_mode.files,
            &mut self.stash.tree_mode.expanded_dirs,
            &mut self.stash.tree_mode.nodes,
            &mut self.stash.panel.list_state,
        );
        Ok(())
    }

    pub fn stash_close_tree(&mut self) {
        let selected_source_index = self
            .stash
            .tree_mode
            .selected_source
            .and_then(|stash_index| self.stash.items.iter().position(|s| s.index == stash_index));

        let was_open = self.stash.tree_mode.active;
        revision_tree::close_tree_mode(
            &mut self.stash.tree_mode.active,
            &mut self.stash.tree_mode.nodes,
            &mut self.stash.tree_mode.files,
            &mut self.stash.tree_mode.expanded_dirs,
            &mut self.stash.panel.list_state,
            selected_source_index,
            self.stash.items.len(),
        );
        if was_open {
            self.stash.tree_mode.selected_source = None;
        }
    }

    pub fn reload_diff_now(&mut self) {
        self.schedule_diff_reload();
        self.start_pending_diff_load();
    }

    fn diff_target_to_cache_key(
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
                    .files
                    .tree_nodes
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
            DiffTarget::None => diff_cache::DiffCacheKey::File {
                path: PathBuf::new(),
                is_staged: false,
            },
        }
    }

    fn diff_cache_key_to_task_target(key: &diff_cache::DiffCacheKey) -> String {
        match key {
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
        if self.active_panel != SidePanel::Commits {
            return Ok(());
        }
        if self.commits.dirty {
            let _ = self.start_commits_load(self.commits_requested_limit);
            return Ok(());
        }

        if !self.commits.tree_mode.active {
            let Some(oid) = self.selected_commit_oid() else {
                return Ok(());
            };
            let files = self.repo.commit_files(&oid)?;
            revision_tree::enter_tree_mode(
                oid,
                files,
                revision_tree::TreeModeState {
                    tree_mode: &mut self.commits.tree_mode.active,
                    tree_nodes: &mut self.commits.tree_mode.nodes,
                    tree_files: &mut self.commits.tree_mode.files,
                    expanded_dirs: &mut self.commits.tree_mode.expanded_dirs,
                    selected_tree_revision: &mut self.commits.tree_mode.selected_source,
                    list_state: &mut self.commits.panel.list_state,
                },
            );
            return Ok(());
        }

        let selected_dir_path = self.selected_commit_tree_node().and_then(|node| {
            if node.is_dir {
                Some(node.path.clone())
            } else {
                None
            }
        });
        revision_tree::toggle_tree_dir(
            selected_dir_path,
            &self.commits.tree_mode.files,
            &mut self.commits.tree_mode.expanded_dirs,
            &mut self.commits.tree_mode.nodes,
            &mut self.commits.panel.list_state,
        );
        Ok(())
    }

    pub fn commit_close_tree(&mut self) {
        let selected_source_index = self
            .commits
            .tree_mode
            .selected_source
            .as_ref()
            .and_then(|oid| self.commits.items.iter().position(|c| c.oid == *oid));

        let was_open = self.commits.tree_mode.active;
        revision_tree::close_tree_mode(
            &mut self.commits.tree_mode.active,
            &mut self.commits.tree_mode.nodes,
            &mut self.commits.tree_mode.files,
            &mut self.commits.tree_mode.expanded_dirs,
            &mut self.commits.panel.list_state,
            selected_source_index,
            self.commits.items.len(),
        );
        if was_open {
            self.commits.tree_mode.selected_source = None;
        }
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

    fn max_refresh_kind(a: RefreshKind, b: RefreshKind) -> RefreshKind {
        use RefreshKind::*;
        match (a, b) {
            (Full, _) | (_, Full) => Full,
            (StatusAndRefs, _) | (_, StatusAndRefs) => StatusAndRefs,
            _ => StatusOnly,
        }
    }

    fn reload_commits_now(&mut self) {
        self.commits.items = self
            .repo
            .commits(self.commits_requested_limit)
            .unwrap_or_default();
        self.commits.dirty = false;
        if self.commits.tree_mode.active {
            if let Some(ref oid) = self.commits.tree_mode.selected_source {
                if self.commits.items.iter().any(|c| c.oid == *oid) {
                    self.commits.tree_mode.files = self.repo.commit_files(oid).unwrap_or_default();
                    revision_tree::rebuild_tree_nodes(
                        &self.commits.tree_mode.files,
                        &self.commits.tree_mode.expanded_dirs,
                        &mut self.commits.tree_mode.nodes,
                        &mut self.commits.panel.list_state,
                    );
                } else {
                    self.commit_close_tree();
                }
            } else {
                self.commit_close_tree();
            }
        }
    }

    pub(crate) fn refresh_render_cache(&mut self) {
        self.render_cache.files_visual_selected_indices = self.visual_selected_indices();
        self.render_cache.files_search_summary =
            self.search_match_summary_for(SidePanel::Files, false, false);
        self.render_cache.branches_search_summary =
            self.search_match_summary_for(SidePanel::LocalBranches, false, false);
        self.render_cache.commits_search_summary =
            self.search_match_summary_for(SidePanel::Commits, self.commits.tree_mode.active, false);
        self.render_cache.stash_search_summary =
            self.search_match_summary_for(SidePanel::Stash, false, self.stash.tree_mode.active);
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
        assert!(app.current_diff.is_empty());
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
        assert_eq!(app.diff_scroll, 0);
        // With some diff content, scroll can increase
        app.current_diff = vec![
            crate::git::DiffLine {
                kind: crate::git::DiffLineKind::Added,
                content: "line".to_string(),
            };
            20
        ];
        app.diff_scroll_down();
        assert!(app.diff_scroll > 0);
        app.diff_scroll_up();
        assert_eq!(app.diff_scroll, 0);
    }

    #[test]
    fn test_toggle_visual_select_mode() {
        let mut app = mock_app();
        assert!(!app.files.visual_mode);
        app.toggle_visual_select_mode();
        assert!(app.files.visual_mode);
        app.toggle_visual_select_mode();
        assert!(!app.files.visual_mode);
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
        assert_eq!(app.branch_switch_target, Some("feature".to_string()));
        assert_eq!(app.input_mode, Some(InputMode::BranchSwitchConfirm));
    }

    #[test]
    fn test_take_branch_switch_target() {
        let mut app = mock_app();
        app.branch_switch_target = Some("feature".to_string());
        let target = app.take_branch_switch_target();
        assert_eq!(target, Some("feature".to_string()));
        assert!(app.branch_switch_target.is_none());
    }
}
