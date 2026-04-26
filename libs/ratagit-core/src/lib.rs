mod branches;
mod commits;
mod details;
mod files;
mod operations;
mod scroll;
mod search;
mod state;
mod text_edit;

use search::{
    backspace_search, cancel_search, clear_search_if_incompatible, confirm_search,
    input_search_char, jump_search_match, recompute_search_matches, start_search,
};
use text_edit::{CursorMove, backspace_at_cursor, insert_char_at_cursor, move_cursor_in_text};

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
    CommitFileEntry, CommitFileStatus, CommitFilesPanelState, FileEntry, FileInputMode,
    FileRowKind, FileTreeRow, FilesPanelState, build_commit_file_tree_rows, build_file_tree_rows,
    clamp_selected as clamp_file_selection, collect_directories, commit_file_tree_rows,
    enter_multi_select, file_tree_rows, initialize_commit_files_tree, initialize_tree_if_needed,
    leave_multi_select, move_commit_file_selected, move_selected, reconcile_after_items_changed,
    refresh_commit_files_tree_projection, refresh_tree_projection, select_commit_file_tree_path,
    select_file_tree_path, selected_commit_file, selected_commit_file_targets, selected_row,
    selected_target_paths, toggle_commit_files_directory, toggle_current_row_selection,
    toggle_selected_directory,
};
pub use scroll::ScrollDirection;
pub use state::{
    AppState, AutoStashConfirmState, AutoStashOperation, BranchCreateState, BranchDeleteChoice,
    BranchDeleteMenuState, BranchDeleteMode, BranchEntry, BranchForceDeleteConfirmState,
    BranchRebaseChoice, BranchRebaseMenuState, BranchesPanelState, CachedBranchLog,
    CachedCommitDiff, CachedFilesDiff, CommitEditorIntent, CommitEntry, CommitField,
    CommitFileDiffPath, CommitFileDiffTarget, CommitHashStatus, CommitInputMode, CommitsPanelState,
    DetailsPanelState, DiscardConfirmState, EditorKind, EditorState, PanelFocus, RepoSnapshot,
    ResetChoice, ResetMenuState, ResetMode, SearchScope, SearchState, StashEntry, StashPanelState,
    StashScope, StatusPanelState, WorkStatusState,
};

