//! Repository operation observation and control on the primary worker.

use super::Git2Repository;
use crate::git::{GitError, OperationRepository};
use std::process::Command;

impl OperationRepository for Git2Repository {
    fn operation_snapshot(&self) -> Result<crate::git::operation::RepositoryOperation, GitError> {
        self.spawn_repo_job("operation_snapshot", |inner| {
            crate::git::operation_backend::snapshot(&inner.repo)
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".into()))?
    }

    fn control_operation(
        &self,
        control: crate::git::operation::OperationControl,
    ) -> Result<(), GitError> {
        self.spawn_repo_job("control_operation", move |inner| {
            let observed = crate::git::operation_backend::snapshot(&inner.repo)?;
            let args = crate::git::operation_backend::arguments(&observed, control)?;
            let output = Command::new("git")
                .args(args)
                .env("GIT_EDITOR", "true")
                .env("GIT_TERMINAL_PROMPT", "0")
                .current_dir(inner.repo_root()?)
                .output()
                .map_err(|e| GitError::Git2(e.to_string()))?;
            if output.status.success() {
                Ok(())
            } else {
                Err(GitError::Git2(
                    String::from_utf8_lossy(&output.stderr).trim().into(),
                ))
            }
        })?
        .recv()
        .map_err(|_| GitError::Git2("worker disconnected".into()))?
    }
}
