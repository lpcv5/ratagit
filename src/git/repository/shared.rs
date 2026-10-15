//! Helpers shared by capability implementations; no worker routing.

use super::Git2RepoInner;
use crate::git::{DiffLine, DiffLineKind, GitError};
use std::path::PathBuf;
use std::process::Command;

/// Maximum number of diff lines returned to the UI to prevent OOM on huge files.
pub(super) const MAX_DIFF_LINES: usize = 5_000;

impl Git2RepoInner {
    pub(super) fn signature(&self) -> Result<git2::Signature<'_>, GitError> {
        match self.repo.signature() {
            Ok(sig) => Ok(sig),
            Err(_) => {
                Ok(git2::Signature::now("ratagit", "ratagit@localhost").map_err(GitError::from)?)
            }
        }
    }

    pub(super) fn repo_root(&self) -> Result<PathBuf, GitError> {
        if let Some(workdir) = self.repo.workdir() {
            return Ok(workdir.to_path_buf());
        }

        self.repo
            .path()
            .parent()
            .map(|p| p.to_path_buf())
            .ok_or(GitError::InvalidState)
    }

    /// Helper to validate path contains valid Unicode.
    pub(super) fn path_to_str<'a>(
        path: &'a std::path::Path,
        context: &str,
    ) -> Result<&'a str, GitError> {
        path.to_str()
            .ok_or_else(|| GitError::Git2(format!("{} contains invalid unicode", context)))
    }

    pub(super) fn run_git_owned(&self, args: &[String]) -> Result<String, GitError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(self.repo_root()?)
            .output()
            .map_err(|e| GitError::Git2(format!("failed to run git {:?}: {}", args, e)))?;

        if output.status.success() {
            return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };
        Err(GitError::Git2(detail))
    }
}

pub(super) fn parse_diff(diff: &git2::Diff) -> Vec<DiffLine> {
    parse_diff_with_limit(diff, MAX_DIFF_LINES)
}

pub(super) fn parse_diff_with_limit(diff: &git2::Diff, limit: usize) -> Vec<DiffLine> {
    let mut lines = Vec::new();
    let mut truncated = false;
    let _ = diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        if lines.len() >= limit {
            truncated = true;
            return false;
        }
        // Skip binary content markers (origin 'B')
        if line.origin() == 'B' {
            lines.push(DiffLine {
                kind: DiffLineKind::Header,
                content: "<binary file>".to_string(),
            });
            return true;
        }
        let content = String::from_utf8_lossy(line.content())
            .trim_end_matches('\n')
            .to_string();
        let kind = match line.origin() {
            '+' => DiffLineKind::Added,
            '-' => DiffLineKind::Removed,
            'H' | 'F' => DiffLineKind::Header,
            _ => DiffLineKind::Context,
        };
        lines.push(DiffLine { kind, content });
        true
    });
    if truncated {
        lines.push(DiffLine {
            kind: DiffLineKind::Header,
            content: format!("... diff truncated at {} lines ...", limit),
        });
    }
    lines
}
