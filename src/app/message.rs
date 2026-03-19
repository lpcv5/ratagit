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
    Commit(String),
    CreateBranch(String),
    StageFile(std::path::PathBuf),
    UnstageFile(std::path::PathBuf),
    ToggleStageSelection,
    PrepareCommitFromSelection,
    RefreshStatus,

    // Comment in English.
}
