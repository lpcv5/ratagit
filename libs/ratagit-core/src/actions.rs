use crate::{
    AppState, BranchDeleteMode, BranchEntry, CommitEntry, CommitFileDiffTarget, CommitFileEntry,
    FileDiffTarget, FilesSnapshot, RefreshTarget, RepoSnapshot, ResetMode, StashEntry, operations,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    RefreshAll,
    FocusNext,
    FocusPrev,
    FocusPanel { panel: crate::PanelFocus },
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
    FilesRefreshed(FilesSnapshot),
    BranchesRefreshed(Vec<BranchEntry>),
    CommitsRefreshed(Vec<CommitEntry>),
    StashesRefreshed(Vec<StashEntry>),
    FilesDetailsDiff {
        targets: Vec<FileDiffTarget>,
        truncated_from: Option<usize>,
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
        target: Option<RefreshTarget>,
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
    RefreshFiles,
    RefreshBranches,
    RefreshCommits,
    RefreshStash,
    LoadMoreCommits {
        offset: usize,
        limit: usize,
        epoch: u64,
    },
    RefreshFilesDetailsDiff {
        targets: Vec<FileDiffTarget>,
        truncated_from: Option<usize>,
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
    pub fn log_label(&self) -> &'static str {
        match self {
            Command::RefreshAll => "refresh_all",
            Command::RefreshFiles => "refresh_files",
            Command::RefreshBranches => "refresh_branches",
            Command::RefreshCommits => "refresh_commits",
            Command::RefreshStash => "refresh_stash",
            Command::LoadMoreCommits { .. } => "load_more_commits",
            Command::RefreshFilesDetailsDiff { .. } => "refresh_files_details_diff",
            Command::RefreshBranchDetailsLog { .. } => "refresh_branch_details_log",
            Command::RefreshCommitDetailsDiff { .. } => "refresh_commit_details_diff",
            Command::RefreshCommitFiles { .. } => "refresh_commit_files",
            Command::RefreshCommitFileDiff { .. } => "refresh_commit_file_diff",
            Command::StageFiles { .. } => "stage_files",
            Command::UnstageFiles { .. } => "unstage_files",
            Command::StashFiles { .. } => "stash_files",
            Command::Reset { .. } => "reset",
            Command::Nuke => "nuke",
            Command::DiscardFiles { .. } => "discard_files",
            Command::CreateCommit { .. } => "create_commit",
            Command::CreateBranch { .. } => "create_branch",
            Command::CheckoutBranch { .. } => "checkout_branch",
            Command::DeleteBranch { .. } => "delete_branch",
            Command::RebaseBranch { .. } => "rebase_branch",
            Command::SquashCommits { .. } => "squash_commits",
            Command::FixupCommits { .. } => "fixup_commits",
            Command::RewordCommit { .. } => "reword_commit",
            Command::DeleteCommits { .. } => "delete_commits",
            Command::CheckoutCommitDetached { .. } => "checkout_commit_detached",
            Command::StashPush { .. } => "stash_push",
            Command::StashPop { .. } => "stash_pop",
        }
    }

    pub fn refresh_all_commands() -> Vec<Self> {
        vec![
            Self::RefreshFiles,
            Self::RefreshBranches,
            Self::RefreshCommits,
            Self::RefreshStash,
        ]
    }

    pub fn debounce_key(&self) -> Option<&'static str> {
        match self {
            Command::RefreshFilesDetailsDiff { .. } => Some("files_details_diff"),
            Command::RefreshBranchDetailsLog { .. } => Some("branch_details_log"),
            Command::RefreshCommitDetailsDiff { .. } => Some("commit_details_diff"),
            Command::RefreshCommitFileDiff { .. } => Some("commit_file_diff"),
            Command::RefreshAll
            | Command::RefreshFiles
            | Command::RefreshBranches
            | Command::RefreshCommits
            | Command::RefreshStash
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
            | Command::RefreshFiles
            | Command::RefreshBranches
            | Command::RefreshCommits
            | Command::RefreshStash
            | Command::LoadMoreCommits { .. }
            | Command::RefreshFilesDetailsDiff { .. }
            | Command::RefreshBranchDetailsLog { .. }
            | Command::RefreshCommitDetailsDiff { .. }
            | Command::RefreshCommitFiles { .. }
            | Command::RefreshCommitFileDiff { .. } => None,
        }
    }
}

