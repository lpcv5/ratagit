use crate::app::AppRuntime;
use crate::background::{BackgroundCompleted, BackgroundLoad, BackgroundPayload, TaskRequest};
use crate::file_tree::FileTreeNodeStatus;
use crate::flux::action::{Action, DomainAction, SystemAction};
use crate::git::{GitError, GitRepository};
use std::path::PathBuf;

#[derive(Debug, Clone)]
/// Owned repository I/O. State decisions travel as RuntimeEvent actions.
pub enum EffectRequest {
    ApplyHunks {
        id: u64,
        patch: std::sync::Arc<crate::git::patch::Patch>,
        ids: Vec<crate::git::patch::HunkId>,
        direction: crate::git::patch::PatchDirection,
    },
    StageHunks {
        patch: std::sync::Arc<crate::git::patch::Patch>,
        file: usize,
        hunks: Vec<usize>,
    },
    ControlRepositoryOperation {
        id: u64,
        control: crate::git::operation::OperationControl,
    },
    LoadBackground {
        request: TaskRequest,
    },
    ToggleStageSelection {
        stage: Vec<PathBuf>,
        unstage: Vec<PathBuf>,
    },
    PrepareCommitFromVisualSelection {
        paths: Vec<PathBuf>,
    },
    FetchRemote,
    StageFile(PathBuf),
    StagePaths(Vec<PathBuf>),
    UnstageFile(PathBuf),
    DiscardPaths(Vec<PathBuf>),
    CreateBranch(String),
    CheckoutBranch {
        name: String,
        auto_stash: bool,
    },
    DeleteBranch(String),
    Commit(String),
    StashPush {
        message: String,
        paths: Vec<PathBuf>,
    },
    StashApply(usize),
    StashPop(usize),
    StashDrop(usize),
}

impl EffectRequest {
    pub fn is_write(&self) -> bool {
        !matches!(self, Self::LoadBackground { .. } | Self::FetchRemote)
    }
}

fn receive<T>(
    request: Result<std::sync::mpsc::Receiver<Result<T, GitError>>, GitError>,
) -> Result<T, GitError> {
    request?.recv().map_err(|e| GitError::Git2(e.to_string()))?
}

fn domain(action: DomainAction) -> Vec<Action> {
    vec![Action::Domain(action)]
}

/// Effects can access execution dependencies, never mutable application state.
pub async fn run(request: EffectRequest, runtime: &AppRuntime) -> Vec<Action> {
    let repo = runtime.repo.clone();
    let fallback = request.clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    // Git jobs may be uncancellable (e.g. libgit2 network I/O). Their completion
    // wait must not keep Tokio's shutdown blocked after the TUI has exited.
    let launched = std::thread::Builder::new()
        .name("ratagit-effect".into())
        .spawn(move || {
            let result =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| execute(request, &*repo)))
                    .map_err(|_| "effect worker panicked".to_string());
            let _ = tx.send(result);
        });
    if let Err(err) = launched {
        return failed(fallback, format!("effect worker failed to start: {err}"));
    }
    match rx.await {
        Ok(Ok(actions)) => actions,
        Ok(Err(err)) => failed(fallback, err),
        Err(err) => failed(fallback, format!("effect worker disconnected: {err}")),
    }
}

