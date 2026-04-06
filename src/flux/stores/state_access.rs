use crate::app::{CommitFieldFocus, InputMode, RefreshKind, SearchScopeKey, SidePanel};
use crate::flux::action::DomainAction;
use crate::git::{CommitInfo, FileEntry, StashInfo};
use std::path::PathBuf;
use std::time::Duration;

/// StateAccess trait provides an abstraction layer between Stores and App.
/// This decouples Stores from App's concrete implementation, making them easier to test
/// and allowing App's internal structure to evolve independently.
pub trait StateAccess {
    // Logging
    fn push_log(&mut self, msg: String, success: bool);

    // Input management
    fn cancel_input(&mut self);
    fn clear_search(&mut self);
    fn confirm_search_input(&mut self);

    // Refresh coordination
    fn request_refresh(&mut self, kind: RefreshKind);
    fn has_pending_diff_reload(&self) -> bool;
    fn diff_reload_debounce_elapsed(&self, debounce: Duration) -> bool;
    fn schedule_diff_reload(&mut self);

    // Branch operations
    fn take_branch_switch_target(&mut self) -> Option<String>;
    fn selected_branch_name(&self) -> Option<String>;

    // Selection queries
    fn selected_file_entry(&self) -> Option<&FileEntry>;
    fn selected_commit(&self) -> Option<&CommitInfo>;
    fn selected_stash(&self) -> Option<&StashInfo>;
    fn selected_stash_index(&self) -> Option<usize>;

    // Input state access
    fn commit_focus(&self) -> CommitFieldFocus;
    fn set_commit_focus(&mut self, focus: CommitFieldFocus);
    fn commit_message_buffer(&self) -> &str;
    fn commit_description_buffer(&self) -> &str;
    fn clear_commit_buffers(&mut self);
    fn input_buffer(&self) -> &str;
    fn clear_input_buffer(&mut self);
    fn stash_message_buffer(&self) -> &str;
    fn stash_targets(&self) -> &[PathBuf];
    fn clear_stash_buffers(&mut self);
    fn set_input_mode(&mut self, mode: Option<InputMode>);
    fn input_mode(&self) -> Option<InputMode>;

    // Command palette
    fn resolve_command_palette_command(&self, cmd: &str) -> Option<DomainAction>;

    // Panel state
    fn active_panel(&self) -> SidePanel;
    fn set_active_panel(&mut self, panel: SidePanel);
    fn is_fetching_remote(&self) -> bool;
    fn set_fetching_remote(&mut self, fetching: bool);

    // Search state
    fn search_scope(&self) -> SearchScopeKey;
    fn search_query(&self) -> &str;
    fn apply_search_query(&mut self, query: String) -> usize;
    fn search_select_initial_match(&mut self) -> bool;
    fn search_jump_next(&mut self) -> bool;
    fn search_jump_prev(&mut self) -> bool;
    fn restore_search_for_active_scope(&mut self);

    // App lifecycle
    fn set_running(&mut self, running: bool);
    fn mark_all_dirty(&mut self);

    // Navigation
    fn list_down(&mut self);
    fn list_up(&mut self);
    fn toggle_selected_dir(&mut self);
    fn collapse_all(&mut self);
    fn expand_all(&mut self);
    fn diff_scroll_up(&mut self);
    fn diff_scroll_down(&mut self);
    fn recompute_commit_highlight(&mut self);

    // Overlay / input mode launchers
    fn start_command_palette(&mut self);
    fn start_branch_create_input(&mut self);
    fn start_search_input(&mut self);
    fn start_stash_editor(&mut self, targets: Vec<PathBuf>);

    // Visual selection
    fn toggle_visual_select_mode(&mut self);
    fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf>;
    fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf>;
    fn clear_files_visual_selection(&mut self);

    // Revision tree
    fn stash_close_tree(&mut self);
    fn commit_close_tree(&mut self);
    fn close_branch_commits_subview(&mut self);

    // Commit description buffer manipulation
    fn push_newline_to_commit_description(&mut self);
    fn pop_commit_message_char(&mut self);
    fn pop_commit_description_char(&mut self);
    fn pop_input_buffer_char(&mut self);
    fn pop_stash_message_char(&mut self);
    fn push_commit_message_char(&mut self, c: char);
    fn push_commit_description_char(&mut self, c: char);
    fn push_input_buffer_char(&mut self, c: char);
    fn push_stash_message_char(&mut self, c: char);

    // Buffer cloning helpers (needed for borrow-safe reads before mutation)
    fn input_buffer_clone(&self) -> String;
    fn search_query_clone(&self) -> String;
}
