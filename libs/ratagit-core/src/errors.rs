use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitErrorKind {
    DivergentPush,
    UnmergedBranchDelete,
    Cli,
    Git2,
    Io,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitFailure {
    pub kind: GitErrorKind,
    pub message: String,
}

impl GitFailure {
    pub fn new(kind: GitErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for GitFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.message.fmt(formatter)
    }
}

impl std::error::Error for GitFailure {}
