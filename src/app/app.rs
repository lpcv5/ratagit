use super::dirty_flags;
use super::refresh_scheduler::RefreshScheduler;
use super::revisions::{RevisionAction, RevisionState};
use super::states::{CommandLogEntry, GitState, InputState, RenderCache, SidePanel, UiState};
use super::{AppRuntime, BackgroundLoad, DiffState, FilesState};
use crate::background::{TaskKey, TaskPriority};
use crate::config::keymap::Keymap;
#[cfg(test)]
use crate::flux::action::RuntimeEvent;
use crate::flux::command_output::CommandOutput;
use crate::flux::effects::EffectRequest;
use crate::flux::task_manager::TaskManager;
use crate::git::{Git2Repository, GitRepository, GitStatus};
use color_eyre::Result;

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

/// Reducer-owned state, caches and scheduling metadata. Contains no Git or I/O handles.
pub struct AppState {
    pub running: bool,
    pub git: GitState,
    pub ui: UiState,
    pub input: InputState,
    pub search: super::search::SearchState,
    pub command_log: crate::shared::Shared<Vec<CommandLogEntry>>,

    pub(super) refresh: RefreshScheduler,
    pub diff: DiffState,
    pub files: FilesState,
    pub revisions: RevisionState,
    pub(super) tasks: TaskManager,
    keymap: crate::shared::Shared<Keymap>,
}

/// Startup composition of reducer state and execution dependencies.
pub struct App {
    pub state: AppState,
    pub runtime: AppRuntime,
    /// Bootstrap output transferred to the dispatcher before its first action.
    pub initial_output: crate::flux::command_output::CommandOutput,
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
                    dirty: dirty_flags::DirtyFlags::default(),
                    render_cache: RenderCache::default(),
                },
                input: InputState::default(),
                search: super::search::SearchState::default(),
                command_log: Default::default(),
                refresh: RefreshScheduler::new(!preload_commits_and_diff),
                diff: DiffState::default(),
                files: FilesState::new(status, preload_commits_and_diff),
                revisions: RevisionState::new(branches, commits, stashes, preload_commits_and_diff),
                tasks: TaskManager::new(),
                keymap: keymap.into(),
            },
            runtime: AppRuntime::new(repo),
            initial_output: Default::default(),
        };
        app.refresh_render_cache();
        app.ui.dirty.mark_all();
        if preload_commits_and_diff {
            app.state.reload_diff_now(&mut app.initial_output);
        } else {
            app.state
                .request_refresh(RefreshKind::StatusAndRefs, &mut app.initial_output);
            app.state.schedule_diff_reload(&mut app.initial_output);
            // Load commits immediately on startup
            let _ = app.state.start_commits_load(
                app.state.revisions.commits_requested_limit(),
                &mut app.initial_output,
            );
        }
        Ok(app)
    }
}

impl AppState {
    pub(super) fn queue_background(
        &mut self,
        load: BackgroundLoad,
        priority: TaskPriority,
        output: &mut CommandOutput,
    ) {
        let request = self.tasks.start(load, priority);
        output.effect(EffectRequest::LoadBackground { request });
    }

    pub(crate) fn shared_keymap(&self) -> crate::shared::Shared<Keymap> {
        self.keymap.clone()
    }

    pub(crate) fn keymap(&self) -> &Keymap {
        &self.keymap
    }

    pub fn request_refresh(&mut self, kind: RefreshKind, output: &mut CommandOutput) {
        self.cancel_pending_diff_task(output);
        self.refresh.pending_full_status_after_fast = true;
        self.refresh.pending_refresh_fast_done = false;
        if matches!(kind, RefreshKind::Full) {
            self.reduce_revisions(RevisionAction::MarkCommitsDirty);
        }
        self.diff
            .repository_refreshed(!matches!(kind, RefreshKind::StatusOnly));
        self.refresh.pending_refresh = Some(match self.refresh.pending_refresh {
            None => kind,
            Some(existing) => Self::max_refresh_kind(existing, kind),
        });
    }

    pub(super) fn has_background_task(&self, key: &TaskKey) -> bool {
        self.tasks.has_task(key)
    }

    pub(super) fn can_start_background_task(&self, priority: TaskPriority) -> bool {
        self.tasks.can_start(priority)
    }

