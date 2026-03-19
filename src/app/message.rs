/// 应用消息（TEA 架构中的 Message）
#[derive(Debug, Clone)]
pub enum Message {
    // 全局事件
    Quit,
    TabNext,
    TabPrev,

    // 面板导航
    PanelNext,
    PanelPrev,

    // 列表导航
    ListUp,
    ListDown,

    // Git 操作
    StageFile(std::path::PathBuf),
    UnstageFile(std::path::PathBuf),
    RefreshStatus,

    // Git 结果
    GitStatusLoaded(crate::git::GitStatus),
    GitError(crate::git::GitError),
}