const DETAILS_DIFF_CACHE_LIMIT: usize = 16;
pub const BRANCH_DETAILS_LOG_MAX_COUNT: usize = 50;
pub const COMMITS_PAGE_SIZE: usize = 100;
pub const COMMITS_PREFETCH_THRESHOLD: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    RefreshAll,
    FocusNext,
    FocusPrev,
    FocusPanel { panel: PanelFocus },
    MoveUp,
    MoveDown,
    DetailsScrollUp { lines: usize },
    DetailsScrollDown { lines: usize, visible_lines: usize },
    ToggleSelectedDirectory,
    ToggleSelectedFileStage,
    ToggleFilesMultiSelect,
    ToggleCurrentFileSelection,
    StartSearch,
    InputSearchChar(char),
    BackspaceSearch,
    ConfirmSearch,
    CancelSearch,
    NextSearchMatch,
    PrevSearchMatch,
    StageSelectedFile,
    UnstageSelectedFile,
    StashSelectedFiles,
    OpenCommitEditor,
    OpenStashEditor,
    OpenResetMenu,
    MoveResetMenuUp,
    MoveResetMenuDown,
    ConfirmResetMenu,
    CancelResetMenu,
    OpenDiscardConfirm,
    ConfirmDiscard,
    CancelDiscard,
    EditorInputChar(char),
    EditorBackspace,
    EditorMoveCursorLeft,
    EditorMoveCursorRight,
    EditorMoveCursorHome,
    EditorMoveCursorEnd,
    EditorNextField,
    EditorPrevField,
    EditorInsertNewline,
    EditorConfirm,
    EditorCancel,
    CreateCommit { message: String },
    OpenCommitFilesPanel,
    CloseCommitFilesPanel,
    ToggleCommitFilesDirectory,
    ToggleCommitsMultiSelect,
    SquashSelectedCommits,
    FixupSelectedCommits,
    OpenCommitRewordEditor,
    DeleteSelectedCommits,
    CheckoutSelectedCommitDetached,
    OpenBranchCreateInput,
    BranchCreateInputChar(char),
    BranchCreateBackspace,
    BranchCreateMoveCursorLeft,
    BranchCreateMoveCursorRight,
    BranchCreateMoveCursorHome,
    BranchCreateMoveCursorEnd,
    ConfirmBranchCreate,
    CancelBranchCreate,
    CreateBranch { name: String, start_point: String },
    CheckoutSelectedBranch,
    OpenBranchDeleteMenu,
    MoveBranchDeleteMenuUp,
    MoveBranchDeleteMenuDown,
    ConfirmBranchDeleteMenu,
    CancelBranchDeleteMenu,
    ConfirmBranchForceDelete,
    CancelBranchForceDelete,
    OpenBranchRebaseMenu,
    MoveBranchRebaseMenuUp,
    MoveBranchRebaseMenuDown,
    ConfirmBranchRebaseMenu,
    CancelBranchRebaseMenu,
    ConfirmAutoStash,
    CancelAutoStash,
    StashPush { message: String },
    StashPopSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitResult {
    Refreshed(RepoSnapshot),
    FilesDetailsDiff {
        paths: Vec<String>,
        result: Result<String, String>,
    },
    BranchDetailsLog {
        branch: String,
        result: Result<String, String>,
    },
    CommitDetailsDiff {
        commit_id: String,
        result: Result<String, String>,
    },
    CommitFiles {
        commit_id: String,
        result: Result<Vec<CommitFileEntry>, String>,
    },
    CommitFileDiff {
        target: CommitFileDiffTarget,
        result: Result<String, String>,
    },
    RefreshFailed {
        error: String,
    },
    CommitsPage {
        offset: usize,
        limit: usize,
        epoch: u64,
        result: Result<Vec<CommitEntry>, String>,
    },
    StageFiles {
        paths: Vec<String>,
        result: Result<(), String>,
    },
    UnstageFiles {
        paths: Vec<String>,
        result: Result<(), String>,
    },
    StashFiles {
        message: String,
        paths: Vec<String>,
        result: Result<(), String>,
    },
    Reset {
        mode: ResetMode,
        result: Result<(), String>,
    },
    Nuke {
        result: Result<(), String>,
    },
    DiscardFiles {
        paths: Vec<String>,
        result: Result<(), String>,
    },
    CreateCommit {
        message: String,
        result: Result<(), String>,
    },
    CreateBranch {
        name: String,
        start_point: String,
        result: Result<(), String>,
    },
    CheckoutBranch {
        name: String,
        auto_stash: bool,
        result: Result<(), String>,
    },
    DeleteBranch {
        name: String,
        mode: BranchDeleteMode,
        force: bool,
        result: Result<(), String>,
    },
    RebaseBranch {
        target: String,
        interactive: bool,
        auto_stash: bool,
        result: Result<(), String>,
    },
    SquashCommits {
        commit_ids: Vec<String>,
        result: Result<(), String>,
    },
    FixupCommits {
        commit_ids: Vec<String>,
        result: Result<(), String>,
    },
    RewordCommit {
        commit_id: String,
        message: String,
        result: Result<(), String>,
    },
    DeleteCommits {
        commit_ids: Vec<String>,
        result: Result<(), String>,
    },
    CheckoutCommitDetached {
        commit_id: String,
        auto_stash: bool,
        result: Result<(), String>,
    },
    StashPush {
        message: String,
        result: Result<(), String>,
    },
    StashPop {
        stash_id: String,
        result: Result<(), String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Ui(UiAction),
    GitResult(GitResult),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    RefreshAll,
    LoadMoreCommits {
        offset: usize,
        limit: usize,
        epoch: u64,
    },
    RefreshFilesDetailsDiff {
        paths: Vec<String>,
    },
    RefreshBranchDetailsLog {
        branch: String,
        max_count: usize,
    },
    RefreshCommitDetailsDiff {
        commit_id: String,
    },
    RefreshCommitFiles {
        commit_id: String,
    },
    RefreshCommitFileDiff {
        target: CommitFileDiffTarget,
    },
    StageFiles {
        paths: Vec<String>,
    },
    UnstageFiles {
        paths: Vec<String>,
    },
    StashFiles {
        message: String,
        paths: Vec<String>,
    },
    Reset {
        mode: ResetMode,
    },
    Nuke,
    DiscardFiles {
        paths: Vec<String>,
    },
    CreateCommit {
        message: String,
    },
    CreateBranch {
        name: String,
        start_point: String,
    },
    CheckoutBranch {
        name: String,
        auto_stash: bool,
    },
    DeleteBranch {
        name: String,
        mode: BranchDeleteMode,
        force: bool,
    },
    RebaseBranch {
        target: String,
        interactive: bool,
        auto_stash: bool,
    },
    SquashCommits {
        commit_ids: Vec<String>,
    },
    FixupCommits {
        commit_ids: Vec<String>,
    },
    RewordCommit {
        commit_id: String,
        message: String,
    },
    DeleteCommits {
        commit_ids: Vec<String>,
    },
    CheckoutCommitDetached {
        commit_id: String,
        auto_stash: bool,
    },
    StashPush {
        message: String,
    },
    StashPop {
        stash_id: String,
    },
}

impl Command {
    pub fn debounce_key(&self) -> Option<&'static str> {
        match self {
            Command::RefreshFilesDetailsDiff { .. } => Some("files_details_diff"),
            Command::RefreshBranchDetailsLog { .. } => Some("branch_details_log"),
            Command::RefreshCommitDetailsDiff { .. } => Some("commit_details_diff"),
            Command::RefreshCommitFileDiff { .. } => Some("commit_file_diff"),
            Command::RefreshAll
            | Command::LoadMoreCommits { .. }
            | Command::RefreshCommitFiles { .. }
            | Command::StageFiles { .. }
            | Command::UnstageFiles { .. }
            | Command::StashFiles { .. }
            | Command::Reset { .. }
            | Command::Nuke
            | Command::DiscardFiles { .. }
            | Command::CreateCommit { .. }
            | Command::CreateBranch { .. }
            | Command::CheckoutBranch { .. }
            | Command::DeleteBranch { .. }
            | Command::RebaseBranch { .. }
            | Command::SquashCommits { .. }
            | Command::FixupCommits { .. }
            | Command::RewordCommit { .. }
            | Command::DeleteCommits { .. }
            | Command::CheckoutCommitDetached { .. }
            | Command::StashPush { .. }
            | Command::StashPop { .. } => None,
        }
    }

    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            Command::StageFiles { .. }
                | Command::UnstageFiles { .. }
                | Command::StashFiles { .. }
                | Command::Reset { .. }
                | Command::Nuke
                | Command::DiscardFiles { .. }
                | Command::CreateCommit { .. }
                | Command::CreateBranch { .. }
                | Command::CheckoutBranch { .. }
                | Command::DeleteBranch { .. }
                | Command::RebaseBranch { .. }
                | Command::SquashCommits { .. }
                | Command::FixupCommits { .. }
                | Command::RewordCommit { .. }
                | Command::DeleteCommits { .. }
                | Command::CheckoutCommitDetached { .. }
                | Command::StashPush { .. }
                | Command::StashPop { .. }
        )
    }

    pub fn pending_operation_label(&self) -> Option<String> {
        match self {
            Command::StageFiles { .. } => Some("stage".to_string()),
            Command::UnstageFiles { .. } => Some("unstage".to_string()),
            Command::StashFiles { .. } => Some("stash_files".to_string()),
            Command::Reset { mode } => {
                Some(format!("reset_{}", operations::reset_mode_name(*mode)))
            }
            Command::Nuke => Some("nuke".to_string()),
            Command::DiscardFiles { .. } => Some("discard_files".to_string()),
            Command::CreateCommit { .. } => Some("commit".to_string()),
            Command::CreateBranch { .. } => Some("create_branch".to_string()),
            Command::CheckoutBranch { .. } => Some("checkout_branch".to_string()),
            Command::DeleteBranch { mode, .. } => Some(format!(
                "delete_branch_{}",
                operations::delete_mode_name(*mode)
            )),
            Command::RebaseBranch { interactive, .. } => {
                let mode = if *interactive {
                    "interactive"
                } else {
                    "simple"
                };
                Some(format!("rebase_branch_{mode}"))
            }
            Command::SquashCommits { .. } => Some("squash_commits".to_string()),
            Command::FixupCommits { .. } => Some("fixup_commits".to_string()),
            Command::RewordCommit { .. } => Some("reword_commit".to_string()),
            Command::DeleteCommits { .. } => Some("delete_commits".to_string()),
            Command::CheckoutCommitDetached { .. } => Some("checkout_detached".to_string()),
            Command::StashPush { .. } => Some("stash_push".to_string()),
            Command::StashPop { .. } => Some("stash_pop".to_string()),
            Command::RefreshAll
            | Command::LoadMoreCommits { .. }
            | Command::RefreshFilesDetailsDiff { .. }
            | Command::RefreshBranchDetailsLog { .. }
            | Command::RefreshCommitDetailsDiff { .. }
            | Command::RefreshCommitFiles { .. }
            | Command::RefreshCommitFileDiff { .. } => None,
        }
    }
}

