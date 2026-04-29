use crate::{
    AppContext, BranchDeleteMode, BranchEntry, CommitEntry, CommitFileDiffTarget, CommitFileEntry,
    DetailsRequestId, FileDiffTarget, FilesSnapshot, GitFailure, RefreshTarget, RepoSnapshot,
    ResetMode, StashEntry, operations,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    RefreshAll,
    Pull,
    Push,
    ConfirmForcePush,
    CancelForcePush,
    ConfirmStageAll,
    CancelStageAll,
    OpenCommandPalette,
    CloseCommandPalette,
    MoveCommandPaletteUp,
    MoveCommandPaletteDown,
    ExecuteCommandPalette {
        details_scroll_lines: usize,
        details_visible_lines: usize,
    },
    FocusNext,
    FocusPrev,
    FocusPanel {
        panel: crate::PanelFocus,
    },
    MoveUp,
    MoveDown,
    MoveUpInViewport {
        visible_lines: usize,
    },
    MoveDownInViewport {
        visible_lines: usize,
    },
    DetailsScrollUp {
        lines: usize,
    },
    DetailsScrollDown {
        lines: usize,
        visible_lines: usize,
    },
    ToggleSelectedDirectory,
    ToggleSelectedFileStage,
    EnterFilesMultiSelect,
    ExitFilesMultiSelect,
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
    AmendStagedChanges,
    OpenCommitEditor,
    OpenStashEditor,
    OpenResetMenu,
    MoveResetMenuUp,
    MoveResetMenuDown,
    ConfirmResetMenu,
    CancelResetMenu,
    ConfirmResetDanger,
    CancelResetDanger,
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
    CreateCommit {
        message: String,
    },
    OpenBranchCommitsPanel,
    CloseBranchCommitsPanel,
    OpenBranchCommitFilesPanel,
    CloseBranchCommitFilesPanel,
    ToggleBranchCommitFilesDirectory,
    OpenCommitFilesPanel,
    CloseCommitFilesPanel,
    ToggleCommitFilesDirectory,
    EnterCommitFilesMultiSelect,
    ExitCommitFilesMultiSelect,
    EnterCommitsMultiSelect,
    ExitCommitsMultiSelect,
    SquashSelectedCommits,
    FixupSelectedCommits,
    OpenCommitRewordEditor,
    DeleteSelectedCommits,
    CheckoutSelectedCommitDetached,
    OpenBranchCreateInput,
    EnterBranchesMultiSelect,
    ExitBranchesMultiSelect,
    BranchCreateInputChar(char),
    BranchCreateBackspace,
    BranchCreateMoveCursorLeft,
    BranchCreateMoveCursorRight,
    BranchCreateMoveCursorHome,
    BranchCreateMoveCursorEnd,
    ConfirmBranchCreate,
    CancelBranchCreate,
    CreateBranch {
        name: String,
        start_point: String,
    },
    CheckoutSelectedBranch,
    OpenBranchDeleteMenu,
    MoveBranchDeleteMenuUp,
    MoveBranchDeleteMenuDown,
    ConfirmBranchDeleteMenu,
    CancelBranchDeleteMenu,
    ConfirmBranchDeleteDanger,
    CancelBranchDeleteDanger,
    ConfirmBranchForceDelete,
    CancelBranchForceDelete,
    OpenBranchRebaseMenu,
    MoveBranchRebaseMenuUp,
    MoveBranchRebaseMenuDown,
    ConfirmBranchRebaseMenu,
    CancelBranchRebaseMenu,
    ConfirmAutoStash,
    CancelAutoStash,
    StashPush {
        message: String,
    },
    StashPopSelected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitResult {
    Refreshed(RepoSnapshot),
    SplitRefreshed {
        files: FilesSnapshot,
        branches: Vec<BranchEntry>,
        commits: Vec<CommitEntry>,
        stashes: Vec<StashEntry>,
    },
    FilesRefreshed(FilesSnapshot),
    BranchesRefreshed(Vec<BranchEntry>),
    CommitsRefreshed(Vec<CommitEntry>),
    StashesRefreshed(Vec<StashEntry>),
    FilesDetailsDiff {
        request_id: DetailsRequestId,
        targets: Vec<FileDiffTarget>,
        truncated_from: Option<usize>,
        result: Result<String, String>,
    },
    BranchDetailsLog {
        request_id: DetailsRequestId,
        branch: String,
        result: Result<String, String>,
    },
    CommitDetailsDiff {
        request_id: DetailsRequestId,
        commit_id: String,
        result: Result<String, String>,
    },
    BranchCommits {
        branch: String,
        result: Result<Vec<CommitEntry>, String>,
    },
    BranchCommitFiles {
        branch: String,
        commit_id: String,
        result: Result<Vec<CommitFileEntry>, String>,
    },
    CommitFiles {
        commit_id: String,
        result: Result<Vec<CommitFileEntry>, String>,
    },
    CommitFileDiff {
        request_id: DetailsRequestId,
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
    StageAllThenCreateCommit {
        message: String,
        paths: Vec<String>,
        result: Result<(), String>,
    },
    AmendStagedChanges {
        commit_id: String,
        result: Result<(), String>,
    },
    StageAllThenAmendStagedChanges {
        commit_id: String,
        paths: Vec<String>,
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
        result: Result<(), GitFailure>,
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
    Pull {
        result: Result<(), String>,
    },
    Push {
        force: bool,
        result: Result<(), GitFailure>,
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
        request_id: DetailsRequestId,
        targets: Vec<FileDiffTarget>,
        truncated_from: Option<usize>,
    },
    RefreshBranchDetailsLog {
        request_id: DetailsRequestId,
        branch: String,
        max_count: usize,
    },
    RefreshCommitDetailsDiff {
        request_id: DetailsRequestId,
        commit_id: String,
    },
    RefreshBranchCommits {
        branch: String,
    },
    RefreshBranchCommitFiles {
        branch: String,
        commit_id: String,
    },
    RefreshCommitFiles {
        commit_id: String,
    },
    RefreshCommitFileDiff {
        request_id: DetailsRequestId,
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
    StageAllThenCreateCommit {
        message: String,
        paths: Vec<String>,
    },
    AmendStagedChanges {
        commit_id: String,
    },
    StageAllThenAmendStagedChanges {
        commit_id: String,
        paths: Vec<String>,
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
    Pull,
    Push {
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandRefreshKey {
    All,
    Target(RefreshTarget),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandLogLabel {
    Static(&'static str),
    Push,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PendingOperationLabel {
    None,
    Static(&'static str),
    Reset,
    Push,
    DeleteBranch,
    RebaseBranch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommandMetadata {
    log_label: CommandLogLabel,
    mutating: bool,
    debounce_key: Option<&'static str>,
    refresh_key: Option<CommandRefreshKey>,
    pending_label: PendingOperationLabel,
}

impl Command {
    pub fn log_label(&self) -> &'static str {
        match self.metadata().log_label {
            CommandLogLabel::Static(label) => label,
            CommandLogLabel::Push => match self {
                Command::Push { force } if *force => "force_push",
                Command::Push { .. } => "push",
                _ => unreachable!("push log label metadata used for non-push command"),
            },
        }
    }

    fn metadata(&self) -> CommandMetadata {
        match self {
            Command::RefreshAll => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_all"),
                mutating: false,
                debounce_key: None,
                refresh_key: Some(CommandRefreshKey::All),
                pending_label: PendingOperationLabel::None,
            },
            Command::Pull => CommandMetadata {
                log_label: CommandLogLabel::Static("pull"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("pull"),
            },
            Command::Push { .. } => CommandMetadata {
                log_label: CommandLogLabel::Push,
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Push,
            },
            Command::RefreshFiles => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_files"),
                mutating: false,
                debounce_key: None,
                refresh_key: Some(CommandRefreshKey::Target(RefreshTarget::Files)),
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshBranches => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_branches"),
                mutating: false,
                debounce_key: None,
                refresh_key: Some(CommandRefreshKey::Target(RefreshTarget::Branches)),
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshCommits => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_commits"),
                mutating: false,
                debounce_key: None,
                refresh_key: Some(CommandRefreshKey::Target(RefreshTarget::Commits)),
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshStash => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_stash"),
                mutating: false,
                debounce_key: None,
                refresh_key: Some(CommandRefreshKey::Target(RefreshTarget::Stash)),
                pending_label: PendingOperationLabel::None,
            },
            Command::LoadMoreCommits { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("load_more_commits"),
                mutating: false,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshFilesDetailsDiff { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_files_details_diff"),
                mutating: false,
                debounce_key: Some("files_details_diff"),
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshBranchDetailsLog { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_branch_details_log"),
                mutating: false,
                debounce_key: Some("branch_details_log"),
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshCommitDetailsDiff { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_commit_details_diff"),
                mutating: false,
                debounce_key: Some("commit_details_diff"),
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshBranchCommits { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_branch_commits"),
                mutating: false,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshBranchCommitFiles { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_branch_commit_files"),
                mutating: false,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshCommitFiles { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_commit_files"),
                mutating: false,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::RefreshCommitFileDiff { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("refresh_commit_file_diff"),
                mutating: false,
                debounce_key: Some("commit_file_diff"),
                refresh_key: None,
                pending_label: PendingOperationLabel::None,
            },
            Command::StageFiles { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("stage_files"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("stage"),
            },
            Command::UnstageFiles { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("unstage_files"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("unstage"),
            },
            Command::StashFiles { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("stash_files"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("stash_files"),
            },
            Command::Reset { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("reset"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Reset,
            },
            Command::Nuke => CommandMetadata {
                log_label: CommandLogLabel::Static("nuke"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("nuke"),
            },
            Command::DiscardFiles { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("discard_files"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("discard_files"),
            },
            Command::CreateCommit { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("create_commit"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("commit"),
            },
            Command::StageAllThenCreateCommit { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("stage_all_then_create_commit"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("commit"),
            },
            Command::AmendStagedChanges { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("amend_staged_changes"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("amend"),
            },
            Command::StageAllThenAmendStagedChanges { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("stage_all_then_amend_staged_changes"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("amend"),
            },
            Command::CreateBranch { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("create_branch"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("create_branch"),
            },
            Command::CheckoutBranch { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("checkout_branch"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("checkout_branch"),
            },
            Command::DeleteBranch { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("delete_branch"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::DeleteBranch,
            },
            Command::RebaseBranch { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("rebase_branch"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::RebaseBranch,
            },
            Command::SquashCommits { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("squash_commits"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("squash_commits"),
            },
            Command::FixupCommits { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("fixup_commits"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("fixup_commits"),
            },
            Command::RewordCommit { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("reword_commit"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("reword_commit"),
            },
            Command::DeleteCommits { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("delete_commits"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("delete_commits"),
            },
            Command::CheckoutCommitDetached { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("checkout_commit_detached"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("checkout_detached"),
            },
            Command::StashPush { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("stash_push"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("stash_push"),
            },
            Command::StashPop { .. } => CommandMetadata {
                log_label: CommandLogLabel::Static("stash_pop"),
                mutating: true,
                debounce_key: None,
                refresh_key: None,
                pending_label: PendingOperationLabel::Static("stash_pop"),
            },
        }
    }

    fn push_pending_label(&self) -> String {
        match self {
            Command::Push { force } => {
                if *force {
                    "force_push".to_string()
                } else {
                    "push".to_string()
                }
            }
            _ => unreachable!("push pending label metadata used for non-push command"),
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
        self.metadata().debounce_key
    }

    pub fn refresh_coalescing_key(&self) -> Option<CommandRefreshKey> {
        self.metadata().refresh_key
    }

    pub fn is_mutating(&self) -> bool {
        self.metadata().mutating
    }

    pub fn pending_operation_label(&self) -> Option<String> {
        match self.metadata().pending_label {
            PendingOperationLabel::None => None,
            PendingOperationLabel::Static(label) => Some(label.to_string()),
            PendingOperationLabel::Reset => match self {
                Command::Reset { mode } => {
                    Some(format!("reset_{}", operations::reset_mode_name(*mode)))
                }
                _ => unreachable!("reset pending label metadata used for non-reset command"),
            },
            PendingOperationLabel::Push => Some(self.push_pending_label()),
            PendingOperationLabel::DeleteBranch => match self {
                Command::DeleteBranch { mode, .. } => Some(format!(
                    "delete_branch_{}",
                    operations::delete_mode_name(*mode)
                )),
                _ => {
                    unreachable!("delete branch pending label metadata used for non-delete command")
                }
            },
            PendingOperationLabel::RebaseBranch => match self {
                Command::RebaseBranch { interactive, .. } => {
                    let mode = if *interactive {
                        "interactive"
                    } else {
                        "simple"
                    };
                    Some(format!("rebase_branch_{mode}"))
                }
                _ => unreachable!("rebase pending label metadata used for non-rebase command"),
            },
        }
    }
}

impl GitResult {
    pub fn log_label(&self) -> &'static str {
        match self {
            GitResult::Refreshed(_) => "refreshed",
            GitResult::SplitRefreshed { .. } => "split_refreshed",
            GitResult::Pull { .. } => "pull",
            GitResult::Push { force, .. } => {
                if *force {
                    "force_push"
                } else {
                    "push"
                }
            }
            GitResult::FilesRefreshed(_) => "files_refreshed",
            GitResult::BranchesRefreshed(_) => "branches_refreshed",
            GitResult::CommitsRefreshed(_) => "commits_refreshed",
            GitResult::StashesRefreshed(_) => "stashes_refreshed",
            GitResult::FilesDetailsDiff { .. } => "files_details_diff",
            GitResult::BranchDetailsLog { .. } => "branch_details_log",
            GitResult::CommitDetailsDiff { .. } => "commit_details_diff",
            GitResult::BranchCommits { .. } => "branch_commits",
            GitResult::BranchCommitFiles { .. } => "branch_commit_files",
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
            GitResult::StageAllThenCreateCommit { .. } => "stage_all_then_create_commit",
            GitResult::AmendStagedChanges { .. } => "amend_staged_changes",
            GitResult::StageAllThenAmendStagedChanges { .. } => {
                "stage_all_then_amend_staged_changes"
            }
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
            | GitResult::SplitRefreshed { .. }
            | GitResult::FilesRefreshed(_)
            | GitResult::BranchesRefreshed(_)
            | GitResult::CommitsRefreshed(_)
            | GitResult::StashesRefreshed(_) => true,
            GitResult::RefreshFailed { .. } => false,
            GitResult::FilesDetailsDiff { result, .. }
            | GitResult::BranchDetailsLog { result, .. }
            | GitResult::CommitDetailsDiff { result, .. }
            | GitResult::CommitFileDiff { result, .. } => result.is_ok(),
            GitResult::BranchCommits { result, .. } => result.is_ok(),
            GitResult::BranchCommitFiles { result, .. } => result.is_ok(),
            GitResult::CommitFiles { result, .. } => result.is_ok(),
            GitResult::CommitsPage { result, .. } => result.is_ok(),
            GitResult::StageFiles { result, .. }
            | GitResult::UnstageFiles { result, .. }
            | GitResult::StashFiles { result, .. }
            | GitResult::Reset { result, .. }
            | GitResult::Nuke { result }
            | GitResult::DiscardFiles { result, .. }
            | GitResult::CreateCommit { result, .. }
            | GitResult::StageAllThenCreateCommit { result, .. }
            | GitResult::AmendStagedChanges { result, .. }
            | GitResult::StageAllThenAmendStagedChanges { result, .. }
            | GitResult::CreateBranch { result, .. }
            | GitResult::CheckoutBranch { result, .. }
            | GitResult::RebaseBranch { result, .. }
            | GitResult::SquashCommits { result, .. }
            | GitResult::FixupCommits { result, .. }
            | GitResult::RewordCommit { result, .. }
            | GitResult::DeleteCommits { result, .. }
            | GitResult::CheckoutCommitDetached { result, .. }
            | GitResult::StashPush { result, .. }
            | GitResult::StashPop { result, .. }
            | GitResult::Pull { result } => result.is_ok(),
            GitResult::DeleteBranch { result, .. } | GitResult::Push { result, .. } => {
                result.is_ok()
            }
        }
    }
}

pub fn debounce_key_for_command(command: &Command) -> Option<&'static str> {
    command.debounce_key()
}

pub fn refresh_key_for_command(command: &Command) -> Option<CommandRefreshKey> {
    command.refresh_coalescing_key()
}

pub(crate) fn with_pending(state: &mut AppContext, commands: Vec<Command>) -> Vec<Command> {
    for command in &commands {
        mark_command_pending(state, command);
    }
    commands
}

fn mark_command_pending(state: &mut AppContext, command: &Command) {
    match command {
        Command::RefreshAll => {
            state.work.mark_refresh_all_pending();
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
            state.work.pagination.commits_loading_more = true;
        }
        Command::RefreshFilesDetailsDiff { .. }
        | Command::RefreshBranchDetailsLog { .. }
        | Command::RefreshCommitDetailsDiff { .. }
        | Command::RefreshCommitFileDiff { .. } => {
            state.work.details.details_pending = true;
        }
        Command::RefreshCommitFiles { .. } | Command::RefreshBranchCommitFiles { .. } => {
            state.work.commit_files.commit_files_loading = true;
        }
        Command::RefreshBranchCommits { .. } => {
            mark_refresh_target_pending(state, RefreshTarget::Branches);
        }
        _ => {
            if let Some(label) = command.pending_operation_label() {
                state.work.mutation.operation_pending = Some(label);
            }
        }
    }
}

fn mark_refresh_target_pending(state: &mut AppContext, target: RefreshTarget) {
    state.work.mark_refresh_target_pending(target);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_log_labels_are_stable() {
        assert_eq!(Command::RefreshFiles.log_label(), "refresh_files");
        assert_eq!(
            Command::RefreshFilesDetailsDiff {
                request_id: DetailsRequestId(0),
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
