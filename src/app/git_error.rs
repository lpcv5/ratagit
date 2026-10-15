use crate::git::{GitError, GitErrorCategory};

/// Add guidance only at presentation; never infer categories from diagnostic text.
pub(crate) fn message(error: &GitError) -> String {
    let hint = hint(error);
    match hint {
        Some(hint) => format!("Hint: {hint} Details: {error}"),
        None => error.to_string(),
    }
}

fn hint(error: &GitError) -> Option<&'static str> {
    match error {
        GitError::Context { source, .. } => return hint(source),
        GitError::Reload { .. } => return Some("The operation completed. Refresh repository state before taking further action; do not repeat the operation."),
        GitError::WithReloadFailure { .. } => return Some("Refresh repository state and inspect the operation before trying again."),
        GitError::PartialWrite { .. } => return Some("Refresh repository state and review which paths were staged before repeating the selection operation."),
        GitError::Io(error) if error.kind == std::io::ErrorKind::PermissionDenied => return Some("Check filesystem permissions and access to the repository."),
        _ => {}
    }
    match error.category() {
        GitErrorCategory::Conflict => Some("Resolve conflicts and refresh repository state before continuing."),
        GitErrorCategory::Authentication => Some("Check credentials, remote access permissions, and certificate configuration."),
        GitErrorCategory::Locked => Some("Another Git operation may be active. Wait for it to finish and inspect repository state before trying again."),
        GitErrorCategory::NotFound => Some("Refresh repository state and verify the selected path or reference still exists."),
        GitErrorCategory::Exists => Some("Choose a different name or inspect the existing reference."),
        GitErrorCategory::InvalidState => Some("Refresh repository state and check the active Git operation."),
        GitErrorCategory::StalePatch => Some("Reload the diff and select hunks again."),
        GitErrorCategory::Worker => Some("The operation outcome may be unknown. Refresh or reopen the repository and inspect its state before repeating a write."),
        GitErrorCategory::Other => None,
    }
}
