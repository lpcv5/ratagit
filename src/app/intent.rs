use super::Panel;
use crate::backend::BackendCommand;

/// Intent 枚举：组件 → App 的意图通信
#[derive(Debug, Clone)]
pub enum Intent {
    SelectNext,
    SelectPrevious,
    SwitchFocus(Panel),
    RefreshPanelDetail,
    ScrollMainView(i16),
    ScrollLog(i16),
    ActivatePanel,
    ToggleStageFile,
    StageAll,
    DiscardSelected,
    StashSelected,
    AmendCommit,
    ShowResetMenu,
    ExecuteResetOption(usize),
    CloseModal,
    #[allow(dead_code)]
    SendCommand(BackendCommand),
    None,
}
