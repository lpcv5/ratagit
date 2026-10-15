//! Patch loading and application, worktree previews and historical diffs.

use super::shared::{parse_diff, parse_diff_with_limit, MAX_DIFF_LINES};
use super::{Git2RepoInner, Git2Repository};
use crate::git::{DiffLine, DiffLineKind, GitError};
use std::path::PathBuf;
use std::sync::mpsc::Receiver;

/// Maximum file size for untracked file preview, in bytes.
const MAX_UNTRACKED_PREVIEW_BYTES: usize = 256 * 1024; // 256 KiB

impl Git2RepoInner {
    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""))
            .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64); // treat files larger than limit as binary

        let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());

        let mut opts = git2::DiffOptions::new();
        opts.pathspec(path.to_str().unwrap_or(""))
            .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64); // treat files larger than limit as binary

        let diff = self
            .repo
            .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
        Ok(parse_diff(&diff))
    }

    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let workdir = self.repo.workdir().ok_or(GitError::InvalidState)?;
        let full_path = workdir.join(path);

        // Check file size before reading to avoid OOM on huge files
        let file_size = std::fs::metadata(&full_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);

        let content = if file_size == 0 {
            String::new()
        } else if file_size > MAX_UNTRACKED_PREVIEW_BYTES {
            format!(
                "<file too large to preview: {} bytes, limit {} bytes>",
                file_size, MAX_UNTRACKED_PREVIEW_BYTES
            )
        } else {
            match std::fs::read(&full_path) {
                Ok(bytes) => {
                    // Detect binary content by checking for null bytes
                    if bytes.contains(&0u8) {
                        format!("<binary file: {} bytes>", bytes.len())
                    } else {
                        String::from_utf8_lossy(&bytes).into_owned()
                    }
                }
                Err(_) => "<unreadable file>".to_string(),
            }
        };

        let header = format!("--- /dev/null\n+++ b/{}", path.display());
        let mut lines = vec![DiffLine {
            kind: DiffLineKind::Header,
            content: header,
        }];
        for (i, line) in content.lines().enumerate() {
            if i >= MAX_DIFF_LINES {
                lines.push(DiffLine {
                    kind: DiffLineKind::Header,
                    content: format!("... preview truncated at {} lines ...", MAX_DIFF_LINES),
                });
                break;
            }
            lines.push(DiffLine {
                kind: DiffLineKind::Added,
                content: line.to_string(),
            });
        }
        Ok(lines)
    }

    fn diff_directory(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path_str = Self::path_to_str(path, "path")?;
        let mut lines = Vec::new();
        let mut remaining = MAX_DIFF_LINES;

        let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());

        if remaining > 0 {
            let mut opts = git2::DiffOptions::new();
            opts.pathspec(path_str)
                .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64);
            let diff = self
                .repo
                .diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?;
            let staged = parse_diff_with_limit(&diff, remaining);
            remaining = remaining.saturating_sub(staged.len());
            lines.extend(staged);
        }

        if remaining > 0 {
            let mut opts = git2::DiffOptions::new();
            opts.pathspec(path_str)
                .max_size(MAX_UNTRACKED_PREVIEW_BYTES as i64);
            let diff = self.repo.diff_index_to_workdir(None, Some(&mut opts))?;
            let unstaged = parse_diff_with_limit(&diff, remaining);
            remaining = remaining.saturating_sub(unstaged.len());
            lines.extend(unstaged);
        }

        if remaining > 0 {
            let untracked = self.status()?;
            for entry in untracked
                .untracked
                .iter()
                .filter(|entry| entry.path.starts_with(path))
            {
                if remaining == 0 {
                    break;
                }
                let mut file_lines = self.diff_untracked(&entry.path)?;
                if file_lines.len() > remaining {
                    file_lines.truncate(remaining);
                    lines.extend(file_lines);
                    lines.push(DiffLine {
                        kind: DiffLineKind::Header,
                        content: format!(
                            "... directory diff truncated at {} lines ...",
                            MAX_DIFF_LINES
                        ),
                    });
                    break;
                }
                remaining = remaining.saturating_sub(file_lines.len());
                lines.extend(file_lines);
            }
        }

        Ok(lines)
    }

    fn commit_diff_scoped(
        &self,
        oid: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let oid = git2::Oid::from_str(oid)?;
        let commit = self.repo.find_commit(oid)?;
        let tree = commit.tree()?;
        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            None
        };
        let mut opts = git2::DiffOptions::new();
        if let Some(path) = path {
            let path_str = Self::path_to_str(path, "commit path")?;
            opts.pathspec(path_str);
        }
        let diff =
            match self
                .repo
                .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut opts))
            {
                Ok(diff) => diff,
                Err(err) => {
                    return Ok(vec![DiffLine {
                        kind: DiffLineKind::Header,
                        content: format!("diff unavailable: {}", err),
                    }]);
                }
            };
        Ok(parse_diff(&diff))
    }
}

