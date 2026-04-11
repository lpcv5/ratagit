use crate::app::{
    branch_panel_adapter, commits_panel_adapter, files_panel_adapter, stash_panel_adapter,
};
use crate::app::{
    App, BranchesPanelState, CommitFieldFocus, CommitsPanelState, FilesPanelState, InputMode,
    RenderCache, SidePanel, StashPanelState,
};
use crate::config::keymap::Keymap;
use crate::flux::branch_backend::BranchPanelViewState;
use crate::flux::commits_backend::CommitsPanelViewState;
use crate::flux::files_backend::FilesPanelViewState;
use crate::flux::git_backend::detail::{DetailBackend, DetailPanelViewState};
use crate::flux::git_backend::stash::StashPanelViewState;
pub struct CommandLogSnapshotEntry<'a> {
    pub command: &'a str,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLogViewEntry {
    pub command: String,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandLogViewState {
    pub branch_input: Option<String>,
    pub entries: Vec<CommandLogViewEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitEditorViewState {
    pub is_open: bool,
    pub commit_focus: CommitFieldFocus,
    pub message: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEditorViewState {
    pub is_open: bool,
    pub message: String,
    pub targets: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandPaletteViewState {
    pub is_open: bool,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSwitchConfirmViewState {
    pub is_open: bool,
    pub target: Option<String>,
    pub uncommitted_change_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitAllConfirmViewState {
    pub is_open: bool,
    pub uncommitted_change_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutHintViewEntry {
    pub keys: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutBarViewState {
    pub has_background_tasks: bool,
    pub search_input: Option<String>,
    pub hints: Vec<ShortcutHintViewEntry>,
}

pub struct AppStateSnapshot<'a> {
    pub keymap: &'a Keymap,
    pub active_panel: SidePanel,
    pub input_mode: Option<InputMode>,
    pub input_buffer: &'a str,

    files: &'a FilesPanelState,
    branches: &'a BranchesPanelState,
    commits: &'a CommitsPanelState,
    stash: &'a StashPanelState,
    pub render_cache: &'a RenderCache,

    pub diff_scroll: usize,
    pub branch_log_limit: usize,
    pub detail: &'a crate::app::DetailState,

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
    #[cfg(test)]
    pub fn from_app(app: &'a App) -> Self {
        Self {
            keymap: app.keymap(),
            active_panel: app.ui.active_panel,
            input_mode: app.input.mode,
            input_buffer: app.input.buffer.as_str(),
            files: &app.ui.files,
            branches: &app.ui.branches,
            commits: &app.ui.commits,
            stash: &app.ui.stash,
            render_cache: &app.ui.render_cache,
            diff_scroll: app.ui.diff_scroll,
            branch_log_limit: app.current_branch_log_limit(),
            detail: &app.git.detail,
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
            uncommitted_change_count: app.git.status.staged.len()
                + app.git.status.unstaged.len()
                + app.git.status.untracked.len(),
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

    pub fn files_view_state(&self) -> FilesPanelViewState {
        files_panel_adapter::view_state_from_shell(self.files)
    }

    pub fn branches_view_state(&self) -> BranchPanelViewState {
        branch_panel_adapter::view_state_from_shell(self.branches)
    }

    pub fn commits_view_state(&self) -> CommitsPanelViewState {
        commits_panel_adapter::view_state_from_shell(self.commits)
    }

    pub fn stash_view_state(&self) -> StashPanelViewState {
        stash_panel_adapter::view_state_from_shell(self.stash)
    }

    pub fn detail_view_state(&self) -> DetailPanelViewState {
        let mut detail = self.detail.panel.clone();
        if !detail.is_loading {
            detail.request = DetailBackend::request_from_shell_panels(
                self.active_panel,
                self.files,
                self.branches,
                self.commits,
                self.stash,
                self.branch_log_limit,
            );
        }
        DetailBackend::build_view_state(&detail, self.diff_scroll)
    }

    pub fn command_log_view_state(&self) -> CommandLogViewState {
        CommandLogViewState {
            branch_input: (self.input_mode == Some(InputMode::CreateBranch))
                .then(|| self.input_buffer.to_string()),
            entries: self
                .command_log
                .iter()
                .map(|entry| CommandLogViewEntry {
                    command: entry.command.to_string(),
                    success: entry.success,
                })
                .collect(),
        }
    }

    pub fn commit_editor_view_state(&self) -> CommitEditorViewState {
        CommitEditorViewState {
            is_open: self.input_mode == Some(InputMode::CommitEditor),
            commit_focus: self.commit_focus,
            message: self.commit_message_buffer.to_string(),
            description: self.commit_description_buffer.to_string(),
        }
    }

    pub fn stash_editor_view_state(&self) -> StashEditorViewState {
        StashEditorViewState {
            is_open: self.input_mode == Some(InputMode::StashEditor),
            message: self.stash_message_buffer.to_string(),
            targets: self.stash_targets.clone(),
        }
    }

    pub fn command_palette_view_state(&self) -> CommandPaletteViewState {
        CommandPaletteViewState {
            is_open: self.input_mode == Some(InputMode::CommandPalette),
            input: self.input_buffer.to_string(),
        }
    }

    pub fn branch_switch_confirm_view_state(&self) -> BranchSwitchConfirmViewState {
        BranchSwitchConfirmViewState {
            is_open: self.input_mode == Some(InputMode::BranchSwitchConfirm),
            target: self.branch_switch_target.map(str::to_owned),
            uncommitted_change_count: self.uncommitted_change_count,
        }
    }

    pub fn commit_all_confirm_view_state(&self) -> CommitAllConfirmViewState {
        CommitAllConfirmViewState {
            is_open: self.input_mode == Some(InputMode::CommitAllConfirm),
            uncommitted_change_count: self.uncommitted_change_count,
        }
    }

    pub fn shortcut_bar_view_state(&self) -> ShortcutBarViewState {
        ShortcutBarViewState {
            has_background_tasks: self.has_background_tasks,
            search_input: (self.input_mode == Some(InputMode::Search))
                .then(|| self.input_buffer.to_string()),
            hints: self
                .shortcut_hints
                .iter()
                .map(|(keys, description)| ShortcutHintViewEntry {
                    keys: keys.clone(),
                    description: description.clone(),
                })
                .collect(),
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
    pub keymap: Keymap,
    pub active_panel: SidePanel,
    pub input_mode: Option<InputMode>,
    pub input_buffer: String,

    pub files: FilesPanelState,
    pub branches: BranchesPanelState,
    pub commits: CommitsPanelState,
    pub stash: StashPanelState,
    pub render_cache: RenderCache,

    pub diff_scroll: usize,
    pub branch_log_limit: usize,
    pub detail: crate::app::DetailState,

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
    pub fn from_app(app: &App) -> Self {
        Self {
            keymap: app.keymap().clone(),
            active_panel: app.ui.active_panel,
            input_mode: app.input.mode,
            input_buffer: app.input.buffer.clone(),
            files: app.ui.files.clone(),
            branches: app.ui.branches.clone(),
            commits: app.ui.commits.clone(),
            stash: app.ui.stash.clone(),
            render_cache: app.ui.render_cache.clone(),
            diff_scroll: app.ui.diff_scroll,
            branch_log_limit: app.current_branch_log_limit(),
            detail: app.git.detail.clone(),
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
            uncommitted_change_count: app.git.status.staged.len()
                + app.git.status.unstaged.len()
                + app.git.status.untracked.len(),
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
            keymap: &self.keymap,
            active_panel: self.active_panel,
            input_mode: self.input_mode,
            input_buffer: &self.input_buffer,
            files: &self.files,
            branches: &self.branches,
            commits: &self.commits,
            stash: &self.stash,
            render_cache: &self.render_cache,
            diff_scroll: self.diff_scroll,
            branch_log_limit: self.branch_log_limit,
            detail: &self.detail,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::git::{CommitInfo, CommitSyncState, FileStatus, GraphCell};
    use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};
    use std::collections::HashSet;

    #[test]
    fn files_view_state_projects_selection_without_widget_state() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.ui.files.tree_nodes = vec![FileTreeNode {
            path: "src/main.rs".into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 1,
            is_dir: false,
            is_expanded: false,
        }];
        app.ui.files.panel.list_state.select(Some(0));
        app.ui.files.visual_mode = true;
        app.ui.files.visual_anchor = Some(0);

        let view = AppStateSnapshot::from_app(&app).files_view_state();

        assert_eq!(view.selection.selected_index, Some(0));
        assert!(view.selection.visual_mode);
        assert_eq!(view.selection.visual_anchor, Some(0));
        assert_eq!(view.nodes.len(), 1);
    }

    #[test]
    fn commits_view_state_projects_selection_without_widget_state() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.ui.commits.items = vec![CommitInfo {
            oid: "abc123".to_string(),
            message: "test commit".to_string(),
            author: "tester".to_string(),
            graph: vec![GraphCell {
                text: "*".to_string(),
                lane: 0,
                pipe_oid: None,
                pipe_oids: vec![],
            }],
            time: "2026-04-11 00:00".to_string(),
            parent_count: 1,
            sync_state: CommitSyncState::DefaultBranch,
            parent_oids: vec![],
        }];
        app.ui.commits.panel.list_state.select(Some(0));
        app.ui.commits.tree_mode.active = true;
        app.ui.commits.tree_mode.selected_source = Some("abc123".to_string());
        app.ui.commits.tree_mode.nodes = vec![FileTreeNode {
            path: "src/main.rs".into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }];
        app.ui.commits.highlighted_oids = HashSet::from(["abc123".to_string()]);

        let view = AppStateSnapshot::from_app(&app).commits_view_state();

        assert_eq!(view.selected_index, Some(0));
        assert_eq!(view.items.len(), 1);
        assert!(view.tree_mode.active);
        assert_eq!(view.tree_mode.selected_source.as_deref(), Some("abc123"));
        assert_eq!(view.tree_mode.nodes.len(), 1);
        assert!(view.highlighted_oids.contains("abc123"));
    }

    #[test]
    fn stash_view_state_projects_selection_without_widget_state() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.ui.stash.items = vec![crate::git::StashInfo {
            index: 1,
            message: "wip".to_string(),
        }];
        app.ui.stash.panel.list_state.select(Some(0));
        app.ui.stash.tree_mode.active = true;
        app.ui.stash.tree_mode.selected_source = Some(1);
        app.ui.stash.tree_mode.nodes = vec![FileTreeNode {
            path: "src/main.rs".into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        }];

        let view = AppStateSnapshot::from_app(&app).stash_view_state();

        assert_eq!(view.selected_index, Some(0));
        assert_eq!(view.items.len(), 1);
        assert!(view.tree_mode.active);
        assert_eq!(view.tree_mode.selected_source, Some(1));
        assert_eq!(view.tree_mode.nodes.len(), 1);
    }

    #[test]
    fn command_log_view_state_projects_branch_input_and_entries() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.input.mode = Some(InputMode::CreateBranch);
        app.input.buffer = "feature/x".to_string();
        app.push_log("created".to_string(), true);

        let view = AppStateSnapshot::from_app(&app).command_log_view_state();

        assert_eq!(view.branch_input.as_deref(), Some("feature/x"));
        assert_eq!(view.entries.len(), 1);
        assert_eq!(view.entries[0].command, "created");
        assert!(view.entries[0].success);
    }

    #[test]
    fn commit_editor_view_state_projects_focus_and_buffers() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.input.mode = Some(InputMode::CommitEditor);
        app.input.commit_focus = CommitFieldFocus::Description;
        app.input.commit_message_buffer = "msg".to_string();
        app.input.commit_description_buffer = "desc".to_string();

        let view = AppStateSnapshot::from_app(&app).commit_editor_view_state();

        assert!(view.is_open);
        assert_eq!(view.commit_focus, CommitFieldFocus::Description);
        assert_eq!(view.message, "msg");
        assert_eq!(view.description, "desc");
    }

    #[test]
    fn confirm_and_palette_views_project_input_mode_specific_state() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.input.mode = Some(InputMode::CommandPalette);
        app.input.buffer = "refresh".to_string();
        let palette = AppStateSnapshot::from_app(&app).command_palette_view_state();
        assert!(palette.is_open);
        assert_eq!(palette.input, "refresh");

        app.input.mode = Some(InputMode::BranchSwitchConfirm);
        app.input.branch_switch_target = Some("feature/x".to_string());
        let branch_confirm = AppStateSnapshot::from_app(&app).branch_switch_confirm_view_state();
        assert!(branch_confirm.is_open);
        assert_eq!(branch_confirm.target.as_deref(), Some("feature/x"));

        app.input.mode = Some(InputMode::CommitAllConfirm);
        let commit_all = AppStateSnapshot::from_app(&app).commit_all_confirm_view_state();
        assert!(commit_all.is_open);
        assert_eq!(
            commit_all.uncommitted_change_count,
            branch_confirm.uncommitted_change_count
        );
    }

    #[test]
    fn shortcut_bar_view_state_projects_search_and_hints() {
        let mut app =
            App::from_repo(Box::new(crate::flux::stores::test_support::MockRepo)).unwrap();
        app.input.mode = Some(InputMode::Search);
        app.input.buffer = "abc".to_string();

        let view = AppStateSnapshot::from_app(&app).shortcut_bar_view_state();

        assert_eq!(view.search_input.as_deref(), Some("abc"));
        assert!(!view.hints.is_empty());
    }
}
