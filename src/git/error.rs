use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitErrorCategory {
    Conflict,
    Authentication,
    Locked,
    NotFound,
    Exists,
    InvalidState,
    StalePatch,
    Worker,
    Other,
}

/// Owned libgit2 diagnostics; no repository handle crosses a worker boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGitError {
    pub code: i32,
    pub class: i64,
    pub message: String,
    pub category: GitErrorCategory,
}

impl std::fmt::Display for NativeGitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (code {}, class {})",
            self.message, self.code, self.class
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitProcessError {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitIoError {
    pub kind: std::io::ErrorKind,
    pub raw_os_error: Option<i32>,
    pub message: String,
}

impl std::fmt::Display for GitIoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} (kind {:?}, OS {:?})",
            self.message, self.kind, self.raw_os_error
        )
    }
}

impl std::fmt::Display for GitProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.exit_code {
            Some(code) => write!(f, "git exited with code {code}"),
            None => write!(f, "git terminated without an exit code"),
        }?;
        if !self.stderr.is_empty() {
            write!(f, ": {}", self.stderr)?;
        }
        if !self.stdout.is_empty() {
            write!(f, "; stdout: {}", self.stdout)?;
        }
        Ok(())
    }
}

/// Structured failures remain owned and cloneable through completion actions.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git2(String),
    #[error("Git error: {0}")]
    Native(Arc<NativeGitError>),
    #[error("{0}")]
    Process(Arc<GitProcessError>),
    #[error("I/O error: {0}")]
    Io(Arc<GitIoError>),
    #[error("Invalid repository state")]
    InvalidState,
    #[error("{0}")]
    StalePatch(String),
    #[error("{0}")]
    Worker(String),
    #[error("{0}")]
    Message(String),
    #[error("{operation}: {source}")]
    Context {
        operation: &'static str,
        #[source]
        source: Box<GitError>,
    },
    #[error("operation completed, but state reload failed: {source}")]
    Reload {
        #[source]
        source: Box<GitError>,
    },
    #[error("{primary}; state reload also failed: {reload}")]
    WithReloadFailure {
        #[source]
        primary: Box<GitError>,
        reload: Box<GitError>,
    },
    #[error("some paths were staged before a later step failed: {source}")]
    PartialWrite {
        #[source]
        source: Box<GitError>,
    },
}

impl GitError {
    pub fn context(self, operation: &'static str) -> Self {
        if matches!(&self, Self::Context { operation: existing, .. } if *existing == operation) {
            return self;
        }
        Self::Context {
            operation,
            source: Box::new(self),
        }
    }

    pub fn category(&self) -> GitErrorCategory {
        match self {
            Self::Native(error) => error.category,
            Self::Context { source, .. }
            | Self::Reload { source }
            | Self::PartialWrite { source } => source.category(),
            Self::WithReloadFailure { primary, .. } => primary.category(),
            Self::InvalidState => GitErrorCategory::InvalidState,
            Self::StalePatch(_) => GitErrorCategory::StalePatch,
            Self::Worker(_) => GitErrorCategory::Worker,
            _ => GitErrorCategory::Other,
        }
    }

    pub fn process(output: std::process::Output) -> Self {
        Self::Process(Arc::new(GitProcessError {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().into(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().into(),
        }))
    }
}

impl From<std::io::Error> for GitError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(Arc::new(GitIoError {
            kind: error.kind(),
            raw_os_error: error.raw_os_error(),
            message: error.to_string(),
        }))
    }
}

impl From<git2::Error> for GitError {
    fn from(error: git2::Error) -> Self {
        use git2::ErrorCode;
        let category = match error.code() {
            ErrorCode::Conflict | ErrorCode::MergeConflict | ErrorCode::Unmerged => {
                GitErrorCategory::Conflict
            }
            ErrorCode::Auth | ErrorCode::Certificate => GitErrorCategory::Authentication,
            ErrorCode::Locked => GitErrorCategory::Locked,
            ErrorCode::NotFound | ErrorCode::UnbornBranch => GitErrorCategory::NotFound,
            ErrorCode::Exists => GitErrorCategory::Exists,
            _ => GitErrorCategory::Other,
        };
        Self::Native(Arc::new(NativeGitError {
            code: error.raw_code(),
            // C enums use different signedness on Windows and Unix.
            class: i64::from(error.raw_class()),
            message: error.message().into(),
            category,
        }))
    }
}

impl From<String> for GitError {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}

impl From<&str> for GitError {
    fn from(message: &str) -> Self {
        Self::Message(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_native_conversion_preserves_owned_codes_classes_and_messages() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<GitError>();
        for (code, class, category) in [
            (
                git2::ErrorCode::Conflict,
                git2::ErrorClass::Merge,
                GitErrorCategory::Conflict,
            ),
            (
                git2::ErrorCode::Auth,
                git2::ErrorClass::Net,
                GitErrorCategory::Authentication,
            ),
            (
                git2::ErrorCode::Locked,
                git2::ErrorClass::Index,
                GitErrorCategory::Locked,
            ),
            (
                git2::ErrorCode::NotFound,
                git2::ErrorClass::Reference,
                GitErrorCategory::NotFound,
            ),
            (
                git2::ErrorCode::Exists,
                git2::ErrorClass::Reference,
                GitErrorCategory::Exists,
            ),
        ] {
            let original = git2::Error::new(code, class, "identical message");
            let raw_code = original.raw_code();
            let raw_class = i64::from(original.raw_class());
            let converted = GitError::from(original);
            assert_eq!(converted.category(), category);
            let GitError::Native(error) = converted.clone() else {
                panic!("native diagnostic")
            };
            assert_eq!(error.code, raw_code);
            assert_eq!(error.class, raw_class);
            assert_eq!(error.message, "identical message");
            assert_eq!(converted, GitError::Native(error));
        }
        // Message text alone must never manufacture a category.
        assert_eq!(
            GitError::from("authentication conflict index.lock").category(),
            GitErrorCategory::Other
        );
    }

    #[test]
    fn test_io_conversion_retains_kind_and_os_error() {
        let original = std::io::Error::from_raw_os_error(13);
        let expected_kind = original.kind();
        let expected_message = original.to_string();
        let GitError::Io(error) = GitError::from(original) else {
            panic!("I/O diagnostic")
        };
        assert_eq!(error.kind, expected_kind);
        assert_eq!(error.raw_os_error, Some(13));
        assert_eq!(error.message, expected_message);
    }
}
