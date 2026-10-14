use crate::app::commands::{CommandEnv, CommandHelp};
use crate::app::{
    AppState, BranchesPanelState, CommitFieldFocus, CommitsPanelState, FilesPanelState, InputMode,
    RenderCache, SidePanel, StashPanelState,
};
use crate::app::{SelectionView, UiContext};
use crate::config::keymap::Keymap;
use crate::git::DiffLine;

pub struct CommandLogSnapshotEntry<'a> {
    pub command: &'a str,
    pub success: bool,
}

pub struct AppStateSnapshot<'a> {
    pub diff_interaction: &'a crate::app::DiffInteraction,
    pub diff_ready: bool,
    pub context: UiContext,
    pub available_commands: Vec<CommandHelp>,
    pub keymap: &'a Keymap,
    pub active_panel: SidePanel,
    pub input_mode: Option<InputMode>,
    pub input_buffer: &'a str,

    pub files: &'a FilesPanelState,
    pub branches: &'a BranchesPanelState,
    pub commits: &'a CommitsPanelState,
    pub stash: &'a StashPanelState,
    pub render_cache: &'a RenderCache,

    #[cfg_attr(not(test), allow(dead_code))]
    pub operation: crate::git::operation::OperationMachine,
    #[cfg_attr(not(test), allow(dead_code))]
    pub current_patch: Option<std::sync::Arc<crate::git::patch::Patch>>,
    pub current_diff: &'a [DiffLine],
    pub diff_scroll: usize,
    pub diff_loading: bool,

    pub command_log: Vec<CommandLogSnapshotEntry<'a>>,
    pub shortcut_hints: Vec<(String, String)>,

    pub commit_focus: CommitFieldFocus,
    pub commit_message_buffer: &'a str,
    pub commit_description_buffer: &'a str,
    pub stash_message_buffer: &'a str,
    pub stash_targets: Vec<String>,

    pub branch_switch_target: Option<&'a str>,
    pub uncommitted_change_count: usize,

    pub files_search_query: Option<&'a str>,
    pub branches_search_query: Option<&'a str>,
    pub commits_search_query: Option<&'a str>,
    pub stash_search_query: Option<&'a str>,
    pub has_search_for_active_scope: bool,
    pub has_search_query_for_active_scope: bool,
    /// True when background Git tasks are in flight.
    pub has_background_tasks: bool,
}

impl<'a> AppStateSnapshot<'a> {
    pub fn command_env(&self) -> CommandEnv<'_> {
        CommandEnv {
            diff: crate::app::HunkView {
                patch: self.current_patch.as_deref(),
                interaction: self.diff_interaction,
                ready: self.diff_ready,
                files_context: self.active_panel == SidePanel::Files,
            },
            selection: SelectionView::new(
                self.context.view.clone(),
                self.files,
                self.branches,
                self.commits,
                self.stash,
            ),
            input: self.context.input,
            commit_focus: self.commit_focus,
            has_search: self.has_search_for_active_scope,
            has_search_query: self.has_search_query_for_active_scope,
        }
    }

    #[cfg(test)]
    pub fn from_app(app: &'a AppState) -> Self {
        let diff = app.diff.view();
        Self {
            diff_interaction: diff.interaction,
            diff_ready: !diff.loading,
            context: app.ui_context(),
            available_commands: app.available_commands(),
            keymap: app.keymap(),
            active_panel: app.ui.active_panel,
            input_mode: app.input.mode,
            input_buffer: app.input.buffer.as_str(),
            files: app.files.view(),
            branches: &app.ui.branches,
            commits: &app.ui.commits,
            stash: &app.ui.stash,
            render_cache: &app.ui.render_cache,
            operation: app.git.operation.clone(),
            current_patch: diff.document.patch.clone(),
            current_diff: &diff.document.lines,
            diff_scroll: diff.scroll,
            diff_loading: diff.show_loading(),
            command_log: app
                .command_log
                .iter()
                .map(|entry| CommandLogSnapshotEntry {
                    command: entry.command.as_str(),
                    success: entry.success,
                })
                .collect(),
            shortcut_hints: app.shortcut_hints(),
            commit_focus: app.input.commit_focus,
            commit_message_buffer: app.input.commit_message_buffer.as_str(),
            commit_description_buffer: app.input.commit_description_buffer.as_str(),
            stash_message_buffer: app.input.stash_message_buffer.as_str(),
            stash_targets: app
                .input
                .stash_targets
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            branch_switch_target: app.pending_branch_switch_target(),
            uncommitted_change_count: app.files.change_count(),
            files_search_query: app.search_query_for_scope(SidePanel::Files, false, false),
            branches_search_query: app.search_query_for_scope(
                SidePanel::LocalBranches,
                false,
                false,
            ),
            commits_search_query: app.search_query_for_scope(
                SidePanel::Commits,
                app.ui.commits.tree_mode.active,
                false,
            ),
            stash_search_query: app.search_query_for_scope(
                SidePanel::Stash,
                false,
                app.ui.stash.tree_mode.active,
            ),
            has_search_for_active_scope: app.has_search_for_active_scope(),
            has_search_query_for_active_scope: app.has_search_query_for_active_scope(),
            has_background_tasks: app.has_pending_refresh_work(),
        }
    }
}

