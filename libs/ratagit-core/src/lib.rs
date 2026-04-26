mod files;
mod state;

pub use files::{
    FileEntry, FileInputMode, FileRowKind, FileTreeRow, FilesPanelState, ScrollDirection,
    build_file_tree_rows, cancel_search as cancel_file_search,
    clamp_selected as clamp_file_selection, collect_directories,
    confirm_search as confirm_file_search, enter_multi_select, initialize_tree_if_needed,
    jump_search_match, leave_multi_select, move_selected, pop_search_char, push_search_char,
    reconcile_after_items_changed, selected_row, selected_target_paths,
    start_search as start_file_search, toggle_current_row_selection, toggle_selected_directory,
};
pub use state::{
    AppState, BranchEntry, BranchesPanelState, CommitEntry, CommitsPanelState, PanelFocus,
    RepoSnapshot, StashEntry, StashPanelState, StatusPanelState,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    RefreshAll,
    FocusNext,
    FocusPrev,
    FocusPanel { panel: PanelFocus },
    MoveUp,
    MoveDown,
    ToggleSelectedDirectory,
    ToggleSelectedFileStage,
    ToggleFilesMultiSelect,
    ToggleCurrentFileSelection,
    StartFileSearch,
    InputFileSearchChar(char),
    BackspaceFileSearch,
    ConfirmFileSearch,
    CancelFileSearch,
    NextFileSearchMatch,
    PrevFileSearchMatch,
    StageSelectedFile,
    UnstageSelectedFile,
    StashSelectedFiles,
    CreateCommit { message: String },
    CreateBranch { name: String },
    CheckoutSelectedBranch,
    StashPush { message: String },
    StashPopSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitResult {
    Refreshed(RepoSnapshot),
    RefreshFailed {
        error: String,
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
        result: Result<(), String>,
    },
    CheckoutBranch {
        name: String,
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
    StageFiles { paths: Vec<String> },
    UnstageFiles { paths: Vec<String> },
    StashFiles { message: String, paths: Vec<String> },
    DiscardFiles { paths: Vec<String> },
    CreateCommit { message: String },
    CreateBranch { name: String },
    CheckoutBranch { name: String },
    StashPush { message: String },
    StashPop { stash_id: String },
}

pub fn update(state: &mut AppState, action: Action) -> Vec<Command> {
    match action {
        Action::Ui(ui_action) => update_ui(state, ui_action),
        Action::GitResult(git_result) => update_git_result(state, git_result),
    }
}

fn update_ui(state: &mut AppState, action: UiAction) -> Vec<Command> {
    match action {
        UiAction::RefreshAll => vec![Command::RefreshAll],
        UiAction::FocusNext => {
            state.focus = state.focus.next_left();
            state.last_left_focus = state.focus;
            Vec::new()
        }
        UiAction::FocusPrev => {
            state.focus = state.focus.prev_left();
            state.last_left_focus = state.focus;
            Vec::new()
        }
        UiAction::FocusPanel { panel } => {
            state.focus = panel;
            if panel.is_left_panel() {
                state.last_left_focus = panel;
            }
            Vec::new()
        }
        UiAction::MoveUp => {
            move_selection(state, true);
            Vec::new()
        }
        UiAction::MoveDown => {
            move_selection(state, false);
            Vec::new()
        }
        UiAction::ToggleSelectedDirectory => {
            if !toggle_selected_directory(&mut state.files) {
                push_notice(state, "Selected file is not a directory");
            }
            Vec::new()
        }
        UiAction::ToggleSelectedFileStage => {
            let paths = selected_target_paths(&state.files);
            if paths.is_empty() {
                push_notice(state, "No file selected");
                return Vec::new();
            }
            if selected_targets_are_all_staged(state, &paths) {
                vec![Command::UnstageFiles { paths }]
            } else {
                let unstaged_paths = paths
                    .into_iter()
                    .filter(|path| file_staged(state, path) == Some(false))
                    .collect::<Vec<_>>();
                vec![Command::StageFiles {
                    paths: unstaged_paths,
                }]
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
        UiAction::StartFileSearch => {
            start_file_search(&mut state.files);
            Vec::new()
        }
        UiAction::InputFileSearchChar(ch) => {
            push_search_char(&mut state.files, ch);
            Vec::new()
        }
        UiAction::BackspaceFileSearch => {
            pop_search_char(&mut state.files);
            Vec::new()
        }
        UiAction::ConfirmFileSearch => {
            confirm_file_search(&mut state.files);
            Vec::new()
        }
        UiAction::CancelFileSearch => {
            if state.files.mode == FileInputMode::MultiSelect {
                leave_multi_select(&mut state.files);
            } else {
                cancel_file_search(&mut state.files);
            }
            Vec::new()
        }
        UiAction::NextFileSearchMatch => {
            jump_search_match(&mut state.files, false);
            Vec::new()
        }
        UiAction::PrevFileSearchMatch => {
            jump_search_match(&mut state.files, true);
            Vec::new()
        }
        UiAction::StageSelectedFile => {
            let paths = selected_target_paths(&state.files)
                .into_iter()
                .filter(|path| file_staged(state, path) == Some(false))
                .collect::<Vec<_>>();
            if !paths.is_empty() {
                vec![Command::StageFiles { paths }]
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
                vec![Command::UnstageFiles { paths }]
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
                vec![Command::StashFiles {
                    message: "savepoint".to_string(),
                    paths,
                }]
            }
        }
        UiAction::CreateCommit { message } => {
            state.commits.draft_message = message.clone();
            vec![Command::CreateCommit { message }]
        }
        UiAction::CreateBranch { name } => vec![Command::CreateBranch { name }],
        UiAction::CheckoutSelectedBranch => {
            if let Some(branch) = selected_branch_name(state) {
                vec![Command::CheckoutBranch { name: branch }]
            } else {
                push_notice(state, "No branch selected");
                Vec::new()
            }
        }
        UiAction::StashPush { message } => vec![Command::StashPush { message }],
        UiAction::StashPopSelected => {
            if let Some(stash_id) = selected_stash_id(state) {
                vec![Command::StashPop { stash_id }]
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
            apply_snapshot(state, snapshot);
            state.status.refresh_count = state.status.refresh_count.saturating_add(1);
            state.status.last_error = None;
            Vec::new()
        }
        GitResult::RefreshFailed { error } => {
            state.status.last_error = Some(format!("Failed to refresh: {error}"));
            push_notice(state, &format!("Failed to refresh: {error}"));
            Vec::new()
        }
        GitResult::StageFiles { paths, result } => handle_operation_result(
            state,
            result,
            "stage",
            format!("Staged {}", format_paths(&paths)),
            format!("Failed to stage {}", format_paths(&paths)),
        ),
        GitResult::UnstageFiles { paths, result } => handle_operation_result(
            state,
            result,
            "unstage",
            format!("Unstaged {}", format_paths(&paths)),
            format!("Failed to unstage {}", format_paths(&paths)),
        ),
        GitResult::StashFiles {
            message,
            paths,
            result,
        } => handle_operation_result(
            state,
            result,
            "stash_files",
            format!("Stashed {}: {message}", format_paths(&paths)),
            format!("Failed to stash {}", format_paths(&paths)),
        ),
        GitResult::DiscardFiles { paths, result } => handle_operation_result(
            state,
            result,
            "discard_files",
            format!("Discarded {}", format_paths(&paths)),
            format!("Failed to discard {}", format_paths(&paths)),
        ),
        GitResult::CreateCommit { message, result } => handle_operation_result(
            state,
            result,
            "commit",
            format!("Commit created: {message}"),
            "Failed to create commit".to_string(),
        ),
        GitResult::CreateBranch { name, result } => handle_operation_result(
            state,
            result,
            "create_branch",
            format!("Branch created: {name}"),
            format!("Failed to create branch: {name}"),
        ),
        GitResult::CheckoutBranch { name, result } => handle_operation_result(
            state,
            result,
            "checkout_branch",
            format!("Checked out: {name}"),
            format!("Failed to checkout branch: {name}"),
        ),
        GitResult::StashPush { message, result } => handle_operation_result(
            state,
            result,
            "stash_push",
            format!("Stash pushed: {message}"),
            "Failed to stash push".to_string(),
        ),
        GitResult::StashPop { stash_id, result } => handle_operation_result(
            state,
            result,
            "stash_pop",
            format!("Stash popped: {stash_id}"),
            format!("Failed to stash pop: {stash_id}"),
        ),
    }
}

fn handle_operation_result(
    state: &mut AppState,
    result: Result<(), String>,
    operation_key: &str,
    success_message: String,
    failure_prefix: String,
) -> Vec<Command> {
    match result {
        Ok(()) => {
            state.last_operation = Some(operation_key.to_string());
            push_notice(state, &success_message);
            state.status.last_error = None;
            vec![Command::RefreshAll]
        }
        Err(error_message) => {
            state.last_operation = Some(operation_key.to_string());
            let full_error = format!("{failure_prefix}: {error_message}");
            state.status.last_error = Some(full_error.clone());
            push_notice(state, &full_error);
            Vec::new()
        }
    }
}

fn apply_snapshot(state: &mut AppState, snapshot: RepoSnapshot) {
    state.status.summary = snapshot.status_summary;
    state.status.current_branch = snapshot.current_branch;
    state.status.detached_head = snapshot.detached_head;
    state.files.items = snapshot.files;
    initialize_tree_if_needed(&mut state.files);
    reconcile_after_items_changed(&mut state.files);
    state.commits.items = snapshot.commits;
    state.branches.items = snapshot.branches;
    state.stash.items = snapshot.stashes;
    clamp_selection_indexes(state);
}

fn clamp_selection_indexes(state: &mut AppState) {
    clamp_file_selection(&mut state.files);
    state.commits.selected = clamp_index(state.commits.selected, state.commits.items.len());
    state.branches.selected = clamp_index(state.branches.selected, state.branches.items.len());
    state.stash.selected = clamp_index(state.stash.selected, state.stash.items.len());
}

fn clamp_index(index: usize, len: usize) -> usize {
    if len == 0 { 0 } else { index.min(len - 1) }
}

fn move_selection(state: &mut AppState, move_up: bool) {
    match state.focus {
        PanelFocus::Files => {
            move_selected(&mut state.files, move_up);
        }
        PanelFocus::Branches => move_index(
            &mut state.branches.selected,
            state.branches.items.len(),
            move_up,
        ),
        PanelFocus::Commits => move_index(
            &mut state.commits.selected,
            state.commits.items.len(),
            move_up,
        ),
        PanelFocus::Stash => {
            move_index(&mut state.stash.selected, state.stash.items.len(), move_up)
        }
        PanelFocus::Details | PanelFocus::Log => {}
    }
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

fn format_paths(paths: &[String]) -> String {
    match paths {
        [] => "<none>".to_string(),
        [only] => only.clone(),
        _ => format!("{} files", paths.len()),
    }
}

fn selected_branch_name(state: &AppState) -> Option<String> {
    state
        .branches
        .items
        .get(state.branches.selected)
        .map(|branch| branch.name.clone())
}

fn selected_stash_id(state: &AppState) -> Option<String> {
    state
        .stash
        .items
        .get(state.stash.selected)
        .map(|stash| stash.id.clone())
}

fn push_notice(state: &mut AppState, message: &str) {
    state.notices.push(message.to_string());
    if state.notices.len() > 10 {
        let keep_from = state.notices.len() - 10;
        state.notices.drain(0..keep_from);
    }
}
