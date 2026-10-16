use super::*;
use ratagit_core::patch::*;
use ratagit_core::workflow::{Operation, commit_files::PatchAction};

impl MockGitBackend {
    pub(super) fn commit_files_patch(
        &mut self,
        oid: String,
        paths: Vec<std::path::PathBuf>,
        request: PatchRequest,
    ) -> Result<Patch, GitError> {
        self.operations.push(format!(
            "commit-patch:{oid}:{}",
            paths
                .iter()
                .map(|p| p.to_string_lossy())
                .collect::<Vec<_>>()
                .join(",")
        ));
        if !self
            .snapshot
            .commits
            .iter()
            .any(|c| commit_matches(c, &oid))
        {
            return Err(GitError::new("commit not found"));
        }
        let files = paths
            .into_iter()
            .map(|path| {
                let header = format!(
                    "diff --git a/{0} b/{0}\n--- a/{0}\n+++ b/{0}\n",
                    path.display()
                )
                .into_bytes();
                let mut encoded = header.clone();
                encoded.extend_from_slice(b"@@ -1 +1 @@\n-old mock content\n+new mock content\n");
                let side = PatchSide {
                    path: Some(path),
                    oid: None,
                    mode: 0o100644,
                };
                PatchFile {
                    header,
                    source: PatchSource::Commit(oid.clone()),
                    change: PatchChange::Modified,
                    old: side.clone(),
                    new: side,
                    binary: false,
                    encoded: Some(encoded),
                    hunks: vec![PatchHunk {
                        old_start: 1,
                        old_lines: 1,
                        new_start: 1,
                        new_lines: 1,
                        header: b"@@ -1 +1 @@\n".to_vec(),
                        lines: vec![
                            PatchLine {
                                kind: PatchLineKind::Removed,
                                old_line: Some(1),
                                new_line: None,
                                content: b"old mock content\n".to_vec(),
                            },
                            PatchLine {
                                kind: PatchLineKind::Added,
                                old_line: None,
                                new_line: Some(1),
                                content: b"new mock content\n".to_vec(),
                            },
                        ],
                    }],
                }
            })
            .collect();
        Ok(Patch {
            request,
            files,
            truncated: false,
            preview: None,
        })
    }

    pub(super) fn commit_file_operation(
        &mut self,
        operation: &ratagit_core::extras::RepositoryOperation,
    ) -> Result<(), GitError> {
        let ratagit_core::extras::RepositoryOperation::Workflow(op) = operation else {
            return Err(GitError::new("operation unavailable in mock"));
        };
        let (paths, rewrite) = match op {
            Operation::CustomPatch { patch, action } => {
                patch.validate().map_err(GitError::new)?;
                self.operations.push(format!(
                    "custom-patch-{action:?}:{}:{}",
                    patch.oid,
                    patch.files.len()
                ));
                (
                    patch
                        .files
                        .iter()
                        .map(|selected| {
                            ratagit_core::custom_patch::file_key(&selected.file)
                                .map(str::to_string)
                                .map_err(GitError::new)
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                    *action == PatchAction::Discard,
                )
            }
            Operation::CommitFileCheckout { oid, paths } => {
                if paths
                    .iter()
                    .any(|path| self.snapshot.files.iter().any(|f| &f.path == path))
                {
                    return Err(GitError::new("selected paths have local changes"));
                }
                self.operations
                    .push(format!("checkout-commit-files:{oid}:{}", paths.join(",")));
                (paths.clone(), false)
            }
            Operation::CommitFileDiscard { oid, paths } => {
                self.operations
                    .push(format!("discard-commit-files:{oid}:{}", paths.join(",")));
                (paths.clone(), true)
            }
            Operation::CommitPatch { patch, ids, action } => {
                ratagit_core::workflow::commit_files::validate(patch, ids, *action)
                    .map_err(GitError::new)?;
                self.operations
                    .push(format!("commit-patch-{action:?}:{}", ids.len()));
                (
                    ids.iter()
                        .filter_map(|id| {
                            patch.files[id.file]
                                .new
                                .path
                                .as_ref()
                                .or(patch.files[id.file].old.path.as_ref())
                        })
                        .map(|p| p.to_string_lossy().to_string())
                        .collect(),
                    *action == PatchAction::Discard,
                )
            }
            _ => return Err(GitError::new("operation unavailable in mock")),
        };
        if rewrite {
            if !self.snapshot.files.is_empty() {
                return Err(GitError::new("clean worktree required"));
            }
            // Stable rewritten identity for deterministic scenario assertions.
            for commit in &mut self.snapshot.commits {
                commit.full_id = format!("rewritten-{}", commit.full_id);
            }
        } else {
            for path in paths {
                if let Some(file) = self.snapshot.files.iter_mut().find(|f| f.path == path) {
                    file.unstaged = true;
                } else {
                    self.snapshot.files.push(ratagit_core::FileEntry {
                        path,
                        staged: false,
                        unstaged: true,
                        untracked: false,
                        status: CommitFileStatus::Modified,
                        conflicted: false,
                    });
                }
            }
            refresh_status_summary(&mut self.snapshot);
        }
        Ok(())
    }
}