/// Owned version of CommandLogSnapshotEntry — no lifetime binding.
pub struct CommandLogSnapshotEntryOwned {
    pub command: String,
    pub success: bool,
}

/// Owned version of AppStateSnapshot that can be sent through channels
/// without holding the App lock. All fields are cloned/owned.
pub struct AppStateSnapshotOwned {
    pub diff_interaction: crate::app::DiffInteraction,
    pub diff_ready: bool,
    pub context: UiContext,
    pub available_commands: Vec<CommandHelp>,
    pub keymap: Keymap,
    pub active_panel: SidePanel,
    pub input_mode: Option<InputMode>,
    pub input_buffer: String,

    pub files: FilesPanelState,
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,
    pub render_cache: RenderCache,

    pub operation: crate::git::operation::OperationMachine,
    pub current_patch: Option<std::sync::Arc<crate::git::patch::Patch>>,
    pub current_diff: Vec<DiffLine>,
    pub diff_scroll: usize,
    pub diff_loading: bool,

    pub command_log: Vec<CommandLogSnapshotEntryOwned>,
    pub shortcut_hints: Vec<(String, String)>,

    pub commit_focus: CommitFieldFocus,
    pub commit_message_buffer: String,
    pub commit_description_buffer: String,
    pub stash_message_buffer: String,
    pub stash_targets: Vec<String>,

    pub branch_switch_target: Option<String>,
    pub uncommitted_change_count: usize,

    pub files_search_query: Option<String>,
    pub branches_search_query: Option<String>,
    pub commits_search_query: Option<String>,
    pub stash_search_query: Option<String>,
    pub has_search_for_active_scope: bool,
    pub has_search_query_for_active_scope: bool,

    // State for tick check (Phase 4)
    pub running: bool,
    pub has_pending_refresh_work: bool,
    pub pending_diff_reload: bool,
    pub pending_diff_reload_at: Option<std::time::Instant>,
}