impl crate::git::PatchRepository for Git2Repository {
    fn apply_lines(
        &self,
        patch: &crate::git::patch::Patch,
        ids: &[crate::git::patch::LineId],
        direction: crate::git::patch::PatchDirection,
    ) -> Result<(), GitError> {
        let patch = patch.clone();
        let ids = ids.to_vec();
        self.spawn_repo_job("apply_lines", move |inner| {
            crate::git::patch_backend::apply_lines(&mut inner.repo, &patch, &ids, direction)
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".into()))?
    }
    fn apply_hunks(
        &self,
        patch: &crate::git::patch::Patch,
        ids: &[crate::git::patch::HunkId],
        direction: crate::git::patch::PatchDirection,
    ) -> Result<(), GitError> {
        let patch = patch.clone();
        let ids = ids.to_vec();
        self.spawn_repo_job("apply_hunks", move |inner| {
            crate::git::patch_backend::apply(&mut inner.repo, &patch, &ids, direction)
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".into()))?
    }
    fn patch(
        &self,
        request: crate::git::patch::PatchRequest,
    ) -> Result<Option<crate::git::patch::Patch>, GitError> {
        self.spawn_repo_job("patch", move |inner| {
            crate::git::patch_backend::load(&mut inner.repo, request).map(Some)
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".into()))?
    }

    fn stage_hunks(
        &self,
        patch: &crate::git::patch::Patch,
        file: usize,
        hunks: &[usize],
    ) -> Result<(), GitError> {
        let patch = patch.clone();
        let hunks = hunks.to_vec();
        self.spawn_repo_job("stage_hunks", move |inner| {
            crate::git::patch_backend::stage(&mut inner.repo, &patch, file, &hunks)
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".into()))?
    }

    fn diff_unstaged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_unstaged", move |inner| inner.diff_unstaged(&path))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn diff_unstaged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_unstaged", move |inner| inner.diff_unstaged(&path))
    }

    fn diff_staged(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_staged", move |inner| inner.diff_staged(&path))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn diff_staged_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_staged", move |inner| inner.diff_staged(&path))
    }

    fn diff_untracked(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_untracked", move |inner| inner.diff_untracked(&path))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn diff_untracked_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_untracked", move |inner| inner.diff_untracked(&path))
    }

    fn diff_directory(&self, path: &std::path::Path) -> Result<Vec<DiffLine>, GitError> {
        let path = path.to_path_buf();
        self.spawn_repo_job("diff_directory", move |inner| inner.diff_directory(&path))?
            .recv()
            .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn diff_directory_async(
        &self,
        path: PathBuf,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job("diff_directory", move |inner| inner.diff_directory(&path))
    }

    fn commit_diff_scoped(
        &self,
        oid: &str,
        path: Option<&std::path::Path>,
    ) -> Result<Vec<DiffLine>, GitError> {
        let oid = oid.to_string();
        let path = path.map(|p| p.to_path_buf());
        self.spawn_repo_job_external("commit_diff_scoped", move |inner| {
            inner.commit_diff_scoped(&oid, path.as_deref())
        })?
        .recv()
        .map_err(|_| GitError::Worker("worker disconnected".to_string()))?
    }

    fn commit_diff_scoped_async(
        &self,
        oid: String,
        path: Option<PathBuf>,
    ) -> Result<Receiver<Result<Vec<DiffLine>, GitError>>, GitError> {
        self.spawn_repo_job_external("commit_diff_scoped", move |inner| {
            inner.commit_diff_scoped(&oid, path.as_deref())
        })
    }
}
