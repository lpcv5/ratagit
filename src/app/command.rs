use super::Message;

/// 命令（TEA 架构中的 Command）
#[derive(Debug)]
pub enum Command {
    /// 无操作
    None,
    /// 异步任务（Phase 2 实现）
    Async(tokio::task::JoinHandle<Message>),
    /// 同步消息
    Sync(Message),
}
