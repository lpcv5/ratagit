use super::background_task_runner::BackgroundTaskRunner;
use super::refresh_scheduler::RefreshScheduler;
use super::states::{
    BranchesPanelState, CommandLogEntry, CommitsPanelState, GitState, InputState, PanelState,
    RenderCache, SidePanel, StashPanelState, TreeModeState, UiState,
};
use super::{dirty_flags, revision_tree};
use super::{AppRuntime, BackgroundLoad, DiffState, FilesState};
use crate::config::keymap::Keymap;
use crate::flux::effects::EffectRequest;
use crate::flux::task_manager::{TaskKey, TaskPriority, TaskRequestKind};
use crate::git::{Git2Repository, GitRepository, GitStatus};
use color_eyre::Result;
use std::collections::HashSet;
use tracing::debug;

const MAX_NORMAL_PRIORITY_TASKS: usize = 6;
const INITIAL_COMMITS_LOAD_LIMIT: usize = 30;
const COMMITS_LOAD_STEP: usize = 40;
const COMMITS_LOAD_AHEAD_THRESHOLD: usize = 8;

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

pub type SearchScopeKey = super::ViewContext;

/// Reducer-owned state, caches and scheduling metadata. Contains no Git or I/O handles.
pub struct AppState {
    pub running: bool,
    pub git: GitState,
    pub ui: UiState,
    pub input: InputState,
    pub command_log: Vec<CommandLogEntry>,

    pub(super) refresh: RefreshScheduler,
    pub diff: DiffState,
    pub files: FilesState,
    pub(super) tasks: BackgroundTaskRunner,
    keymap: Keymap,
    pub(super) effects: Vec<EffectRequest>,
}

/// Startup composition of reducer state and execution dependencies.
pub struct App {
    pub state: AppState,
    pub runtime: AppRuntime,
}

impl std::ops::Deref for App {
    type Target = AppState;
    fn deref(&self) -> &AppState {
        &self.state
    }
}

impl std::ops::DerefMut for App {
    fn deref_mut(&mut self) -> &mut AppState {
        &mut self.state
    }
}

impl App {
    pub fn new() -> Result<Self> {
        let repo = Git2Repository::discover()?;
        Self::build_with_repo(Box::new(repo), Keymap::load(), false)
    }

    #[cfg(test)]
    pub fn from_repo(repo: Box<dyn GitRepository>) -> Result<Self> {
        Self::from_repo_with_keymap(repo, Keymap::default())
    }

    #[cfg(test)]
    pub fn from_repo_with_keymap(repo: Box<dyn GitRepository>, keymap: Keymap) -> Result<Self> {
        let mut app = Self::build_with_repo(repo, keymap, true)?;
        crate::flux::test_runtime::finish_background(&mut app);
        Ok(app)
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

        let operation = if preload_commits_and_diff {
            crate::git::operation::OperationMachine {
                observed: repo.operation_snapshot()?,
                ..Default::default()
            }
        } else {
            Default::default()
        };
        let mut app = Self {
            state: AppState {
                running: true,
                git: GitState { operation },
                ui: UiState {
                    active_panel: SidePanel::Files,
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
                    dirty: dirty_flags::DirtyFlags::default(),
                    render_cache: RenderCache::default(),
                },
                input: InputState::default(),
                command_log: Vec::new(),
                refresh: RefreshScheduler::new(!preload_commits_and_diff),
                diff: DiffState::default(),
                files: FilesState::new(status, preload_commits_and_diff),
                tasks: BackgroundTaskRunner::new(INITIAL_COMMITS_LOAD_LIMIT),
                keymap,
                effects: Vec::new(),
            },
            runtime: AppRuntime::new(repo),
        };
        app.refresh_render_cache();
        app.ui.dirty.mark_all();
        if preload_commits_and_diff {
            app.reload_diff_now();
        } else {
            app.request_refresh(RefreshKind::StatusAndRefs);
            app.schedule_diff_reload();
            // Load commits immediately on startup
            let _ = app
                .state
                .start_commits_load(app.state.tasks.commits_requested_limit);
        }
        Ok(app)
    }
}

impl AppState {
    pub(super) fn queue_background(
        &mut self,
        request: crate::flux::task_manager::TaskRequest,
        load: BackgroundLoad,
    ) {
        self.tasks
            .task_manager
            .mark_started(&request.key, request.generation);
        self.tasks
            .pending_background_tasks
            .insert(request.generation, request.clone());
        self.effects
            .push(EffectRequest::LoadBackground { request, load });
    }

    pub(crate) fn take_effects(&mut self) -> Vec<EffectRequest> {
        std::mem::take(&mut self.effects)
    }

