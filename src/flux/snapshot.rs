use crate::app::{
    App, BranchesPanelState, CommitFieldFocus, CommitsPanelState, FilesPanelState, InputMode,
    RenderCache, SidePanel, StashPanelState,
};
use crate::config::keymap::Keymap;
use crate::git::DiffLine;

pub struct CommandLogSnapshotEntry<'a> {
    pub command: &'a str,
    pub success: bool,
}

pub struct AppStateSnapshot<'a> {
    pub keymap: &'a Keymap,
    pub active_panel: SidePanel,
    pub input_mode: Option<InputMode>,
    pub input_buffer: &'a str,

    pub files: &'a FilesPanelState,
    pub branches: &'a BranchesPanelState,
    pub commits: &'a CommitsPanelState,
    pub stash: &'a StashPanelState,
    pub render_cache: &'a RenderCache,

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
}

impl<'a> AppStateSnapshot<'a> {
    pub fn from_app(app: &'a App) -> Self {
        Self {
            keymap: app.keymap(),
            active_panel: app.active_panel,
            input_mode: app.input_mode,
            input_buffer: app.input_buffer.as_str(),
            files: &app.files,
            branches: &app.branches,
            commits: &app.commits,
            stash: &app.stash,
            render_cache: &app.render_cache,
            current_diff: &app.current_diff,
            diff_scroll: app.diff_scroll,
            diff_loading: app.has_pending_diff_reload() && app.current_diff.is_empty(),
            command_log: app
                .command_log
                .iter()
                .map(|entry| CommandLogSnapshotEntry {
                    command: entry.command.as_str(),
                    success: entry.success,
                })
                .collect(),
            shortcut_hints: app.shortcut_hints(),
            commit_focus: app.commit_focus,
            commit_message_buffer: app.commit_message_buffer.as_str(),
            commit_description_buffer: app.commit_description_buffer.as_str(),
            stash_message_buffer: app.stash_message_buffer.as_str(),
            stash_targets: app
                .stash_targets
                .iter()
                .map(|p| p.display().to_string())
                .collect(),
            branch_switch_target: app.pending_branch_switch_target(),
            uncommitted_change_count: app.status.staged.len()
                + app.status.unstaged.len()
                + app.status.untracked.len(),
            files_search_query: app.search_query_for_scope(SidePanel::Files, false, false),
            branches_search_query: app.search_query_for_scope(
                SidePanel::LocalBranches,
                false,
                false,
            ),
            commits_search_query: app.search_query_for_scope(
                SidePanel::Commits,
                app.commits.tree_mode.active,
                false,
            ),
            stash_search_query: app.search_query_for_scope(
                SidePanel::Stash,
                false,
                app.stash.tree_mode.active,
            ),
            has_search_for_active_scope: app.has_search_for_active_scope(),
            has_search_query_for_active_scope: app.has_search_query_for_active_scope(),
        }
    }
}
