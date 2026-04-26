mod files;
mod state;

pub use files::{
    FileEntry, FileInputMode, FileRowKind, FileTreeRow, FilesPanelState, ScrollDirection,
    build_file_tree_rows, cancel_search as cancel_file_search,
    clamp_selected as clamp_file_selection, collect_directories,
    confirm_search as confirm_file_search, enter_multi_select, file_tree_rows,
    initialize_tree_if_needed, jump_search_match, leave_multi_select, move_selected,
    pop_search_char, push_search_char, reconcile_after_items_changed, refresh_tree_projection,
    selected_row, selected_target_paths, start_search as start_file_search,
    toggle_current_row_selection, toggle_selected_directory,
};
pub use state::{
    AppState, BranchEntry, BranchesPanelState, CachedFilesDiff, CommitEntry, CommitField,
    CommitsPanelState, DetailsPanelState, DiscardConfirmState, EditorKind, EditorState, PanelFocus,
    RepoSnapshot, ResetChoice, ResetMenuState, ResetMode, StashEntry, StashPanelState, StashScope,
    StatusPanelState, WorkStatusState,
};

const DETAILS_DIFF_CACHE_LIMIT: usize = 16;

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
    CreateBranch { name: String },
    CheckoutSelectedBranch,
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
    RefreshFilesDetailsDiff { paths: Vec<String> },
    StageFiles { paths: Vec<String> },
    UnstageFiles { paths: Vec<String> },
    StashFiles { message: String, paths: Vec<String> },
    Reset { mode: ResetMode },
    Nuke,
    DiscardFiles { paths: Vec<String> },
    CreateCommit { message: String },
    CreateBranch { name: String },
    CheckoutBranch { name: String },
    StashPush { message: String },
    StashPop { stash_id: String },
}

pub fn debounce_key_for_command(command: &Command) -> Option<&'static str> {
    match command {
        Command::RefreshFilesDetailsDiff { .. } => Some("files_details_diff"),
        Command::RefreshAll
        | Command::StageFiles { .. }
        | Command::UnstageFiles { .. }
        | Command::StashFiles { .. }
        | Command::Reset { .. }
        | Command::Nuke
        | Command::DiscardFiles { .. }
        | Command::CreateCommit { .. }
        | Command::CreateBranch { .. }
        | Command::CheckoutBranch { .. }
        | Command::StashPush { .. }
        | Command::StashPop { .. } => None,
    }
}

