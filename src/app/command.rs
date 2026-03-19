use super::Message;

/// Documentation comment in English.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Command {
    /// Documentation comment in English.
    None,
    /// Documentation comment in English.
    Async(tokio::task::JoinHandle<Message>),
    /// Documentation comment in English.
    Sync(Message),
}