impl AppStateSnapshotOwned {
    pub fn from_app(app: &AppState) -> Self {
        let diff = app.diff.view();
        Self {
            diff_interaction: diff.interaction.clone(),
            diff_ready: !diff.loading,
            context: app.ui_context(),
            available_commands: app.available_commands(),
            keymap: app.keymap().clone(),
            active_panel: app.ui.active_panel,
            input_mode: app.input.mode,
            input_buffer: app.input.buffer.clone(),
            files: app.files.view().clone(),
            branches: app.ui.branches.clone(),
            commits: app.ui.commits.clone(),
            stash: app.ui.stash.clone(),
            render_cache: app.ui.render_cache.clone(),
            operation: app.git.operation.clone(),
            current_patch: diff.document.patch.clone(),
            current_diff: diff.document.lines.clone(),
            diff_scroll: diff.scroll,
            diff_loading: diff.show_loading(),
            command_log: app
                .command_log
                .iter()
                .map(|entry| CommandLogSnapshotEntryOwned {
                    command: entry.command.clone(),
                    success: entry.success,
                })
                .collect(),
            shortcut_hints: app.shortcut_hints(),
            commit_focus: app.input.commit_focus,
            commit_message_buffer: app.input.commit_message_buffer.clone(),
            commit_description_buffer: app.input.commit_description_buffer.clone(),
            stash_message_buffer: app.input.stash_message_buffer.clone(),
            stash_targets: app
                .input
                .stash_targets
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            branch_switch_target: app.pending_branch_switch_target().map(str::to_owned),
            uncommitted_change_count: app.files.change_count(),
            files_search_query: app
                .search_query_for_scope(SidePanel::Files, false, false)
                .map(str::to_owned),
            branches_search_query: app
                .search_query_for_scope(SidePanel::LocalBranches, false, false)
                .map(str::to_owned),
            commits_search_query: app
                .search_query_for_scope(SidePanel::Commits, app.ui.commits.tree_mode.active, false)
                .map(str::to_owned),
            stash_search_query: app
                .search_query_for_scope(SidePanel::Stash, false, app.ui.stash.tree_mode.active)
                .map(str::to_owned),
            has_search_for_active_scope: app.has_search_for_active_scope(),
            has_search_query_for_active_scope: app.has_search_query_for_active_scope(),
            running: app.running,
            has_pending_refresh_work: app.has_pending_refresh_work(),
            pending_diff_reload: app.has_pending_diff_reload(),
            pending_diff_reload_at: app.pending_diff_reload_at(),
        }
    }

    /// Check if a tick event should be sent to the backend.
    pub fn should_tick(&self, debounce: std::time::Duration) -> bool {
        self.has_pending_refresh_work
            || (self.pending_diff_reload
                && self
                    .pending_diff_reload_at
                    .is_some_and(|requested_at| requested_at.elapsed() >= debounce))
    }

    /// Borrow-based view of this owned snapshot, compatible with all existing render functions.
    pub fn as_snapshot(&self) -> AppStateSnapshot<'_> {
        AppStateSnapshot {
            diff_interaction: &self.diff_interaction,
            diff_ready: self.diff_ready,
            context: self.context.clone(),
            available_commands: self.available_commands.clone(),
            keymap: &self.keymap,
            active_panel: self.active_panel,
            input_mode: self.input_mode,
            input_buffer: &self.input_buffer,
            files: &self.files,
            branches: &self.branches,
            commits: &self.commits,
            stash: &self.stash,
            render_cache: &self.render_cache,
            operation: self.operation.clone(),
            current_patch: self.current_patch.clone(),
            current_diff: &self.current_diff,
            diff_scroll: self.diff_scroll,
            diff_loading: self.diff_loading,
            command_log: self
                .command_log
                .iter()
                .map(|entry| CommandLogSnapshotEntry {
                    command: entry.command.as_str(),
                    success: entry.success,
                })
                .collect(),
            shortcut_hints: self.shortcut_hints.clone(),
            commit_focus: self.commit_focus,
            commit_message_buffer: &self.commit_message_buffer,
            commit_description_buffer: &self.commit_description_buffer,
            stash_message_buffer: &self.stash_message_buffer,
            stash_targets: self.stash_targets.clone(),
            branch_switch_target: self.branch_switch_target.as_deref(),
            uncommitted_change_count: self.uncommitted_change_count,
            files_search_query: self.files_search_query.as_deref(),
            branches_search_query: self.branches_search_query.as_deref(),
            commits_search_query: self.commits_search_query.as_deref(),
            stash_search_query: self.stash_search_query.as_deref(),
            has_search_for_active_scope: self.has_search_for_active_scope,
            has_search_query_for_active_scope: self.has_search_query_for_active_scope,
            has_background_tasks: self.has_pending_refresh_work,
        }
    }
}