impl GitResult {
    pub fn log_label(&self) -> &'static str {
        match self {
            GitResult::Refreshed(_) => "refreshed",
            GitResult::FilesRefreshed(_) => "files_refreshed",
            GitResult::BranchesRefreshed(_) => "branches_refreshed",
            GitResult::CommitsRefreshed(_) => "commits_refreshed",
            GitResult::StashesRefreshed(_) => "stashes_refreshed",
            GitResult::FilesDetailsDiff { .. } => "files_details_diff",
            GitResult::BranchDetailsLog { .. } => "branch_details_log",
            GitResult::CommitDetailsDiff { .. } => "commit_details_diff",
            GitResult::CommitFiles { .. } => "commit_files",
            GitResult::CommitFileDiff { .. } => "commit_file_diff",
            GitResult::RefreshFailed { .. } => "refresh_failed",
            GitResult::CommitsPage { .. } => "commits_page",
            GitResult::StageFiles { .. } => "stage_files",
            GitResult::UnstageFiles { .. } => "unstage_files",
            GitResult::StashFiles { .. } => "stash_files",
            GitResult::Reset { .. } => "reset",
            GitResult::Nuke { .. } => "nuke",
            GitResult::DiscardFiles { .. } => "discard_files",
            GitResult::CreateCommit { .. } => "create_commit",
            GitResult::CreateBranch { .. } => "create_branch",
            GitResult::CheckoutBranch { .. } => "checkout_branch",
            GitResult::DeleteBranch { .. } => "delete_branch",
            GitResult::RebaseBranch { .. } => "rebase_branch",
            GitResult::SquashCommits { .. } => "squash_commits",
            GitResult::FixupCommits { .. } => "fixup_commits",
            GitResult::RewordCommit { .. } => "reword_commit",
            GitResult::DeleteCommits { .. } => "delete_commits",
            GitResult::CheckoutCommitDetached { .. } => "checkout_commit_detached",
            GitResult::StashPush { .. } => "stash_push",
            GitResult::StashPop { .. } => "stash_pop",
        }
    }

    pub fn is_success(&self) -> bool {
        match self {
            GitResult::Refreshed(_)
            | GitResult::FilesRefreshed(_)
            | GitResult::BranchesRefreshed(_)
            | GitResult::CommitsRefreshed(_)
            | GitResult::StashesRefreshed(_) => true,
            GitResult::RefreshFailed { .. } => false,
            GitResult::FilesDetailsDiff { result, .. }
            | GitResult::BranchDetailsLog { result, .. }
            | GitResult::CommitDetailsDiff { result, .. }
            | GitResult::CommitFileDiff { result, .. } => result.is_ok(),
            GitResult::CommitFiles { result, .. } => result.is_ok(),
            GitResult::CommitsPage { result, .. } => result.is_ok(),
            GitResult::StageFiles { result, .. }
            | GitResult::UnstageFiles { result, .. }
            | GitResult::StashFiles { result, .. }
            | GitResult::Reset { result, .. }
            | GitResult::Nuke { result }
            | GitResult::DiscardFiles { result, .. }
            | GitResult::CreateCommit { result, .. }
            | GitResult::CreateBranch { result, .. }
            | GitResult::CheckoutBranch { result, .. }
            | GitResult::DeleteBranch { result, .. }
            | GitResult::RebaseBranch { result, .. }
            | GitResult::SquashCommits { result, .. }
            | GitResult::FixupCommits { result, .. }
            | GitResult::RewordCommit { result, .. }
            | GitResult::DeleteCommits { result, .. }
            | GitResult::CheckoutCommitDetached { result, .. }
            | GitResult::StashPush { result, .. }
            | GitResult::StashPop { result, .. } => result.is_ok(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_log_labels_are_stable() {
        assert_eq!(Command::RefreshFiles.log_label(), "refresh_files");
        assert_eq!(
            Command::RefreshFilesDetailsDiff {
                targets: Vec::new(),
                truncated_from: None,
            }
            .log_label(),
            "refresh_files_details_diff"
        );
        assert_eq!(
            Command::StageFiles {
                paths: vec!["a.txt".to_string()],
            }
            .log_label(),
            "stage_files"
        );
    }

    #[test]
    fn git_result_success_reports_inner_result_status() {
        assert!(GitResult::BranchesRefreshed(Vec::new()).is_success());
        assert!(
            !GitResult::RefreshFailed {
                target: None,
                error: "boom".to_string(),
            }
            .is_success()
        );
        assert!(
            !GitResult::CreateCommit {
                message: "commit".to_string(),
                result: Err("boom".to_string()),
            }
            .is_success()
        );
    }
}

fn mark_command_pending(state: &mut AppState, command: &Command) {
    match command {
        Command::RefreshAll => {
            state.work.refresh_pending = true;
            state.work.pending_refreshes = RefreshTarget::ALL.into_iter().collect();
        }
        Command::RefreshFiles => {
            mark_refresh_target_pending(state, RefreshTarget::Files);
        }
        Command::RefreshBranches => {
            mark_refresh_target_pending(state, RefreshTarget::Branches);
        }
        Command::RefreshCommits => {
            mark_refresh_target_pending(state, RefreshTarget::Commits);
        }
        Command::RefreshStash => {
            mark_refresh_target_pending(state, RefreshTarget::Stash);
        }
        Command::LoadMoreCommits { .. } => {
            state.commits.loading_more = true;
        }
        Command::RefreshFilesDetailsDiff { .. }
        | Command::RefreshBranchDetailsLog { .. }
        | Command::RefreshCommitDetailsDiff { .. }
        | Command::RefreshCommitFileDiff { .. } => {
            state.work.details_pending = true;
        }
        Command::RefreshCommitFiles { .. } => {
            state.commits.files.loading = true;
        }
        _ => {
            if let Some(label) = command.pending_operation_label() {
                state.work.operation_pending = Some(label);
            }
        }
    }
}

fn mark_refresh_target_pending(state: &mut AppState, target: RefreshTarget) {
    state.work.refresh_pending = true;
    state.work.pending_refreshes.insert(target);
}