    pub(crate) fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn request_refresh(&mut self, kind: RefreshKind) {
        self.cancel_pending_diff_task();
        self.refresh.pending_full_status_after_fast = true;
        self.refresh.pending_refresh_fast_done = false;
        if matches!(kind, RefreshKind::Full) {
            self.ui.commits.dirty = true;
        }
        self.diff
            .repository_refreshed(!matches!(kind, RefreshKind::StatusOnly));
        self.refresh.pending_refresh = Some(match self.refresh.pending_refresh {
            None => kind,
            Some(existing) => Self::max_refresh_kind(existing, kind),
        });
    }

    pub(super) fn has_background_task(&self, key: &TaskKey) -> bool {
        self.tasks
            .pending_background_tasks
            .values()
            .any(|task| &task.key == key)
    }

    pub(super) fn can_start_background_task(&self, priority: TaskPriority) -> bool {
        match priority {
            TaskPriority::High => true,
            TaskPriority::Normal | TaskPriority::Low => {
                self.tasks.pending_background_tasks.len() < MAX_NORMAL_PRIORITY_TASKS
            }
        }
    }

    pub(super) fn cancel_background_task(&mut self, key: &TaskKey) {
        let _ = self.tasks.task_manager.cancel(key);
        self.tasks
            .pending_background_tasks
            .retain(|_, task| &task.key != key);
        self.effects.retain(|effect| {
            !matches!(effect,
            EffectRequest::LoadBackground { request, .. } if &request.key == key)
        });
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
        self.queue_background(
            request,
            BackgroundLoad::Status {
                fast,
                operation_epoch: self.git.operation.epoch,
            },
        );
        true
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
        self.queue_background(request, BackgroundLoad::Branches);
        true
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
        self.queue_background(request, BackgroundLoad::Stashes);
        true
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
        self.tasks.commits_requested_limit = self.tasks.commits_requested_limit.max(limit);
        let fast = self.ui.commits.items.is_empty();
        let request = self.tasks.task_manager.enqueue(
            key.clone(),
            priority,
            TaskRequestKind::LoadCommits { limit },
        );
        self.queue_background(request, BackgroundLoad::Commits { limit, fast });
        true
    }

    pub fn has_pending_refresh_work(&self) -> bool {
        self.refresh.pending_refresh.is_some() || !self.tasks.pending_background_tasks.is_empty()
    }

    pub(super) fn pending_refresh_kind(&self) -> Option<RefreshKind> {
        self.refresh.pending_refresh
    }