fn with_pending(state: &mut AppState, commands: Vec<Command>) -> Vec<Command> {
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
        Command::RefreshFilesDetailsDiff { .. } => {
            state.work.details_pending = true;
        }
        Command::StageFiles { .. } => {
            state.work.operation_pending = Some("stage".to_string());
        }
        Command::UnstageFiles { .. } => {
            state.work.operation_pending = Some("unstage".to_string());
        }
        Command::StashFiles { .. } => {
            state.work.operation_pending = Some("stash_files".to_string());
        }
        Command::Reset { mode } => {
            state.work.operation_pending = Some(format!("reset_{}", reset_mode_name(*mode)));
        }
        Command::Nuke => {
            state.work.operation_pending = Some("nuke".to_string());
        }
        Command::DiscardFiles { .. } => {
            state.work.operation_pending = Some("discard_files".to_string());
        }
        Command::CreateCommit { .. } => {
            state.work.operation_pending = Some("commit".to_string());
        }
        Command::CreateBranch { .. } => {
            state.work.operation_pending = Some("create_branch".to_string());
        }
        Command::CheckoutBranch { .. } => {
            state.work.operation_pending = Some("checkout_branch".to_string());
        }
        Command::StashPush { .. } => {
            state.work.operation_pending = Some("stash_push".to_string());
        }
        Command::StashPop { .. } => {
            state.work.operation_pending = Some("stash_pop".to_string());
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
            move_editor_cursor(state, EditorCursorMove::Left);
            Vec::new()
        }
        UiAction::EditorMoveCursorRight => {
            move_editor_cursor(state, EditorCursorMove::Right);
            Vec::new()
        }
        UiAction::EditorMoveCursorHome => {
            move_editor_cursor(state, EditorCursorMove::Home);
            Vec::new()
        }
        UiAction::EditorMoveCursorEnd => {
            move_editor_cursor(state, EditorCursorMove::End);
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
            maybe_refresh_files_details_on_files_focus(state)
        }
        UiAction::FocusPrev => {
            state.focus = state.focus.prev_left();
            state.last_left_focus = state.focus;
            maybe_refresh_files_details_on_files_focus(state)
        }
        UiAction::FocusPanel { panel } => {
            state.focus = panel;
            if panel.is_left_panel() {
                state.last_left_focus = panel;
            }
            maybe_refresh_files_details_on_files_focus(state)
        }
        UiAction::MoveUp => {
            move_selection(state, true);
            maybe_refresh_files_details_on_files_navigation(state)
        }
        UiAction::MoveDown => {
            move_selection(state, false);
            maybe_refresh_files_details_on_files_navigation(state)
        }
        UiAction::ToggleSelectedDirectory => {
            if toggle_selected_directory(&mut state.files) {
                refresh_files_details_command(state)
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
            refresh_files_details_command(state)
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
            refresh_files_details_command(state)
        }
        UiAction::PrevFileSearchMatch => {
            jump_search_match(&mut state.files, true);
            refresh_files_details_command(state)
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
        UiAction::CreateBranch { name } => {
            with_pending(state, vec![Command::CreateBranch { name }])
        }
        UiAction::CheckoutSelectedBranch => {
            if let Some(branch) = selected_branch_name(state) {
                with_pending(state, vec![Command::CheckoutBranch { name: branch }])
            } else {
                push_notice(state, "No branch selected");
                Vec::new()
            }
        }
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
            refresh_files_details_command(state)
        }
        GitResult::FilesDetailsDiff { paths, result } => {
            if paths != selected_target_paths(&state.files) {
                return Vec::new();
            }
            state.work.details_pending = false;
            state.work.last_completed_command = Some("details".to_string());
            state.details.files_targets = paths.clone();
            match result {
                Ok(diff) => {
                    cache_files_details_diff(state, &paths, &diff);
                    state.details.files_diff = diff;
                    state.details.files_error = None;
                }
                Err(error) => {
                    let message = format!("Failed to refresh files details diff: {error}");
                    state.details.files_diff.clear();
                    state.details.files_error = Some(message.clone());
                    state.status.last_error = Some(message.clone());
                    push_notice(state, &message);
                }
            }
            Vec::new()
        }
        GitResult::RefreshFailed { error } => {
            state.work.refresh_pending = false;
            state.work.last_completed_command = Some("refresh".to_string());
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
        GitResult::Reset { mode, result } => handle_operation_result(
            state,
            result,
            &format!("reset_{}", reset_mode_name(mode)),
            format!("Reset {} to HEAD", reset_mode_name(mode)),
            format!("Failed to reset {}", reset_mode_name(mode)),
        ),
        GitResult::Nuke { result } => handle_operation_result(
            state,
            result,
            "nuke",
            "Nuked working tree".to_string(),
            "Failed to nuke working tree".to_string(),
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

fn open_commit_editor(state: &mut AppState) {
    state.reset_menu.active = false;
    close_discard_confirm(state);
    state.editor.kind = Some(EditorKind::Commit {
        message: String::new(),
        message_cursor: 0,
        body: String::new(),
        body_cursor: 0,
        active_field: CommitField::Message,
    });
}

fn open_stash_editor(state: &mut AppState) {
    state.reset_menu.active = false;
    close_discard_confirm(state);
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EditorCursorMove {
    Left,
    Right,
    Home,
    End,
}

fn move_editor_cursor(state: &mut AppState, movement: EditorCursorMove) {
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
        EditorKind::Commit { message, body, .. } => {
            if message.trim().is_empty() {
                push_notice(state, "Commit message cannot be empty");
                return Vec::new();
            }

            let commit_message = build_commit_message(&message, &body);
            state.commits.draft_message = message.trim().to_string();
            state.editor.kind = None;
            with_pending(
                state,
                vec![Command::CreateCommit {
                    message: commit_message,
                }],
            )
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

fn insert_char_at_cursor(text: &mut String, cursor: &mut usize, ch: char) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    text.insert(*cursor, ch);
    *cursor += ch.len_utf8();
}

fn backspace_at_cursor(text: &mut String, cursor: &mut usize) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    let Some(previous) = previous_char_boundary(text, *cursor) else {
        return;
    };
    text.drain(previous..*cursor);
    *cursor = previous;
}

fn move_cursor_in_text(text: &str, cursor: &mut usize, movement: EditorCursorMove) {
    *cursor = clamp_to_char_boundary(text, *cursor);
    *cursor = match movement {
        EditorCursorMove::Left => previous_char_boundary(text, *cursor).unwrap_or(0),
        EditorCursorMove::Right => next_char_boundary(text, *cursor).unwrap_or(text.len()),
        EditorCursorMove::Home => 0,
        EditorCursorMove::End => text.len(),
    };
}

fn clamp_to_char_boundary(text: &str, cursor: usize) -> usize {
    if cursor >= text.len() {
        return text.len();
    }
    if text.is_char_boundary(cursor) {
        return cursor;
    }
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
        .unwrap_or(0)
}

fn previous_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
}

fn next_char_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .find(|index| *index > cursor)
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
            clear_files_details_cache(state);
            state.work.operation_pending = None;
            state.work.last_completed_command = Some(operation_key.to_string());
            state.last_operation = Some(operation_key.to_string());
            push_notice(state, &success_message);
            state.status.last_error = None;
            with_pending(state, vec![Command::RefreshAll])
        }
        Err(error_message) => {
            state.work.operation_pending = None;
            state.work.last_completed_command = Some(operation_key.to_string());
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
    state.details.files_diff.clear();
    state.details.files_error = None;
    state.details.files_targets = selected_target_paths(&state.files);
    clear_files_details_cache(state);
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

fn refresh_files_details_command(state: &mut AppState) -> Vec<Command> {
    let paths = selected_target_paths(&state.files);
    state.details.files_targets = paths.clone();
    if paths.is_empty() {
        state.details.files_diff.clear();
        state.details.files_error = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    if let Some(diff) = cached_files_details_diff(state, &paths) {
        state.details.files_diff = diff;
        state.details.files_error = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    with_pending(state, vec![Command::RefreshFilesDetailsDiff { paths }])
}

fn cached_files_details_diff(state: &AppState, paths: &[String]) -> Option<String> {
    state
        .details
        .cached_files_diffs
        .iter()
        .find(|entry| entry.paths == paths)
        .map(|entry| entry.diff.clone())
}

fn cache_files_details_diff(state: &mut AppState, paths: &[String], diff: &str) {
    state
        .details
        .cached_files_diffs
        .retain(|entry| entry.paths != paths);
    state.details.cached_files_diffs.insert(
        0,
        CachedFilesDiff {
            paths: paths.to_vec(),
            diff: diff.to_string(),
        },
    );
    state
        .details
        .cached_files_diffs
        .truncate(DETAILS_DIFF_CACHE_LIMIT);
}

fn clear_files_details_cache(state: &mut AppState) {
    state.details.cached_files_diffs.clear();
}

fn maybe_refresh_files_details_on_files_focus(state: &mut AppState) -> Vec<Command> {
    if state.focus == PanelFocus::Files {
        return refresh_files_details_command(state);
    }
    Vec::new()
}

fn maybe_refresh_files_details_on_files_navigation(state: &mut AppState) -> Vec<Command> {
    if state.focus == PanelFocus::Files {
        return refresh_files_details_command(state);
    }
    Vec::new()
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

fn reset_mode_name(mode: ResetMode) -> &'static str {
    match mode {
        ResetMode::Mixed => "mixed",
        ResetMode::Soft => "soft",
        ResetMode::Hard => "hard",
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
