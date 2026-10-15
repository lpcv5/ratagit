use std::path::PathBuf;

/// State-only work is expressed as an event and applied by a store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeEvent {
    NavigateDiffFile {
        forward: bool,
    },
    ProcessBackgroundLoads,
    #[cfg_attr(not(test), allow(dead_code))]
    FlushPendingRefresh {
        log_success: bool,
    },
    FlushPendingDiffReload,
    EnsureCommitsLoadedForActivePanel,
    ReloadDiffNow,
    RevisionOpenTreeOrToggleDir,
}

#[derive(Debug, Clone)]
pub enum DomainAction {
    ToggleStageAll,
    ToggleStageAllFinished {
        result: Result<bool, crate::git::GitError>,
    },
    AmendHead,
    AmendHeadFinished {
        result: Result<String, crate::git::GitError>,
    },
    IgnoreFile(PathBuf),
    IgnoreFileFinished {
        path: PathBuf,
        result: Result<(), crate::git::GitError>,
    },
    EditFile(PathBuf),
    EditorPrepared {
        result: Result<crate::desktop::EditorCommand, crate::git::GitError>,
    },
    EditorFinished {
        result: Result<(), crate::git::GitError>,
    },
    CopyPath(PathBuf),
    CopyPathFinished {
        path: PathBuf,
        result: Result<(), crate::git::GitError>,
    },
    ToggleDiffFocus,
    SwitchDiffSide,
    HunkNext,
    HunkPrevious,
    DiffNextLine,
    DiffPreviousLine,
    ToggleHunkSelection,
    ToggleDiffLineMode,
    ToggleDiffLineRange,
    ClearHunkSelection,
    ApplySelectedHunks(crate::git::patch::PatchDirection),
    ApplyHunksFinished {
        id: u64,
        direction: crate::git::patch::PatchDirection,
        result: Result<(), crate::git::GitError>,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    StageHunks {
        patch: std::sync::Arc<crate::git::patch::Patch>,
        file: usize,
        hunks: Vec<usize>,
    },
    StageHunksFinished {
        result: Result<(), crate::git::GitError>,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    ControlRepositoryOperation(crate::git::operation::OperationControl),
    RepositoryOperationFinished {
        id: u64,
        result: Result<(), crate::git::GitError>,
        observed: Option<crate::git::operation::RepositoryOperation>,
    },
    Runtime(RuntimeEvent),
    PanelNext,
    PanelPrev,
    PanelGoto(usize),
    ListUp,
    ListDown,
    DiffScrollUp,
    DiffScrollDown,
    ToggleDir,
    CollapseAll,
    ExpandAll,
    Quit,
    Refresh,
    StartCommitInput,
    StartCommandPalette,
    Help(crate::app::help::HelpAction),
    StartSearchInput,
    StartBranchCreateInput,
    CheckoutSelectedBranch,
    BranchSwitchConfirm(bool),
    CommitAllConfirm(bool),
    DeleteSelectedBranch,
    FetchRemote,
    FetchRemoteFinished(Result<String, crate::git::GitError>),
    CreateBranch(String),
    CreateBranchFinished {
        name: String,
        result: Result<(), crate::git::GitError>,
    },
    CheckoutBranchFinished {
        name: String,
        auto_stash: bool,
        result: Result<(), crate::git::GitError>,
    },
    DeleteBranchFinished {
        name: String,
        result: Result<(), crate::git::GitError>,
    },
    Commit(String),
    CommitFinished {
        message: String,
        result: Result<String, crate::git::GitError>,
    },
    StageFile(PathBuf),
    UnstageFile(PathBuf),
    DiscardPaths(Vec<PathBuf>),
    StageFileFinished {
        path: PathBuf,
        result: Result<(), crate::git::GitError>,
    },
    UnstageFileFinished {
        path: PathBuf,
        result: Result<(), crate::git::GitError>,
    },
    DiscardPathsFinished {
        paths: Vec<PathBuf>,
        result: Result<(), crate::git::GitError>,
    },
    StagePathsFinished {
        result: Result<(), crate::git::GitError>,
    },
    ToggleStageSelectionFinished {
        result: Result<(usize, usize), crate::git::GitError>,
    },
    PrepareCommitFromSelectionFinished {
        result: Result<usize, crate::git::GitError>,
    },
    ToggleVisualSelectMode,
    ToggleStageSelection,
    DiscardSelection,
    PrepareCommitFromSelection,
    StartStashInput,
    StashPush {
        message: String,
        paths: Vec<PathBuf>,
    },
    StashPushFinished {
        message: String,
        result: Result<usize, crate::git::GitError>,
    },
    RevisionOpenTreeOrToggleDir,
    RevisionCloseTree,
    StashApplySelected,
    StashApplyFinished {
        index: usize,
        result: Result<(), crate::git::GitError>,
    },
    StashPopSelected,
    StashPopFinished {
        index: usize,
        result: Result<(), crate::git::GitError>,
    },
    StashDropSelected,
    StashDropFinished {
        index: usize,
        result: Result<(), crate::git::GitError>,
    },
    SearchSetQuery(String),
    SearchConfirm,
    SearchClear,
    SearchNext,
    SearchPrev,
    InputEsc,
    InputTab,
    InputEnter,
    InputBackspace,
    InputChar(char),
}

#[derive(Debug, Clone)]
pub enum SystemAction {
    BackgroundCompleted(Box<crate::background::BackgroundCompleted>),
    Tick,
    AutoRefresh,
    /// Terminal resize event. The `width` and `height` are forwarded for completeness;
    /// stores trigger a full UI invalidation without reading the dimensions (ratatui
    /// handles the actual resize internally).
    #[allow(dead_code)]
    Resize {
        width: u16,
        height: u16,
    },
}

#[derive(Debug, Clone)]
pub enum Action {
    Domain(DomainAction),
    System(SystemAction),
}

#[derive(Debug, Clone)]
pub struct ActionEnvelope {
    pub sequence: u64,
    pub action: Action,
}
