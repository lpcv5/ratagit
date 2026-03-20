/// Documentation comment in English.
#[derive(Debug, Clone)]
pub enum Message {
    // Comment in English.
    Quit,

    // Comment in English.
    PanelNext,
    PanelPrev,
    PanelGoto(usize), // 1-based

    // Comment in English.
    ListUp,
    ListDown,

    // Comment in English.
    ToggleDir,
    ToggleVisualSelectMode,
    CollapseAll,
    ExpandAll,

    // Comment in English.
    DiffScrollUp,
    DiffScrollDown,

    // Comment in English.
    StartCommitInput,
    StartBranchCreateInput,
    CheckoutSelectedBranch,
    DeleteSelectedBranch,
    FetchRemote,
    Commit(String),
    CreateBranch(String),
    StageFile(std::path::PathBuf),
    UnstageFile(std::path::PathBuf),
    ToggleStageSelection,
    PrepareCommitFromSelection,
    StartStashInput,
    StashPush {
        message: String,
        paths: Vec<std::path::PathBuf>,
    },
    RevisionOpenTreeOrToggleDir,
    RevisionCloseTree,
    StashApplySelected,
    StashPopSelected,
    StashDropSelected,
    RefreshStatus,
    // Comment in English.
}
