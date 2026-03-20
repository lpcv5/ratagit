use super::Message;
use std::sync::mpsc::Receiver;

/// Documentation comment in English.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Command {
    /// Documentation comment in English.
    None,
    /// Documentation comment in English.
    Async(Receiver<Message>),
    /// Documentation comment in English.
    Sync(Message),
}
