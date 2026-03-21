#[derive(Debug, Clone)]
pub enum GlobalMessage {
    Quit,
    PanelNext,
    PanelPrev,
    PanelGoto(usize),
    ListUp,
    ListDown,
    DiffScrollUp,
    DiffScrollDown,
    RefreshStatus,
}

/// Documentation comment in English.
#[derive(Debug, Clone)]
pub enum Message {
    Quit,

    PanelNext,
    PanelPrev,
    PanelGoto(usize), // 1-based

    ListUp,
    ListDown,

    ToggleDir,
    ToggleVisualSelectMode,
    CollapseAll,
    ExpandAll,

    DiffScrollUp,
    DiffScrollDown,

    StartCommitInput,
    StartSearchInput,
    StartBranchCreateInput,
    CheckoutSelectedBranch,
    DeleteSelectedBranch,
    FetchRemote,
    FetchRemoteFinished(Result<String, String>),
    Commit(String),
    CreateBranch(String),
    StageFile(std::path::PathBuf),
    UnstageFile(std::path::PathBuf),
    DiscardPaths(Vec<std::path::PathBuf>),
    ToggleStageSelection,
    DiscardSelection,
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
    SearchSetQuery(String),
    SearchConfirm,
    SearchClear,
    SearchNext,
    SearchPrev,
}

impl Message {
    pub fn as_global(&self) -> Option<GlobalMessage> {
        match self {
            Message::Quit => Some(GlobalMessage::Quit),
            Message::PanelNext => Some(GlobalMessage::PanelNext),
            Message::PanelPrev => Some(GlobalMessage::PanelPrev),
            Message::PanelGoto(v) => Some(GlobalMessage::PanelGoto(*v)),
            Message::ListUp => Some(GlobalMessage::ListUp),
            Message::ListDown => Some(GlobalMessage::ListDown),
            Message::DiffScrollUp => Some(GlobalMessage::DiffScrollUp),
            Message::DiffScrollDown => Some(GlobalMessage::DiffScrollDown),
            Message::RefreshStatus => Some(GlobalMessage::RefreshStatus),
            _ => None,
        }
    }

}
