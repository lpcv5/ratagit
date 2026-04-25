#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelFocus {
    Files,
    Branches,
    Commits,
    Stash,
    Details,
    Log,
}

impl PanelFocus {
    pub fn next_left(self) -> Self {
        match self {
            Self::Files => Self::Branches,
            Self::Branches => Self::Commits,
            Self::Commits => Self::Stash,
            Self::Stash => Self::Files,
            Self::Details | Self::Log => Self::Files,
        }
    }

    pub fn prev_left(self) -> Self {
        match self {
            Self::Files => Self::Stash,
            Self::Branches => Self::Files,
            Self::Commits => Self::Branches,
            Self::Stash => Self::Commits,
            Self::Details | Self::Log => Self::Stash,
        }
    }

    pub fn is_left_panel(self) -> bool {
        matches!(
            self,
            Self::Files | Self::Branches | Self::Commits | Self::Stash
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    pub path: String,
    pub staged: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitEntry {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchEntry {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashEntry {
    pub id: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoSnapshot {
    pub status_summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub files: Vec<FileEntry>,
    pub commits: Vec<CommitEntry>,
    pub branches: Vec<BranchEntry>,
    pub stashes: Vec<StashEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusPanelState {
    pub summary: String,
    pub current_branch: String,
    pub detached_head: bool,
    pub refresh_count: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilesPanelState {
    pub items: Vec<FileEntry>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitsPanelState {
    pub items: Vec<CommitEntry>,
    pub selected: usize,
    pub draft_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchesPanelState {
    pub items: Vec<BranchEntry>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashPanelState {
    pub items: Vec<StashEntry>,
    pub selected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppState {
    pub focus: PanelFocus,
    pub last_left_focus: PanelFocus,
    pub status: StatusPanelState,
    pub files: FilesPanelState,
    pub commits: CommitsPanelState,
    pub branches: BranchesPanelState,
    pub stash: StashPanelState,
    pub notices: Vec<String>,
    pub last_operation: Option<String>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            focus: PanelFocus::Files,
            last_left_focus: PanelFocus::Files,
            status: StatusPanelState {
                summary: "No data yet".to_string(),
                current_branch: "unknown".to_string(),
                detached_head: false,
                refresh_count: 0,
                last_error: None,
            },
            files: FilesPanelState {
                items: Vec::new(),
                selected: 0,
            },
            commits: CommitsPanelState {
                items: Vec::new(),
                selected: 0,
                draft_message: String::new(),
            },
            branches: BranchesPanelState {
                items: Vec::new(),
                selected: 0,
            },
            stash: StashPanelState {
                items: Vec::new(),
                selected: 0,
            },
            notices: vec!["Ready".to_string()],
            last_operation: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    RefreshAll,
    FocusNext,
    FocusPrev,
    FocusPanel { panel: PanelFocus },
    MoveUp,
    MoveDown,
    StageSelectedFile,
    UnstageSelectedFile,
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
    StageFile {
        path: String,
        result: Result<(), String>,
    },
    UnstageFile {
        path: String,
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
    StageFile { path: String },
    UnstageFile { path: String },
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
        UiAction::StageSelectedFile => {
            if let Some(path) = selected_file_path(state, false) {
                vec![Command::StageFile { path }]
            } else {
                push_notice(state, "No unstaged file selected");
                Vec::new()
            }
        }
        UiAction::UnstageSelectedFile => {
            if let Some(path) = selected_file_path(state, true) {
                vec![Command::UnstageFile { path }]
            } else {
                push_notice(state, "No staged file selected");
                Vec::new()
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
        GitResult::StageFile { path, result } => handle_operation_result(
            state,
            result,
            "stage",
            format!("Staged {path}"),
            format!("Failed to stage {path}"),
        ),
        GitResult::UnstageFile { path, result } => handle_operation_result(
            state,
            result,
            "unstage",
            format!("Unstaged {path}"),
            format!("Failed to unstage {path}"),
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
    state.commits.items = snapshot.commits;
    state.branches.items = snapshot.branches;
    state.stash.items = snapshot.stashes;
    clamp_selection_indexes(state);
}

fn clamp_selection_indexes(state: &mut AppState) {
    state.files.selected = clamp_index(state.files.selected, state.files.items.len());
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
            move_index(&mut state.files.selected, state.files.items.len(), move_up)
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

fn selected_file_path(state: &AppState, staged: bool) -> Option<String> {
    state
        .files
        .items
        .get(state.files.selected)
        .and_then(|entry| {
            if entry.staged == staged {
                Some(entry.path.clone())
            } else {
                None
            }
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_action_emits_refresh_command() {
        let mut state = AppState::default();
        let commands = update(&mut state, Action::Ui(UiAction::RefreshAll));
        assert_eq!(commands, vec![Command::RefreshAll]);
    }

    #[test]
    fn stage_selected_file_emits_stage_command() {
        let mut state = AppState {
            focus: PanelFocus::Files,
            ..AppState::default()
        };
        state.files.items = vec![
            FileEntry {
                path: "a.txt".to_string(),
                staged: false,
            },
            FileEntry {
                path: "b.txt".to_string(),
                staged: false,
            },
        ];
        state.files.selected = 1;
        let commands = update(&mut state, Action::Ui(UiAction::StageSelectedFile));
        assert_eq!(
            commands,
            vec![Command::StageFile {
                path: "b.txt".to_string()
            }]
        );
    }

    #[test]
    fn refreshed_snapshot_updates_state_and_clamps_indexes() {
        let mut state = AppState::default();
        state.files.selected = 99;
        let snapshot = RepoSnapshot {
            status_summary: "dirty".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: vec![FileEntry {
                path: "only.txt".to_string(),
                staged: true,
            }],
            commits: vec![],
            branches: vec![],
            stashes: vec![],
        };
        let commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(snapshot)),
        );
        assert!(commands.is_empty());
        assert_eq!(state.status.summary, "dirty");
        assert_eq!(state.files.selected, 0);
        assert_eq!(state.status.refresh_count, 1);
    }

    #[test]
    fn failed_git_result_is_visible_in_state() {
        let mut state = AppState::default();
        let commands = update(
            &mut state,
            Action::GitResult(GitResult::CreateCommit {
                message: "wip".to_string(),
                result: Err("nothing staged".to_string()),
            }),
        );
        assert!(commands.is_empty());
        assert!(
            state
                .status
                .last_error
                .as_ref()
                .expect("error should be stored")
                .contains("nothing staged")
        );
    }

    #[test]
    fn focus_next_and_prev_cycle_only_left_panels() {
        let mut state = AppState::default();
        assert_eq!(state.focus, PanelFocus::Files);
        assert_eq!(state.last_left_focus, PanelFocus::Files);

        update(&mut state, Action::Ui(UiAction::FocusNext));
        assert_eq!(state.focus, PanelFocus::Branches);
        assert_eq!(state.last_left_focus, PanelFocus::Branches);

        update(&mut state, Action::Ui(UiAction::FocusNext));
        assert_eq!(state.focus, PanelFocus::Commits);
        assert_eq!(state.last_left_focus, PanelFocus::Commits);

        update(&mut state, Action::Ui(UiAction::FocusPrev));
        assert_eq!(state.focus, PanelFocus::Branches);
        assert_eq!(state.last_left_focus, PanelFocus::Branches);
    }

    #[test]
    fn focus_panel_allows_right_focus_and_preserves_last_left() {
        let mut state = AppState::default();
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Stash,
            }),
        );
        assert_eq!(state.focus, PanelFocus::Stash);
        assert_eq!(state.last_left_focus, PanelFocus::Stash);

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
        );
        assert_eq!(state.focus, PanelFocus::Details);
        assert_eq!(state.last_left_focus, PanelFocus::Stash);
    }

    #[test]
    fn move_selection_does_not_change_left_indexes_when_focus_is_right_panel() {
        let mut state = AppState::default();
        state.files.items = vec![
            FileEntry {
                path: "a".to_string(),
                staged: false,
            },
            FileEntry {
                path: "b".to_string(),
                staged: false,
            },
        ];
        state.files.selected = 1;
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
        );
        update(&mut state, Action::Ui(UiAction::MoveUp));
        assert_eq!(state.files.selected, 1);
    }
}
