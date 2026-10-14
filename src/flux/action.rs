use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum DomainAction {
    ToggleDiffFocus,
    SwitchDiffSide,
    HunkNext,
    HunkPrevious,
    ToggleHunkSelection,
    ClearHunkSelection,
    ApplySelectedHunks(crate::git::patch::PatchDirection),
    ApplyHunksFinished {
        id: u64,
        direction: crate::git::patch::PatchDirection,
        result: Result<(), String>,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    StageHunks {
        patch: std::sync::Arc<crate::git::patch::Patch>,
        file: usize,
        hunks: Vec<usize>,
    },
    StageHunksFinished {
        result: Result<(), String>,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    ControlRepositoryOperation(crate::git::operation::OperationControl),
    RepositoryOperationFinished {
        id: u64,
        result: Result<(), String>,
        observed: Option<crate::git::operation::RepositoryOperation>,
    },
    Runtime(crate::flux::effects::RuntimeEvent),
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
    StartSearchInput,
    StartBranchCreateInput,
    CheckoutSelectedBranch,
    BranchSwitchConfirm(bool),
    CommitAllConfirm(bool),
    DeleteSelectedBranch,
    FetchRemote,
    FetchRemoteFinished(Result<String, String>),
    CreateBranch(String),
    CreateBranchFinished {
        name: String,
        result: Result<(), String>,
    },
    CheckoutBranchFinished {
        name: String,
        auto_stash: bool,
        result: Result<(), String>,
    },
    DeleteBranchFinished {
        name: String,
        result: Result<(), String>,
    },
    Commit(String),
    CommitFinished {
        message: String,
        result: Result<String, String>,
    },
    StageFile(PathBuf),
    UnstageFile(PathBuf),
    DiscardPaths(Vec<PathBuf>),
    StageFileFinished {
        path: PathBuf,
        result: Result<(), String>,
    },
    UnstageFileFinished {
        path: PathBuf,
        result: Result<(), String>,
    },
    DiscardPathsFinished {
        paths: Vec<PathBuf>,
        result: Result<(), String>,
    },
    StagePathsFinished {
        result: Result<(), String>,
    },
    ToggleStageSelectionFinished {
        result: Result<(usize, usize), String>,
    },
    PrepareCommitFromSelectionFinished {
        result: Result<usize, String>,
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
        result: Result<usize, String>,
    },
    RevisionOpenTreeOrToggleDir,
    RevisionCloseTree,
    StashApplySelected,
    StashApplyFinished {
        index: usize,
        result: Result<(), String>,
    },
    StashPopSelected,
    StashPopFinished {
        index: usize,
        result: Result<(), String>,
    },
    StashDropSelected,
    StashDropFinished {
        index: usize,
        result: Result<(), String>,
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
    BackgroundCompleted(Box<crate::app::BackgroundCompleted>),
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