/// Shared by production execution and deterministic test drivers.
pub(crate) fn execute(request: EffectRequest, repo: &dyn GitRepository) -> Vec<Action> {
    match request {
        EffectRequest::ApplyHunks {
            id,
            patch,
            ids,
            direction,
        } => domain(DomainAction::ApplyHunksFinished {
            id,
            direction,
            result: repo
                .apply_hunks(&patch, &ids, direction)
                .map_err(|e| e.to_string()),
        }),
        EffectRequest::StageHunks { patch, file, hunks } => {
            domain(DomainAction::StageHunksFinished {
                result: repo
                    .stage_hunks(&patch, file, &hunks)
                    .map_err(|e| e.to_string()),
            })
        }
        EffectRequest::ControlRepositoryOperation { id, control } => {
            let mut result = repo.control_operation(control).map_err(|e| e.to_string());
            let observed = match repo.operation_snapshot() {
                Ok(snapshot) => Some(snapshot),
                Err(error) => {
                    if result.is_ok() {
                        result = Err(format!(
                            "operation completed, but state reload failed: {error}"
                        ));
                    }
                    None
                }
            };
            domain(DomainAction::RepositoryOperationFinished {
                id,
                result,
                observed,
            })
        }
        EffectRequest::LoadBackground { request } => vec![Action::System(
            SystemAction::BackgroundCompleted(Box::new(BackgroundCompleted {
                result: load_background(request.load.clone(), repo)
                    .map(std::sync::Arc::new)
                    .map_err(|e| e.to_string()),
                request,
            })),
        )],
        EffectRequest::StageFile(path) => {
            let result = receive(repo.stage_async(path.clone())).map_err(|e| e.to_string());
            domain(DomainAction::StageFileFinished { path, result })
        }
        EffectRequest::StagePaths(paths) => domain(DomainAction::StagePathsFinished {
            result: receive(repo.stage_paths_async(paths.clone())).map_err(|e| e.to_string()),
        }),
        EffectRequest::UnstageFile(path) => {
            let result = receive(repo.unstage_async(path.clone())).map_err(|e| e.to_string());
            domain(DomainAction::UnstageFileFinished { path, result })
        }
        EffectRequest::DiscardPaths(paths) => {
            let result =
                receive(repo.discard_paths_async(paths.clone())).map_err(|e| e.to_string());
            domain(DomainAction::DiscardPathsFinished { paths, result })
        }
        EffectRequest::ToggleStageSelection { stage, unstage } => {
            let result = (|| {
                if !stage.is_empty() {
                    receive(repo.stage_paths_async(stage.clone()))?;
                }
                if !unstage.is_empty() {
                    repo.unstage_paths(&unstage)?;
                }
                Ok::<_, GitError>((stage.len(), unstage.len()))
            })()
            .map_err(|e| e.to_string());
            domain(DomainAction::ToggleStageSelectionFinished { result })
        }
        EffectRequest::PrepareCommitFromVisualSelection { paths } => {
            let result = if paths.is_empty() {
                Ok(0)
            } else {
                receive(repo.stage_paths_async(paths.clone()))
                    .map(|()| paths.len())
                    .map_err(|e| e.to_string())
            };
            domain(DomainAction::PrepareCommitFromSelectionFinished { result })
        }
        EffectRequest::FetchRemote => {
            let result = repo
                .fetch_default_async()
                .and_then(|rx| rx.recv().map_err(|e| GitError::Git2(e.to_string()))?)
                .map_err(|e| e.to_string());
            domain(DomainAction::FetchRemoteFinished(result))
        }
        EffectRequest::CreateBranch(name) => {
            let result = receive(repo.create_branch_async(name.clone())).map_err(|e| e.to_string());
            domain(DomainAction::CreateBranchFinished { name, result })
        }
        EffectRequest::CheckoutBranch { name, auto_stash } => {
            let result = if auto_stash {
                receive(repo.checkout_branch_with_auto_stash_async(name.clone()))
            } else {
                receive(repo.checkout_branch_async(name.clone()))
            }
            .map_err(|e| e.to_string());
            domain(DomainAction::CheckoutBranchFinished {
                name,
                auto_stash,
                result,
            })
        }
        EffectRequest::DeleteBranch(name) => {
            let result = receive(repo.delete_branch_async(name.clone())).map_err(|e| e.to_string());
            domain(DomainAction::DeleteBranchFinished { name, result })
        }
        EffectRequest::Commit(message) => {
            let result = receive(repo.commit_async(message.clone())).map_err(|e| e.to_string());
            domain(DomainAction::CommitFinished { message, result })
        }
        EffectRequest::StashPush { message, paths } => {
            let result = receive(repo.stash_push_paths_async(paths.clone(), message.clone()))
                .map_err(|e| e.to_string());
            domain(DomainAction::StashPushFinished { message, result })
        }
        EffectRequest::StashApply(index) => domain(DomainAction::StashApplyFinished {
            index,
            result: receive(repo.stash_apply_async(index)).map_err(|e| e.to_string()),
        }),
        EffectRequest::StashPop(index) => domain(DomainAction::StashPopFinished {
            index,
            result: receive(repo.stash_pop_async(index)).map_err(|e| e.to_string()),
        }),
        EffectRequest::StashDrop(index) => domain(DomainAction::StashDropFinished {
            index,
            result: receive(repo.stash_drop_async(index)).map_err(|e| e.to_string()),
        }),
    }
}

