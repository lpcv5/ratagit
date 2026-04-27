mod actions;
mod branches;
mod commit_workflow;
mod commits;
mod details;
mod editor;
mod files;
mod navigation;
mod operations;
mod reducer;
mod results;
mod scroll;
mod search;
mod selectors;
mod snapshot;
mod state;
mod text_edit;
mod worktree;

pub use actions::{Action, Command, GitResult, UiAction, debounce_key_for_command};
pub use commits::{
    clamp_selected as clamp_commit_selection, commit_key,
    enter_multi_select as enter_commit_multi_select,
    is_selected_for_batch as commit_is_selected_for_batch,
    leave_multi_select as leave_commit_multi_select, move_selected as move_commit_selected,
    reconcile_after_items_appended as reconcile_commits_after_items_appended,
    reconcile_after_items_changed as reconcile_commits_after_items_changed, selected_commit,
    selected_commit_ids, selected_commits, toggle_multi_select as toggle_commit_multi_select,
};
pub use files::{
    CommitFileEntry, CommitFileStatus, CommitFilesPanelState, FileDiffTarget, FileEntry,
    FileInputMode, FileRowKind, FileTreeRow, FilesPanelState, build_commit_file_tree_rows,
    build_file_tree_rows, clamp_selected as clamp_file_selection, collect_directories,
    commit_file_tree_rows, enter_multi_select, file_tree_rows, initialize_commit_files_tree,
    initialize_tree_if_needed, initialize_tree_with_initial_expansion, leave_multi_select,
    move_commit_file_selected, move_selected, reconcile_after_items_changed,
    refresh_commit_files_tree_projection, refresh_tree_projection, select_commit_file_tree_path,
    select_file_tree_path, selected_commit_file, selected_commit_file_targets,
    selected_diff_targets, selected_row, selected_target_paths, toggle_commit_files_directory,
    toggle_current_row_selection, toggle_selected_directory,
};
pub use reducer::update;
pub use scroll::ScrollDirection;
pub use state::{
    AppState, AutoStashConfirmState, AutoStashOperation, BranchCreateState, BranchDeleteChoice,
    BranchDeleteMenuState, BranchDeleteMode, BranchEntry, BranchForceDeleteConfirmState,
    BranchRebaseChoice, BranchRebaseMenuState, BranchesPanelState, CachedBranchLog,
    CachedCommitDiff, CachedFilesDiff, CommitEditorIntent, CommitEntry, CommitField,
    CommitFileDiffPath, CommitFileDiffTarget, CommitHashStatus, CommitInputMode, CommitsPanelState,
    DetailsPanelState, DiscardConfirmState, EditorKind, EditorState, FilesSnapshot, PanelFocus,
    RefreshTarget, RepoSnapshot, ResetChoice, ResetMenuState, ResetMode, SearchScope, SearchState,
    StashEntry, StashPanelState, StashScope, StatusMode, StatusPanelState, WorkStatusState,
};

pub(crate) use actions::with_pending;
pub(crate) use reducer::push_notice;
pub(crate) use selectors::{selected_branch_name, selected_commit_id, selected_stash_id};

const DETAILS_DIFF_CACHE_LIMIT: usize = 16;
pub const BRANCH_DETAILS_LOG_MAX_COUNT: usize = 50;
pub const COMMITS_PAGE_SIZE: usize = 100;
pub const COMMITS_PREFETCH_THRESHOLD: usize = 20;
pub const FILES_DETAILS_DIFF_TARGET_LIMIT: usize = 100;