    pub(super) fn maybe_schedule_commits_extend(&mut self) {
        if self.ui.active_panel != SidePanel::Commits || self.ui.commits.tree_mode.active {
            return;
        }
        if self.has_background_task(&TaskKey::Commits) || self.ui.commits.dirty {
            return;
        }
        let len = self.ui.commits.items.len();
        if len == 0 {
            return;
        }
        let Some(selected) = self.ui.commits.panel.list_state.selected() else {
            return;
        };
        if selected + COMMITS_LOAD_AHEAD_THRESHOLD < len {
            return;
        }
        self.tasks.commits_requested_limit = self
            .tasks
            .commits_requested_limit
            .saturating_add(COMMITS_LOAD_STEP);
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

    pub fn open_selected_branch_commits(&mut self, limit: usize) -> Result<()> {
        if self.ui.active_panel != SidePanel::LocalBranches {
            return Ok(());
        }
        let Some(branch_name) = self.selected_branch_name() else {
            return Ok(());
        };

        self.ui.branches.commits_subview_active = true;
        self.ui.branches.commits_subview_loading = true;
        self.ui.branches.commits_subview_source = Some(branch_name.clone());
        self.ui.branches.commits_subview.items.clear();
        self.ui.branches.commits_subview.dirty = false;
        self.ui.branches.commits_subview.highlighted_oids.clear();
        self.ui.branches.commits_subview.tree_mode = TreeModeState::default();
        self.ui
            .branches
            .commits_subview
            .panel
            .list_state
            .select(None);
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
        self.queue_background(
            request,
            BackgroundLoad::BranchCommits {
                branch: branch_name.clone(),
                limit,
            },
        );
        self.push_log(
            format!("branch commits: {} (Esc to back)", branch_name),
            true,
        );
        Ok(())
    }

    pub fn close_branch_commits_subview(&mut self) {
        if !self.ui.branches.commits_subview_active {
            return;
        }

        let source_branch = self.ui.branches.commits_subview_source.take();
        self.ui.branches.commits_subview_active = false;
        self.ui.branches.commits_subview_loading = false;
        self.ui.branches.commits_subview = CommitsPanelState::default();

        let selected_index = source_branch.and_then(|name| {
            self.ui
                .branches
                .items
                .iter()
                .position(|branch| branch.name == name)
        });
        if self.ui.branches.items.is_empty() {
            self.ui.branches.panel.list_state.select(None);
        } else {
            self.ui
                .branches
                .panel
                .list_state
                .select(selected_index.or(Some(0)));
        }
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

    pub fn stash_open_tree_or_toggle_dir(&mut self) -> Result<()> {
        if self.ui.active_panel != SidePanel::Stash {
            return Ok(());
        }

        if !self.ui.stash.tree_mode.active {
            let Some(index) = self.selected_stash_index() else {
                return Ok(());
            };
            self.ui.stash.tree_mode.selected_source = Some(index);
            let request = self.tasks.task_manager.enqueue(
                TaskKey::StashFiles,
                TaskPriority::High,
                TaskRequestKind::LoadStashFiles { index },
            );
            self.queue_background(request, BackgroundLoad::StashFiles { index });
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
        self.cancel_background_task(&TaskKey::StashFiles);
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
        if was_open || self.ui.stash.tree_mode.selected_source.is_some() {
            self.ui.stash.tree_mode.selected_source = None;
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
            self.ui.commits.tree_mode.selected_source = Some(oid.clone());
            let request = self.tasks.task_manager.enqueue(
                TaskKey::CommitFiles,
                TaskPriority::High,
                TaskRequestKind::LoadCommitFiles { oid: oid.clone() },
            );
            self.queue_background(request, BackgroundLoad::CommitFiles { oid });
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
            &self.ui.commits.tree_mode.files,
            &mut self.ui.commits.tree_mode.expanded_dirs,
            &mut self.ui.commits.tree_mode.nodes,
            &mut self.ui.commits.panel.list_state,
        );
        Ok(())
    }

    pub fn commit_close_tree(&mut self) {
        self.cancel_background_task(&TaskKey::CommitFiles);
        let selected_source_index = self
            .ui
            .commits
            .tree_mode
            .selected_source
            .as_ref()
            .and_then(|oid| self.ui.commits.items.iter().position(|c| c.oid == *oid));

        let was_open = self.ui.commits.tree_mode.active;
        revision_tree::close_tree_mode(
            &mut self.ui.commits.tree_mode.active,
            &mut self.ui.commits.tree_mode.nodes,
            &mut self.ui.commits.tree_mode.files,
            &mut self.ui.commits.tree_mode.expanded_dirs,
            &mut self.ui.commits.panel.list_state,
            selected_source_index,
            self.ui.commits.items.len(),
        );
        if was_open || self.ui.commits.tree_mode.selected_source.is_some() {
            self.ui.commits.tree_mode.selected_source = None;
        }
    }

    pub fn recompute_commit_highlight(&mut self) {
        use crate::app::graph_highlight::compute_highlight_set;
        if self.ui.active_panel == SidePanel::Commits && !self.ui.commits.tree_mode.active {
            if let Some(idx) = self.ui.commits.panel.list_state.selected() {
                if let Some(commit) = self.ui.commits.items.get(idx) {
                    let oid = commit.oid.clone();
                    self.ui.commits.highlighted_oids =
                        compute_highlight_set(&self.ui.commits.items, &oid);
                    return;
                }
            }
        }
        self.ui.commits.highlighted_oids.clear();
    }

    pub(super) fn max_refresh_kind(a: RefreshKind, b: RefreshKind) -> RefreshKind {
        use RefreshKind::*;
        match (a, b) {
            (Full, _) | (_, Full) => Full,
            (StatusAndRefs, _) | (_, StatusAndRefs) => StatusAndRefs,
            _ => StatusOnly,
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
        assert!(app.diff.document().lines.is_empty());
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
        assert_eq!(app.diff.scroll(), 0);
        // With some diff content, scroll can increase
        app.diff.set_test_document(
            vec![
                crate::git::DiffLine {
                    kind: crate::git::DiffLineKind::Added,
                    content: "line".to_string(),
                };
                20
            ]
            .into(),
        );
        app.diff_scroll_down();
        assert!(app.diff.scroll() > 0);
        app.diff_scroll_up();
        assert_eq!(app.diff.scroll(), 0);
    }

    #[test]
    fn test_toggle_visual_select_mode() {
        let mut app = mock_app();
        assert!(!app.files.view().visual_mode);
        app.toggle_visual_select_mode();
        assert!(app.files.view().visual_mode);
        app.toggle_visual_select_mode();
        assert!(!app.files.view().visual_mode);
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