fn load_background(
    load: BackgroundLoad,
    repo: &dyn GitRepository,
) -> Result<BackgroundPayload, GitError> {
    use crate::app::diff::DiffTarget;
    Ok(match load {
        BackgroundLoad::Status {
            fast,
            operation_epoch,
        } => BackgroundPayload::Status {
            status: (if fast {
                receive(repo.status_fast_async())?
            } else {
                receive(repo.status_async())?
            })
            .into(),
            fast,
            operation_epoch,
            operation: repo.operation_snapshot()?,
        },
        BackgroundLoad::Branches => {
            BackgroundPayload::Branches(receive(repo.branches_async())?.into())
        }
        BackgroundLoad::Stashes => {
            BackgroundPayload::Stashes(receive(repo.stashes_async())?.into())
        }
        BackgroundLoad::Commits { limit, fast } => {
            if fast {
                BackgroundPayload::CommitsFast(receive(repo.commits_fast_async(limit))?.into())
            } else {
                BackgroundPayload::Commits(receive(repo.commits_async(limit))?.into())
            }
        }
        BackgroundLoad::BranchCommits { branch, limit } => {
            let items = receive(repo.commits_for_branch_async(&branch, limit))?;
            BackgroundPayload::BranchCommits {
                branch,
                items: items.into(),
            }
        }
        BackgroundLoad::Diff { cache_key, target } => {
            use crate::git::patch::PatchRequest;
            let patch_request = match &target {
                DiffTarget::None | DiffTarget::Branch { .. } => None,
                DiffTarget::File { path, status } => Some(match status {
                    FileTreeNodeStatus::Staged(_) => PatchRequest::Index(path.clone()),
                    FileTreeNodeStatus::Untracked => PatchRequest::Untracked(path.clone()),
                    FileTreeNodeStatus::Unstaged(_) => PatchRequest::Worktree(path.clone()),
                    FileTreeNodeStatus::Directory => PatchRequest::Directory(path.clone()),
                }),
                DiffTarget::Directory { path } => Some(PatchRequest::Directory(path.clone())),
                DiffTarget::Commit { oid, path } => Some(PatchRequest::Commit {
                    oid: oid.clone(),
                    path: path.clone(),
                }),
                DiffTarget::Stash { index, path } => Some(PatchRequest::Stash {
                    index: *index,
                    path: path.clone(),
                }),
            };
            let structured = patch_request
                .map(|r| crate::git::patch::document(repo, r))
                .transpose()?
                .flatten();
            let diff = if let Some(document) = structured {
                document
            } else {
                let diff = match target.clone() {
                    DiffTarget::None => Vec::new(),
                    DiffTarget::Branch { name } => receive(repo.branch_log_async(name, 20))?,
                    DiffTarget::File { path, status } => match status {
                        FileTreeNodeStatus::Staged(_) => receive(repo.diff_staged_async(path))?,
                        FileTreeNodeStatus::Untracked => receive(repo.diff_untracked_async(path))?,
                        FileTreeNodeStatus::Unstaged(_) => receive(repo.diff_unstaged_async(path))?,
                        FileTreeNodeStatus::Directory => receive(repo.diff_directory_async(path))?,
                    },
                    DiffTarget::Directory { path } => receive(repo.diff_directory_async(path))?,
                    DiffTarget::Commit { oid, path } => {
                        receive(repo.commit_diff_scoped_async(oid, path))?
                    }
                    DiffTarget::Stash { index, path } => {
                        receive(repo.stash_diff_async(index, path))?
                    }
                };
                diff.into()
            };
            BackgroundPayload::Diff { cache_key, diff }
        }
        BackgroundLoad::CommitFiles { oid } => {
            let files = repo.commit_files(&oid)?;
            BackgroundPayload::CommitFiles {
                oid,
                files: files.into(),
            }
        }
        BackgroundLoad::StashFiles { index } => BackgroundPayload::StashFiles {
            index,
            files: repo.stash_files(index)?.into(),
        },
    })
}