pub fn debounce_key_for_command(command: &Command) -> Option<&'static str> {
    command.debounce_key()
}

pub(crate) fn with_pending(state: &mut AppState, commands: Vec<Command>) -> Vec<Command> {
    for command in &commands {
        mark_command_pending(state, command);
    }
    commands
}

fn mark_command_pending(state: &mut AppState, command: &Command) {
    match command {
        Command::RefreshAll => {
            state.work.refresh_pending = true;
        }
        Command::LoadMoreCommits { .. } => {
            state.commits.loading_more = true;
        }
        Command::RefreshFilesDetailsDiff { .. } => {
            state.work.details_pending = true;
        }
        Command::RefreshBranchDetailsLog { .. } => {
            state.work.details_pending = true;
        }
        Command::RefreshCommitDetailsDiff { .. } => {
            state.work.details_pending = true;
        }
        Command::RefreshCommitFiles { .. } => {
            state.commits.files.loading = true;
        }
        Command::RefreshCommitFileDiff { .. } => {
            state.work.details_pending = true;
        }
        _ => {
            if let Some(label) = command.pending_operation_label() {
                state.work.operation_pending = Some(label);
            }
        }
    }
}

pub fn update(state: &mut AppState, action: Action) -> Vec<Command> {
    match action {
        Action::Ui(ui_action) => update_ui(state, ui_action),
        Action::GitResult(git_result) => update_git_result(state, git_result),
    }
}

