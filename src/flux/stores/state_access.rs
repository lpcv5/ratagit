use crate::app::{CommitFieldFocus, InputMode, RefreshKind, SidePanel};
use crate::flux::action::DomainAction;
use std::path::PathBuf;
use std::time::Duration;

/// Dirty-region flags passed from stores to the App after reducing an action.
/// Mirrors the bit positions in `UiInvalidation` without requiring a concrete `App` reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DirtyHint(pub u8);

impl DirtyHint {
    pub const MAIN_CONTENT: u8 = 0b0000_0001;
    pub const DIFF: u8 = 0b0000_0010;
    pub const COMMAND_LOG: u8 = 0b0000_0100;
    pub const SHORTCUT_BAR: u8 = 0b0000_1000;
    pub const OVERLAY: u8 = 0b0001_0000;
}

// ---------------------------------------------------------------------------
// Focused sub-traits
// ---------------------------------------------------------------------------

/// Core app lifecycle: logging, running state, dirty marking.
pub trait CoreAccess {
    fn push_log(&mut self, msg: String, success: bool);
    fn set_running(&mut self, running: bool);
    fn mark_all_dirty(&mut self);
    /// Granular dirty-region marking — prefer over `mark_all_dirty` when possible.
    fn mark_dirty(&mut self, hint: DirtyHint);
    fn resolve_command_palette_command(&self, cmd: &str) -> Option<DomainAction>;
    fn selected_branch_name(&self) -> Option<String>;
    fn selected_stash_index(&self) -> Option<usize>;
    fn take_branch_switch_target(&mut self) -> Option<String>;
    fn is_fetching_remote(&self) -> bool;
    fn set_fetching_remote(&mut self, fetching: bool);
    fn active_panel(&self) -> SidePanel;
    fn set_active_panel(&mut self, panel: SidePanel);
    fn has_uncommitted_changes(&self) -> bool;
}

/// Text input buffers: commit message, stash message, generic input buffer.
pub trait InputAccess {
    fn cancel_input(&mut self);
    fn input_mode(&self) -> Option<InputMode>;
    fn set_input_mode(&mut self, mode: Option<InputMode>);

    // Commit buffers
    fn commit_focus(&self) -> CommitFieldFocus;
    fn set_commit_focus(&mut self, focus: CommitFieldFocus);
    fn commit_message_buffer(&self) -> &str;
    fn commit_description_buffer(&self) -> &str;
    fn clear_commit_buffers(&mut self);
    fn push_commit_message_char(&mut self, c: char);
    fn push_commit_description_char(&mut self, c: char);
    fn push_newline_to_commit_description(&mut self);
    fn pop_commit_message_char(&mut self);
    fn pop_commit_description_char(&mut self);

    // Generic input buffer
    fn input_buffer(&self) -> &str;
    fn input_buffer_clone(&self) -> String;
    fn clear_input_buffer(&mut self);
    fn push_input_buffer_char(&mut self, c: char);
    fn pop_input_buffer_char(&mut self);

    // Stash buffers
    fn stash_message_buffer(&self) -> &str;
    fn stash_targets(&self) -> &[PathBuf];
    fn clear_stash_buffers(&mut self);
    fn push_stash_message_char(&mut self, c: char);
    fn pop_stash_message_char(&mut self);
}

/// List navigation, panel switching, diff scrolling.
pub trait NavigationAccess {
    fn list_down(&mut self);
    fn list_up(&mut self);
    fn toggle_selected_dir(&mut self);
    fn collapse_all(&mut self);
    fn expand_all(&mut self);
    fn diff_scroll_up(&mut self);
    fn diff_scroll_down(&mut self);
    fn recompute_commit_highlight(&mut self);
}

/// Search query management and navigation.
pub trait SearchAccess {
    fn clear_search(&mut self);
    fn confirm_search_input(&mut self);
    fn apply_search_query(&mut self, query: String) -> usize;
    fn search_select_initial_match(&mut self) -> bool;
    fn search_jump_next(&mut self) -> bool;
    fn search_jump_prev(&mut self) -> bool;
    fn restore_search_for_active_scope(&mut self);
    fn search_query_clone(&self) -> String;
}

/// Visual selection mode in the files panel.
pub trait SelectionAccess {
    fn toggle_visual_select_mode(&mut self);
    fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf>;
    fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf>;
    fn clear_files_visual_selection(&mut self);
    /// Returns paths of all files (staged + unstaged + untracked).
    fn all_file_paths(&self) -> Vec<PathBuf>;
}

/// Refresh and diff-reload scheduling.
pub trait RefreshAccess {
    fn request_refresh(&mut self, kind: RefreshKind);
    fn has_pending_diff_reload(&self) -> bool;
    fn diff_reload_debounce_elapsed(&self, debounce: Duration) -> bool;
    fn schedule_diff_reload(&mut self);
}

/// Overlay / input-mode launchers.
pub trait OverlayAccess {
    fn start_command_palette(&mut self);
    fn start_branch_create_input(&mut self);
    fn start_search_input(&mut self);
    fn start_stash_editor(&mut self, targets: Vec<PathBuf>);
    /// Open the commit editor if there are staged files; otherwise prompt to stage all.
    /// Returns `true` if the editor was opened or the confirm dialog was shown.
    fn start_commit_editor_guarded(&mut self) -> bool;
    fn start_branch_switch_confirm(&mut self, target: String);
    fn start_commit_editor(&mut self);
}

/// Revision tree operations (commit tree, stash tree, branch commits subview).
pub trait RevisionAccess {
    fn stash_close_tree(&mut self);
    fn commit_close_tree(&mut self);
    fn close_branch_commits_subview(&mut self);
}

// ---------------------------------------------------------------------------
// Composite supertrait — keeps all existing code compiling unchanged.
// ReduceCtx still uses `&mut dyn StateAccess`; stores don't need to change.
// ---------------------------------------------------------------------------

/// Composite abstraction layer between Stores and App.
///
/// Composed of focused sub-traits so that future stores (or tests) can depend
/// on only the capabilities they actually need.  All existing code that uses
/// `&mut dyn StateAccess` continues to compile without modification.
pub trait StateAccess:
    CoreAccess
    + InputAccess
    + NavigationAccess
    + SearchAccess
    + SelectionAccess
    + RefreshAccess
    + OverlayAccess
    + RevisionAccess
{
}

/// Blanket impl: any type that implements all sub-traits automatically
/// implements `StateAccess`.
impl<T> StateAccess for T where
    T: CoreAccess
        + InputAccess
        + NavigationAccess
        + SearchAccess
        + SelectionAccess
        + RefreshAccess
        + OverlayAccess
        + RevisionAccess
{
}