/// Preserve operation identity when a blocking task panics.
fn failed(request: EffectRequest, error: String) -> Vec<Action> {
    match request {
        EffectRequest::ApplyHunks { id, direction, .. } => {
            domain(DomainAction::ApplyHunksFinished {
                id,
                direction,
                result: Err(error),
            })
        }
        EffectRequest::StageHunks { .. } => {
            domain(DomainAction::StageHunksFinished { result: Err(error) })
        }
        EffectRequest::ControlRepositoryOperation { id, .. } => {
            domain(DomainAction::RepositoryOperationFinished {
                id,
                result: Err(error),
                observed: None,
            })
        }
        EffectRequest::LoadBackground { request, .. } => vec![Action::System(
            SystemAction::BackgroundCompleted(Box::new(BackgroundCompleted {
                request,
                result: Err(error),
            })),
        )],
        EffectRequest::StageFile(path) => domain(DomainAction::StageFileFinished {
            path,
            result: Err(error),
        }),
        EffectRequest::StagePaths(_) => {
            domain(DomainAction::StagePathsFinished { result: Err(error) })
        }
        EffectRequest::UnstageFile(path) => domain(DomainAction::UnstageFileFinished {
            path,
            result: Err(error),
        }),
        EffectRequest::DiscardPaths(paths) => domain(DomainAction::DiscardPathsFinished {
            paths,
            result: Err(error),
        }),
        EffectRequest::ToggleStageSelection { .. } => {
            domain(DomainAction::ToggleStageSelectionFinished { result: Err(error) })
        }
        EffectRequest::PrepareCommitFromVisualSelection { .. } => {
            domain(DomainAction::PrepareCommitFromSelectionFinished { result: Err(error) })
        }
        EffectRequest::FetchRemote => domain(DomainAction::FetchRemoteFinished(Err(error))),
        EffectRequest::CreateBranch(name) => domain(DomainAction::CreateBranchFinished {
            name,
            result: Err(error),
        }),
        EffectRequest::CheckoutBranch { name, auto_stash } => {
            domain(DomainAction::CheckoutBranchFinished {
                name,
                auto_stash,
                result: Err(error),
            })
        }
        EffectRequest::DeleteBranch(name) => domain(DomainAction::DeleteBranchFinished {
            name,
            result: Err(error),
        }),
        EffectRequest::Commit(message) => domain(DomainAction::CommitFinished {
            message,
            result: Err(error),
        }),
        EffectRequest::StashPush { message, .. } => domain(DomainAction::StashPushFinished {
            message,
            result: Err(error),
        }),
        EffectRequest::StashApply(index) => domain(DomainAction::StashApplyFinished {
            index,
            result: Err(error),
        }),
        EffectRequest::StashPop(index) => domain(DomainAction::StashPopFinished {
            index,
            result: Err(error),
        }),
        EffectRequest::StashDrop(index) => domain(DomainAction::StashDropFinished {
            index,
            result: Err(error),
        }),
    }
}