fn update_ui(state: &mut AppState, action: UiAction) -> Vec<Command> {
    match action {
        UiAction::RefreshAll => with_pending(state, vec![Command::RefreshAll]),
        UiAction::OpenCommitEditor => {
            open_commit_editor(state);
            Vec::new()
        }
        UiAction::OpenStashEditor => {
            open_stash_editor(state);
            Vec::new()
        }
        UiAction::OpenBranchCreateInput => {
            branches::open_create_input(state);
            Vec::new()
        }
        UiAction::BranchCreateInputChar(ch) => {
            branches::input_create_char(state, ch);
            Vec::new()
        }
        UiAction::BranchCreateBackspace => {
            branches::backspace_create(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorLeft => {
            branches::move_create_cursor_left(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorRight => {
            branches::move_create_cursor_right(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorHome => {
            branches::move_create_cursor_home(state);
            Vec::new()
        }
        UiAction::BranchCreateMoveCursorEnd => {
            branches::move_create_cursor_end(state);
            Vec::new()
        }
        UiAction::ConfirmBranchCreate => branches::confirm_create(state),
        UiAction::CancelBranchCreate => {
            branches::close_create_input(state);
            Vec::new()
        }
        UiAction::OpenBranchDeleteMenu => {
            branches::open_delete_menu(state);
            Vec::new()
        }
        UiAction::MoveBranchDeleteMenuUp => {
            state.branches.delete_menu.selected = state.branches.delete_menu.selected.prev();
            Vec::new()
        }
        UiAction::MoveBranchDeleteMenuDown => {
            state.branches.delete_menu.selected = state.branches.delete_menu.selected.next();
            Vec::new()
        }
        UiAction::ConfirmBranchDeleteMenu => branches::confirm_delete_menu(state),
        UiAction::CancelBranchDeleteMenu => {
            branches::close_delete_menu(state);
            Vec::new()
        }
        UiAction::ConfirmBranchForceDelete => branches::confirm_force_delete(state),
        UiAction::CancelBranchForceDelete => {
            branches::close_force_delete_confirm(state);
            Vec::new()
        }
        UiAction::OpenBranchRebaseMenu => {
            branches::open_rebase_menu(state);
            Vec::new()
        }
        UiAction::MoveBranchRebaseMenuUp => {
            state.branches.rebase_menu.selected = state.branches.rebase_menu.selected.prev();
            Vec::new()
        }
        UiAction::MoveBranchRebaseMenuDown => {
            state.branches.rebase_menu.selected = state.branches.rebase_menu.selected.next();
            Vec::new()
        }
        UiAction::ConfirmBranchRebaseMenu => branches::confirm_rebase_menu(state),
        UiAction::CancelBranchRebaseMenu => {
            branches::close_rebase_menu(state);
            Vec::new()
        }
        UiAction::ConfirmAutoStash => branches::confirm_auto_stash(state),
        UiAction::CancelAutoStash => {
            branches::close_auto_stash_confirm(state);
            Vec::new()
        }
        UiAction::OpenResetMenu => {
            open_reset_menu(state);
            Vec::new()
        }
        UiAction::MoveResetMenuUp => {
            state.reset_menu.selected = state.reset_menu.selected.prev();
            Vec::new()
        }
        UiAction::MoveResetMenuDown => {
            state.reset_menu.selected = state.reset_menu.selected.next();
            Vec::new()
        }
        UiAction::ConfirmResetMenu => confirm_reset_menu(state),
        UiAction::CancelResetMenu => {
            state.reset_menu.active = false;
            Vec::new()
        }
        UiAction::OpenDiscardConfirm => {
            open_discard_confirm(state);
            Vec::new()
        }
        UiAction::ConfirmDiscard => confirm_discard(state),
        UiAction::CancelDiscard => {
            close_discard_confirm(state);
            Vec::new()
        }
        UiAction::EditorInputChar(ch) => {
            apply_editor_input_char(state, ch);
            Vec::new()
        }
        UiAction::EditorBackspace => {
            apply_editor_backspace(state);
            Vec::new()
        }
        UiAction::EditorMoveCursorLeft => {
            move_editor_cursor(state, CursorMove::Left);
            Vec::new()
        }
        UiAction::EditorMoveCursorRight => {
            move_editor_cursor(state, CursorMove::Right);
            Vec::new()
        }
        UiAction::EditorMoveCursorHome => {
            move_editor_cursor(state, CursorMove::Home);
            Vec::new()
        }
        UiAction::EditorMoveCursorEnd => {
            move_editor_cursor(state, CursorMove::End);
            Vec::new()
        }
        UiAction::EditorNextField => {
            switch_editor_field(state, false);
            Vec::new()
        }
        UiAction::EditorPrevField => {
            switch_editor_field(state, true);
            Vec::new()
        }
        UiAction::EditorInsertNewline => {
            apply_editor_newline(state);
            Vec::new()
        }
        UiAction::EditorConfirm => confirm_editor(state),
        UiAction::EditorCancel => {
            state.editor.kind = None;
            Vec::new()
        }
        UiAction::FocusNext => {
            state.focus = state.focus.next_left();
            state.last_left_focus = state.focus;
            clear_search_if_incompatible(state);
            details::refresh_on_focus(state)
        }
        UiAction::FocusPrev => {
            state.focus = state.focus.prev_left();
            state.last_left_focus = state.focus;
            clear_search_if_incompatible(state);
            details::refresh_on_focus(state)
        }
        UiAction::FocusPanel { panel } => {
            state.focus = panel;
            if panel.is_left_panel() {
                state.last_left_focus = panel;
            }
            clear_search_if_incompatible(state);
            details::refresh_on_focus(state)
        }
        UiAction::MoveUp => {
            let mut commands = move_selection(state, true);
            commands.extend(details::refresh_on_navigation(state));
            commands
        }
        UiAction::MoveDown => {
            let mut commands = move_selection(state, false);
            commands.extend(details::refresh_on_navigation(state));
            commands
        }
        UiAction::DetailsScrollUp { lines } => {
            details::scroll_up(state, lines);
            Vec::new()
        }
        UiAction::DetailsScrollDown {
            lines,
            visible_lines,
        } => {
            details::scroll_down(state, lines, visible_lines);
            Vec::new()
        }
        UiAction::ToggleSelectedDirectory => {
            if toggle_selected_directory(&mut state.files) {
                details::refresh_files_details(state)
            } else {
                push_notice(state, "Selected file is not a directory");
                Vec::new()
            }
        }
        UiAction::ToggleSelectedFileStage => {
            let paths = selected_target_paths(&state.files);
            if paths.is_empty() {
                push_notice(state, "No file selected");
                return Vec::new();
            }
            if selected_targets_are_all_staged(state, &paths) {
                with_pending(state, vec![Command::UnstageFiles { paths }])
            } else {
                let unstaged_paths = paths
                    .into_iter()
                    .filter(|path| file_staged(state, path) == Some(false))
                    .collect::<Vec<_>>();
                with_pending(
                    state,
                    vec![Command::StageFiles {
                        paths: unstaged_paths,
                    }],
                )
            }
        }
        UiAction::ToggleFilesMultiSelect => {
            if state.files.mode == FileInputMode::MultiSelect {
                leave_multi_select(&mut state.files);
            } else {
                enter_multi_select(&mut state.files);
            }
            Vec::new()
        }
        UiAction::ToggleCurrentFileSelection => {
            toggle_current_row_selection(&mut state.files);
            Vec::new()
        }
        UiAction::StartSearch => {
            start_search(state);
            Vec::new()
        }
        UiAction::InputSearchChar(ch) => {
            input_search_char(state, ch);
            Vec::new()
        }
        UiAction::BackspaceSearch => {
            backspace_search(state);
            Vec::new()
        }
        UiAction::ConfirmSearch => {
            if confirm_search(state) {
                details::refresh_for_focus(state)
            } else {
                Vec::new()
            }
        }
        UiAction::CancelSearch => {
            cancel_search(state);
            Vec::new()
        }
        UiAction::NextSearchMatch => {
            if jump_search_match(state, false) {
                details::refresh_for_focus(state)
            } else {
                Vec::new()
            }
        }
        UiAction::PrevSearchMatch => {
            if jump_search_match(state, true) {
                details::refresh_for_focus(state)
            } else {
                Vec::new()
            }
        }
        UiAction::StageSelectedFile => {
            let paths = selected_target_paths(&state.files)
                .into_iter()
                .filter(|path| file_staged(state, path) == Some(false))
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                with_pending(state, vec![Command::StageFiles { paths }])
            } else {
                push_notice(state, "No unstaged file selected");
                Vec::new()
            }
        }
        UiAction::UnstageSelectedFile => {
            let paths = selected_target_paths(&state.files)
                .into_iter()
                .filter(|path| file_staged(state, path) == Some(true))
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                with_pending(state, vec![Command::UnstageFiles { paths }])
            } else {
                push_notice(state, "No staged file selected");
                Vec::new()
            }
        }
        UiAction::StashSelectedFiles => {
            let paths = selected_target_paths(&state.files);
            if paths.is_empty() {
                push_notice(state, "No file selected");
                Vec::new()
            } else {
                with_pending(
                    state,
                    vec![Command::StashFiles {
                        message: "savepoint".to_string(),
                        paths,
                    }],
                )
            }
        }
        UiAction::CreateCommit { message } => {
            state.commits.draft_message = message.clone();
            with_pending(state, vec![Command::CreateCommit { message }])
        }
        UiAction::OpenCommitFilesPanel => open_commit_files_panel(state),
        UiAction::CloseCommitFilesPanel => close_commit_files_panel(state),
        UiAction::ToggleCommitFilesDirectory => {
            if toggle_commit_files_directory(&mut state.commits.files) {
                details::refresh_commit_file_diff(state)
            } else {
                push_notice(state, "Selected commit file is not a directory");
                Vec::new()
            }
        }
        UiAction::ToggleCommitsMultiSelect => {
            toggle_commit_multi_select(&mut state.commits);
            Vec::new()
        }
        UiAction::SquashSelectedCommits => {
            rewrite_selected_commits(state, CommitRewriteKind::Squash, "squash", |commit_ids| {
                Command::SquashCommits { commit_ids }
            })
        }
        UiAction::FixupSelectedCommits => {
            rewrite_selected_commits(state, CommitRewriteKind::Fixup, "fixup", |commit_ids| {
                Command::FixupCommits { commit_ids }
            })
        }
        UiAction::OpenCommitRewordEditor => {
            open_commit_reword_editor(state);
            Vec::new()
        }
        UiAction::DeleteSelectedCommits => {
            rewrite_selected_commits(state, CommitRewriteKind::Delete, "delete", |commit_ids| {
                Command::DeleteCommits { commit_ids }
            })
        }
        UiAction::CheckoutSelectedCommitDetached => checkout_selected_commit_detached(state),
        UiAction::CreateBranch { name, start_point } => {
            with_pending(state, vec![Command::CreateBranch { name, start_point }])
        }
        UiAction::CheckoutSelectedBranch => branches::checkout_selected(state),
        UiAction::StashPush { message } => {
            with_pending(state, vec![Command::StashPush { message }])
        }
        UiAction::StashPopSelected => {
            if let Some(stash_id) = selected_stash_id(state) {
                with_pending(state, vec![Command::StashPop { stash_id }])
            } else {
                push_notice(state, "No stash selected");
                Vec::new()
            }
        }
    }
}

fn update_git_result(state: &mut AppState, result: GitResult) -> Vec<Command> {
    match result {
        GitResult::Refreshed(snapshot) => {
            state.work.refresh_pending = false;
            state.work.last_completed_command = Some("refresh".to_string());
            apply_snapshot(state, snapshot);
            state.status.refresh_count = state.status.refresh_count.saturating_add(1);
            state.status.last_error = None;
            details::refresh_for_focus(state)
        }
        GitResult::CommitsPage {
            offset,
            limit,
            epoch,
            result,
        } => handle_commits_page_result(state, offset, limit, epoch, result),
        GitResult::FilesDetailsDiff { paths, result } => {
            details::apply_files_diff_result(state, paths, result)
        }
        GitResult::BranchDetailsLog { branch, result } => {
            details::apply_branch_log_result(state, branch, result)
        }
        GitResult::CommitDetailsDiff { commit_id, result } => {
            details::apply_commit_diff_result(state, commit_id, result)
        }
        GitResult::CommitFiles { commit_id, result } => {
            if !state.commits.files.active
                || state.commits.files.commit_id.as_deref() != Some(commit_id.as_str())
            {
                return Vec::new();
            }
            state.commits.files.loading = false;
            state.work.last_completed_command = Some("commit_files".to_string());
            match result {
                Ok(files) => {
                    state.commits.files.items = files;
                    state.commits.files.selected = 0;
                    state.commits.files.scroll_direction = None;
                    state.commits.files.scroll_direction_origin = 0;
                    initialize_commit_files_tree(&mut state.commits.files);
                    if state.search.scope == Some(SearchScope::CommitFiles)
                        && !state.search.query.is_empty()
                    {
                        recompute_search_matches(state);
                    }
                    state.status.last_error = None;
                    details::refresh_commit_file_diff(state)
                }
                Err(error) => {
                    let message = format!("Failed to refresh commit files: {error}");
                    state.status.last_error = Some(message.clone());
                    state.details.commit_file_diff.clear();
                    state.details.commit_file_diff_error = Some(message.clone());
                    push_notice(state, &message);
                    Vec::new()
                }
            }
        }
        GitResult::CommitFileDiff { target, result } => {
            details::apply_commit_file_diff_result(state, target, result)
        }
        GitResult::RefreshFailed { error } => {
            state.work.refresh_pending = false;
            state.work.last_completed_command = Some("refresh".to_string());
            state.status.last_error = Some(format!("Failed to refresh: {error}"));
            push_notice(state, &format!("Failed to refresh: {error}"));
            Vec::new()
        }
        GitResult::StageFiles { paths, result } => {
            operations::handle_stage_files_result(state, paths, result)
        }
        GitResult::UnstageFiles { paths, result } => {
            operations::handle_unstage_files_result(state, paths, result)
        }
        GitResult::StashFiles {
            message,
            paths,
            result,
        } => operations::handle_stash_files_result(state, message, paths, result),
        GitResult::Reset { mode, result } => operations::handle_reset_result(state, mode, result),
        GitResult::Nuke { result } => operations::handle_nuke_result(state, result),
        GitResult::DiscardFiles { paths, result } => {
            operations::handle_discard_files_result(state, paths, result)
        }
        GitResult::CreateCommit { message, result } => {
            operations::handle_create_commit_result(state, message, result)
        }
        GitResult::CreateBranch {
            name,
            start_point,
            result,
        } => operations::handle_create_branch_result(state, name, start_point, result),
        GitResult::CheckoutBranch {
            name,
            auto_stash,
            result,
        } => operations::handle_checkout_branch_result(state, name, auto_stash, result),
        GitResult::DeleteBranch {
            name,
            mode,
            force,
            result,
        } => operations::handle_delete_branch_result(state, name, mode, force, result),
        GitResult::RebaseBranch {
            target,
            interactive,
            auto_stash,
            result,
        } => {
            operations::handle_rebase_branch_result(state, target, interactive, auto_stash, result)
        }
        GitResult::SquashCommits { commit_ids, result } => {
            operations::handle_squash_commits_result(state, commit_ids, result)
        }
        GitResult::FixupCommits { commit_ids, result } => {
            operations::handle_fixup_commits_result(state, commit_ids, result)
        }
        GitResult::RewordCommit {
            commit_id,
            message,
            result,
        } => operations::handle_reword_commit_result(state, commit_id, message, result),
        GitResult::DeleteCommits { commit_ids, result } => {
            operations::handle_delete_commits_result(state, commit_ids, result)
        }
        GitResult::CheckoutCommitDetached {
            commit_id,
            auto_stash,
            result,
        } => {
            operations::handle_checkout_commit_detached_result(state, commit_id, auto_stash, result)
        }
        GitResult::StashPush { message, result } => {
            operations::handle_stash_push_result(state, message, result)
        }
        GitResult::StashPop { stash_id, result } => {
            operations::handle_stash_pop_result(state, stash_id, result)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommitRewriteKind {
    Squash,
    Fixup,
    Delete,
}

fn rewrite_selected_commits(
    state: &mut AppState,
    _kind: CommitRewriteKind,
    action_label: &str,
    command: impl FnOnce(Vec<String>) -> Command,
) -> Vec<Command> {
    if repository_has_uncommitted_changes(state) {
        push_notice(state, "Commit rewrite requires a clean working tree");
        return Vec::new();
    }
    let commits = selected_commits(&state.commits);
    if commits.is_empty() {
        push_notice(state, "No commit selected");
        return Vec::new();
    }
    if commits
        .iter()
        .any(|commit| commit.hash_status != CommitHashStatus::Unpushed)
    {
        push_notice(state, "Commit rewrite only supports unpushed commits");
        return Vec::new();
    }
    if commits.iter().any(|commit| commit.is_merge) {
        push_notice(state, "Commit rewrite does not support merge commits yet");
        return Vec::new();
    }
    let commit_ids = selected_commit_ids(&state.commits);
    if commit_ids.is_empty() {
        push_notice(state, "No commit selected");
        return Vec::new();
    }
    if state.commits.mode == CommitInputMode::MultiSelect {
        leave_commit_multi_select(&mut state.commits);
    }
    push_notice(
        state,
        &format!(
            "Queued {action_label} for {}",
            operations::format_commit_count(commit_ids.len())
        ),
    );
    with_pending(state, vec![command(commit_ids)])
}

fn open_commit_reword_editor(state: &mut AppState) {
    if state.commits.mode == CommitInputMode::MultiSelect {
        push_notice(state, "Reword supports one commit at a time");
        return;
    }
    if repository_has_uncommitted_changes(state) {
        push_notice(state, "Commit rewrite requires a clean working tree");
        return;
    }
    let Some(commit) = selected_commit(&state.commits) else {
        push_notice(state, "No commit selected");
        return;
    };
    if commit.is_merge {
        push_notice(state, "Commit rewrite does not support merge commits yet");
        return;
    }
    if commit.hash_status != CommitHashStatus::Unpushed {
        push_notice(state, "Commit rewrite only supports unpushed commits");
        return;
    }
    state.reset_menu.active = false;
    close_discard_confirm(state);
    branches::close_popovers(state);
    let (message, body) = split_commit_message(&commit.message);
    state.editor.kind = Some(EditorKind::Commit {
        message_cursor: message.len(),
        body_cursor: body.len(),
        message,
        body,
        active_field: CommitField::Message,
        intent: CommitEditorIntent::Reword {
            commit_id: commit_key(&commit),
        },
    });
}

fn checkout_selected_commit_detached(state: &mut AppState) -> Vec<Command> {
    if state.commits.mode == CommitInputMode::MultiSelect {
        push_notice(state, "Detached checkout supports one commit at a time");
        return Vec::new();
    }
    let Some(commit) = selected_commit(&state.commits) else {
        push_notice(state, "No commit selected");
        return Vec::new();
    };
    let commit_id = commit_key(&commit);
    if repository_has_uncommitted_changes(state) {
        branches::open_auto_stash_confirm(
            state,
            AutoStashOperation::CheckoutCommitDetached { commit_id },
        );
        return Vec::new();
    }
    with_pending(
        state,
        vec![Command::CheckoutCommitDetached {
            commit_id,
            auto_stash: false,
        }],
    )
}

fn open_commit_editor(state: &mut AppState) {
    state.reset_menu.active = false;
    close_discard_confirm(state);
    branches::close_popovers(state);
    state.editor.kind = Some(EditorKind::Commit {
        message: String::new(),
        message_cursor: 0,
        body: String::new(),
        body_cursor: 0,
        active_field: CommitField::Message,
        intent: CommitEditorIntent::Create,
    });
}

fn open_stash_editor(state: &mut AppState) {
    state.reset_menu.active = false;
    close_discard_confirm(state);
    branches::close_popovers(state);
    state.editor.kind = Some(EditorKind::Stash {
        title: String::new(),
        title_cursor: 0,
        scope: stash_scope_for_current_files_selection(state),
    });
}

fn stash_scope_for_current_files_selection(state: &AppState) -> StashScope {
    if state.files.mode == FileInputMode::MultiSelect {
        let paths = selected_target_paths(&state.files);
        if !paths.is_empty() {
            return StashScope::SelectedPaths(paths);
        }
    }
    StashScope::All
}

fn open_reset_menu(state: &mut AppState) {
    state.editor.kind = None;
    close_discard_confirm(state);
    branches::close_popovers(state);
    state.reset_menu.active = true;
    state.reset_menu.selected = ResetChoice::Mixed;
}

fn confirm_reset_menu(state: &mut AppState) -> Vec<Command> {
    if !state.reset_menu.active {
        return Vec::new();
    }
    let choice = state.reset_menu.selected;
    state.reset_menu.active = false;
    if choice == ResetChoice::Nuke {
        with_pending(state, vec![Command::Nuke])
    } else if let Some(mode) = choice.reset_mode() {
        with_pending(state, vec![Command::Reset { mode }])
    } else {
        Vec::new()
    }
}

fn open_discard_confirm(state: &mut AppState) {
    let paths = selected_target_paths(&state.files);
    if paths.is_empty() {
        push_notice(state, "No file selected");
        return;
    }

    state.editor.kind = None;
    state.reset_menu.active = false;
    branches::close_popovers(state);
    state.discard_confirm.active = true;
    state.discard_confirm.paths = paths;
}

fn confirm_discard(state: &mut AppState) -> Vec<Command> {
    if !state.discard_confirm.active {
        return Vec::new();
    }
    let paths = state.discard_confirm.paths.clone();
    close_discard_confirm(state);
    if paths.is_empty() {
        push_notice(state, "No file selected");
        Vec::new()
    } else {
        with_pending(state, vec![Command::DiscardFiles { paths }])
    }
}

fn close_discard_confirm(state: &mut AppState) {
    state.discard_confirm.active = false;
    state.discard_confirm.paths.clear();
}

fn apply_editor_input_char(state: &mut AppState, ch: char) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => match active_field {
            CommitField::Message => insert_char_at_cursor(message, message_cursor, ch),
            CommitField::Body => insert_char_at_cursor(body, body_cursor, ch),
        },
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => insert_char_at_cursor(title, title_cursor, ch),
    }
}

fn apply_editor_backspace(state: &mut AppState) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => match active_field {
            CommitField::Message => backspace_at_cursor(message, message_cursor),
            CommitField::Body => backspace_at_cursor(body, body_cursor),
        },
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => backspace_at_cursor(title, title_cursor),
    }
}

fn move_editor_cursor(state: &mut AppState, movement: CursorMove) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    match editor {
        EditorKind::Commit {
            message,
            message_cursor,
            body,
            body_cursor,
            active_field,
            ..
        } => match active_field {
            CommitField::Message => move_cursor_in_text(message, message_cursor, movement),
            CommitField::Body => move_cursor_in_text(body, body_cursor, movement),
        },
        EditorKind::Stash {
            title,
            title_cursor,
            ..
        } => move_cursor_in_text(title, title_cursor, movement),
    }
}

fn switch_editor_field(state: &mut AppState, previous: bool) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    if let EditorKind::Commit { active_field, .. } = editor {
        *active_field = if previous {
            active_field.prev()
        } else {
            active_field.next()
        };
    }
}

fn apply_editor_newline(state: &mut AppState) {
    let Some(editor) = state.editor.kind.as_mut() else {
        return;
    };

    if let EditorKind::Commit {
        body,
        body_cursor,
        active_field: CommitField::Body,
        ..
    } = editor
    {
        insert_char_at_cursor(body, body_cursor, '\n');
    }
}

fn confirm_editor(state: &mut AppState) -> Vec<Command> {
    let Some(editor) = state.editor.kind.clone() else {
        return Vec::new();
    };

    match editor {
        EditorKind::Commit {
            message,
            body,
            intent,
            ..
        } => {
            if message.trim().is_empty() {
                push_notice(state, "Commit message cannot be empty");
                return Vec::new();
            }

            let commit_message = build_commit_message(&message, &body);
            state.commits.draft_message = message.trim().to_string();
            state.editor.kind = None;
            match intent {
                CommitEditorIntent::Create => with_pending(
                    state,
                    vec![Command::CreateCommit {
                        message: commit_message,
                    }],
                ),
                CommitEditorIntent::Reword { commit_id } => with_pending(
                    state,
                    vec![Command::RewordCommit {
                        commit_id,
                        message: commit_message,
                    }],
                ),
            }
        }
        EditorKind::Stash { title, scope, .. } => match scope {
            StashScope::All => {
                state.editor.kind = None;
                with_pending(state, vec![Command::StashPush { message: title }])
            }
            StashScope::SelectedPaths(paths) => {
                if paths.is_empty() {
                    push_notice(state, "No file selected");
                    return Vec::new();
                }

                state.editor.kind = None;
                with_pending(
                    state,
                    vec![Command::StashFiles {
                        message: title,
                        paths,
                    }],
                )
            }
        },
    }
}

fn build_commit_message(subject: &str, body: &str) -> String {
    let clean_subject = subject.trim();
    let clean_body = body.trim_end();
    if clean_body.is_empty() {
        clean_subject.to_string()
    } else {
        format!("{clean_subject}\n\n{clean_body}")
    }
}

fn split_commit_message(message: &str) -> (String, String) {
    let clean = message.trim_end();
    let mut parts = clean.splitn(2, '\n');
    let subject = parts.next().unwrap_or("").trim().to_string();
    let remainder = parts.next().unwrap_or("");
    let body = remainder.strip_prefix('\n').unwrap_or(remainder);
    let body = body.trim_end().to_string();
    (subject, body)
}

fn apply_snapshot(state: &mut AppState, snapshot: RepoSnapshot) {
    state.status.summary = snapshot.status_summary;
    state.status.current_branch = snapshot.current_branch;
    state.status.detached_head = snapshot.detached_head;
    state.files.items = snapshot.files;
    initialize_tree_if_needed(&mut state.files);
    reconcile_after_items_changed(&mut state.files);
    state.commits.items = snapshot.commits;
    state.commits.files = CommitFilesPanelState::default();
    state.commits.has_more = state.commits.items.len() >= COMMITS_PAGE_SIZE;
    state.commits.loading_more = false;
    state.commits.pending_select_after_load = false;
    state.commits.pagination_epoch = state.commits.pagination_epoch.wrapping_add(1);
    reconcile_commits_after_items_changed(&mut state.commits);
    state.branches.items = snapshot.branches;
    state.stash.items = snapshot.stashes;
    clamp_selection_indexes(state);
    details::reset_after_snapshot(state);
    state.search.clear();
}

fn load_more_commits_command(state: &mut AppState, select_first_new: bool) -> Vec<Command> {
    if !state.commits.has_more || state.commits.items.is_empty() {
        return Vec::new();
    }
    if state.commits.loading_more {
        state.commits.pending_select_after_load |= select_first_new;
        return Vec::new();
    }
    state.commits.pending_select_after_load |= select_first_new;
    with_pending(
        state,
        vec![Command::LoadMoreCommits {
            offset: state.commits.items.len(),
            limit: COMMITS_PAGE_SIZE,
            epoch: state.commits.pagination_epoch,
        }],
    )
}

fn handle_commits_page_result(
    state: &mut AppState,
    offset: usize,
    limit: usize,
    epoch: u64,
    result: Result<Vec<CommitEntry>, String>,
) -> Vec<Command> {
    if epoch != state.commits.pagination_epoch {
        return Vec::new();
    }
    state.commits.loading_more = false;
    state.work.last_completed_command = Some("load_more_commits".to_string());
    match result {
        Ok(mut commits) => {
            if offset != state.commits.items.len() {
                state.commits.pending_select_after_load = false;
                return Vec::new();
            }
            let first_new_index = state.commits.items.len();
            let loaded = commits.len();
            state.commits.items.append(&mut commits);
            state.commits.has_more = loaded >= limit;
            if state.commits.pending_select_after_load && loaded > 0 {
                state.commits.selected = first_new_index;
            }
            state.commits.pending_select_after_load = false;
            state.status.last_error = None;
            reconcile_commits_after_items_appended(&mut state.commits);
        }
        Err(error) => {
            state.commits.pending_select_after_load = false;
            let message = format!("Failed to load more commits: {error}");
            state.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

fn clamp_selection_indexes(state: &mut AppState) {
    clamp_file_selection(&mut state.files);
    clamp_commit_selection(&mut state.commits);
    state.branches.selected = clamp_index(state.branches.selected, state.branches.items.len());
    state.stash.selected = clamp_index(state.stash.selected, state.stash.items.len());
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}

fn move_selection(state: &mut AppState, move_up: bool) -> Vec<Command> {
    match state.focus {
        PanelFocus::Files => {
            move_selected(&mut state.files, move_up);
            Vec::new()
        }
        PanelFocus::Branches => {
            move_index(
                &mut state.branches.selected,
                state.branches.items.len(),
                move_up,
            );
            Vec::new()
        }
        PanelFocus::Commits => {
            if state.commits.files.active {
                move_commit_file_selected(&mut state.commits.files, move_up);
                return Vec::new();
            }
            let was_at_loaded_end = !move_up
                && !state.commits.items.is_empty()
                && state.commits.selected + 1 >= state.commits.items.len();
            move_commit_selected(&mut state.commits, move_up);
            if was_at_loaded_end {
                load_more_commits_command(state, true)
            } else if should_prefetch_commits(state, move_up) {
                load_more_commits_command(state, false)
            } else {
                Vec::new()
            }
        }
        PanelFocus::Stash => {
            move_index(&mut state.stash.selected, state.stash.items.len(), move_up);
            Vec::new()
        }
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
}

fn should_prefetch_commits(state: &AppState, move_up: bool) -> bool {
    !move_up
        && state.commits.has_more
        && !state.commits.items.is_empty()
        && state
            .commits
            .items
            .len()
            .saturating_sub(1)
            .saturating_sub(state.commits.selected)
            <= COMMITS_PREFETCH_THRESHOLD
}

fn move_index(selected: &mut usize, len: usize, move_up: bool) {
    if len == 0 {
        *selected = 0;
        return;
    }
    if move_up {
        *selected = selected.saturating_sub(1);
    } else {
        *selected = (*selected + 1).min(len - 1);
    }
}

fn open_commit_files_panel(state: &mut AppState) -> Vec<Command> {
    let Some(commit_id) = selected_commit_id(state) else {
        push_notice(state, "No commit selected");
        return Vec::new();
    };
    state.commits.files = CommitFilesPanelState {
        active: true,
        commit_id: Some(commit_id.clone()),
        loading: true,
        ..CommitFilesPanelState::default()
    };
    clear_search_if_incompatible(state);
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_target = None;
    state.details.commit_file_diff_error = None;
    details::reset_scroll(state);
    with_pending(state, vec![Command::RefreshCommitFiles { commit_id }])
}

fn close_commit_files_panel(state: &mut AppState) -> Vec<Command> {
    if !state.commits.files.active {
        return Vec::new();
    }
    state.commits.files.active = false;
    state.commits.files.loading = false;
    clear_search_if_incompatible(state);
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_target = None;
    state.details.commit_file_diff_error = None;
    state.work.details_pending = false;
    details::refresh_commit_diff(state)
}

fn selected_branch_name(state: &AppState) -> Option<String> {
    state
        .branches
        .items
        .get(state.branches.selected)
        .map(|branch| branch.name.clone())
}

fn selected_commit_id(state: &AppState) -> Option<String> {
    selected_commit(&state.commits).map(|commit| commit_key(&commit))
}

fn selected_targets_are_all_staged(state: &AppState, paths: &[String]) -> bool {
    !paths.is_empty()
        && paths
            .iter()
            .all(|path| file_staged(state, path).unwrap_or(false))
}

fn file_staged(state: &AppState, path: &str) -> Option<bool> {
    state
        .files
        .items
        .iter()
        .find(|entry| entry.path == path)
        .map(|entry| entry.staged)
}

fn repository_has_uncommitted_changes(state: &AppState) -> bool {
    !state.files.items.is_empty()
}

fn selected_stash_id(state: &AppState) -> Option<String> {
    state
        .stash
        .items
        .get(state.stash.selected)
        .map(|stash| stash.id.clone())
}

pub(crate) fn push_notice(state: &mut AppState, message: &str) {
    state.notices.push(message.to_string());
    if state.notices.len() > 10 {
        let keep_from = state.notices.len() - 10;
        state.notices.drain(0..keep_from);
    }
}