    pub(super) fn cancel_background_task(&mut self, key: &TaskKey, output: &mut CommandOutput) {
        self.tasks.cancel(key);
        output.cancel_background(key);
    }

    pub(super) fn start_status_load(&mut self, fast: bool, output: &mut CommandOutput) -> bool {
        let key = TaskKey::Status;
        if self.has_background_task(&key) {
            return true;
        }
        let priority = TaskPriority::High;
        if !self.can_start_background_task(priority) {
            return false;
        }

        self.queue_background(
            BackgroundLoad::Status {
                fast,
                operation_epoch: self.git.operation.epoch,
            },
            priority,
            output,
        );
        true
    }

    pub fn has_pending_refresh_work(&self) -> bool {
        self.refresh.pending_refresh.is_some() || !self.tasks.is_empty()
    }

    pub(crate) fn pending_refresh_kind(&self) -> Option<RefreshKind> {
        self.refresh.pending_refresh
    }

    /// Read-only observation for runtime diagnostics; acceptance remains in the reducer.
    pub(crate) fn background_completion_is_current(
        &self,
        completed: &crate::background::BackgroundCompleted,
    ) -> bool {
        self.tasks.is_current(completed) && self.diff_completion_is_current(completed)
    }

    pub(super) fn diff_completion_is_current(
        &self,
        completed: &crate::background::BackgroundCompleted,
    ) -> bool {
        let BackgroundLoad::Diff { cache_key, .. } = &completed.request.load else {
            return true;
        };
        self.diff.accepts_load(cache_key)
            && match &completed.result {
                Ok(payload) => {
                    matches!(payload.as_ref(), super::BackgroundPayload::Diff { cache_key: actual, .. } if actual == cache_key)
                }
                Err(_) => true,
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

    pub(super) fn max_refresh_kind(a: RefreshKind, b: RefreshKind) -> RefreshKind {
        use RefreshKind::*;
        match (a, b) {
            (Full, _) | (_, Full) => Full,
            (StatusAndRefs, _) | (_, StatusAndRefs) => StatusAndRefs,
            _ => StatusOnly,
        }
    }

    pub(crate) fn refresh_render_cache(&mut self) {
        self.ui.render_cache.files_visual_selected_indices = self.visual_selected_indices().into();
        self.ui.render_cache.files_search_summary =
            self.search_match_summary_for(SidePanel::Files, false, false);
        self.ui.render_cache.branches_search_summary =
            self.search_match_summary_for(SidePanel::LocalBranches, false, false);
        self.ui.render_cache.commits_search_summary = self.search_match_summary_for(
            SidePanel::Commits,
            self.revisions.view().commits.tree_mode.active,
            false,
        );
        self.ui.render_cache.stash_search_summary = self.search_match_summary_for(
            SidePanel::Stash,
            false,
            self.revisions.view().stash.tree_mode.active,
        );
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
    fn test_startup_requests_follow_first_action_without_running_during_composition() {
        use crate::flux::action::{Action, SystemAction};
        use crate::flux::dispatcher::Dispatcher;
        let mut app = App::build_with_repo(Box::new(MockRepo), Keymap::default(), false).unwrap();
        assert!(app.revisions.view().commits.items.is_empty());
        let mut dispatcher =
            Dispatcher::with_initial_output(std::mem::take(&mut app.initial_output));
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::AutoRefresh));
        let commands = dispatcher.dispatch(&mut app.state, envelope).commands;
        assert!(matches!(
            commands.as_slice(),
            [
                super::super::Command::Sync(crate::flux::action::DomainAction::Runtime(
                    RuntimeEvent::ProcessBackgroundLoads
                )),
                super::super::Command::Effect(EffectRequest::LoadBackground {
                    request: crate::background::TaskRequest {
                        load: BackgroundLoad::Commits {
                            limit: 30,
                            fast: true
                        },
                        ..
                    }
                }),
            ]
        ));
        assert!(
            app.revisions.view().commits.items.is_empty(),
            "publishing must not perform repository I/O"
        );
        let envelope = dispatcher.next_envelope(Action::System(SystemAction::Resize {
            width: 80,
            height: 24,
        }));
        assert!(dispatcher
            .dispatch(&mut app.state, envelope)
            .commands
            .is_empty());
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
        app.state.schedule_diff_reload(&mut app.initial_output);
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
