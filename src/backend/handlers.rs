use anyhow::Result;

use super::git_ops::GitRepo;
use super::{CommandEnvelope, EventEnvelope, FrontendEvent};
use crate::shared::path_utils::{
    dedupe_targets_parent_first, diff_target_label, diff_target_pathspec,
};
use tokio::sync::mpsc::Sender;

pub fn send_event(event_tx: &Sender<EventEnvelope>, envelope: EventEnvelope) {
    if let Err(e) = event_tx.try_send(envelope) {
        eprintln!("ratagit: event dropped (queue full): {e}");
    }
}

/// 命令处理器 trait
pub trait CommandHandler: Send + Sync {
    /// 执行命令并发送事件
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()>;

    /// 是否需要可变仓库引用
    fn needs_mut_repo(&self) -> bool {
        false
    }

    /// 执行命令并发送事件（可变引用版本）
    fn handle_mut(
        &self,
        _envelope: &CommandEnvelope,
        _repo: &mut GitRepo,
        _event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        unreachable!("handle_mut should only be called when needs_mut_repo returns true")
    }
}

/// 刷新状态处理器
pub struct RefreshStatusHandler;
impl CommandHandler for RefreshStatusHandler {
    fn handle(
        &self,
        _envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::get_status_files(repo) {
            Ok(files) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(None, FrontendEvent::FilesUpdated { files }),
                );
            }
            Err(error) => send_error(event_tx, None, "status", error),
        }
        Ok(())
    }
}

/// 刷新分支处理器
pub struct RefreshBranchesHandler;
impl CommandHandler for RefreshBranchesHandler {
    fn handle(
        &self,
        _envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::get_branches(repo) {
            Ok(branches) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(None, FrontendEvent::BranchesUpdated { branches }),
                );
            }
            Err(error) => send_error(event_tx, None, "branches", error),
        }
        Ok(())
    }
}

/// 刷新提交处理器
pub struct RefreshCommitsHandler;
impl CommandHandler for RefreshCommitsHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let limit =
            if let crate::backend::BackendCommand::RefreshCommits { limit } = &envelope.command {
                *limit
            } else {
                30
            };

        match super::git_ops::get_commits(repo, limit) {
            Ok(commits) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(None, FrontendEvent::CommitsUpdated { commits }),
                );
            }
            Err(error) => send_error(event_tx, None, "commits", error),
        }
        Ok(())
    }
}

/// 刷新贮藏处理器（需要可变引用）
pub struct RefreshStashesHandler;
impl CommandHandler for RefreshStashesHandler {
    fn handle(
        &self,
        _envelope: &CommandEnvelope,
        _repo: &GitRepo,
        _event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        // 这个实现不会被调用，因为 needs_mut_repo 返回 true
        unreachable!()
    }

    fn needs_mut_repo(&self) -> bool {
        true
    }

    fn handle_mut(
        &self,
        _envelope: &CommandEnvelope,
        repo: &mut GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::get_stashes(repo) {
            Ok(stashes) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(None, FrontendEvent::StashesUpdated { stashes }),
                );
            }
            Err(error) => send_error(event_tx, None, "stashes", error),
        }
        Ok(())
    }
}

/// 获取差异处理器
pub struct GetDiffHandler;
impl CommandHandler for GetDiffHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let file_path =
            if let crate::backend::BackendCommand::GetDiff { file_path } = &envelope.command {
                file_path.clone()
            } else {
                return Ok(());
            };

        match super::git_ops::get_diff(repo, &file_path) {
            Ok(diff) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::DiffLoaded {
                            request_id: envelope.request_id,
                            file_path,
                            diff,
                        },
                    ),
                );
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "diff", error),
        }
        Ok(())
    }
}

/// 获取多目标差异处理器（支持目录去重）
pub struct GetDiffBatchHandler;
impl CommandHandler for GetDiffBatchHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let targets =
            if let crate::backend::BackendCommand::GetDiffBatch { targets } = &envelope.command {
                targets.clone()
            } else {
                return Ok(());
            };

        let deduped = dedupe_targets_parent_first(&targets);
        let mut sections = Vec::new();
        for target in deduped {
            let label = diff_target_label(&target);
            let pathspec = diff_target_pathspec(&target);
            let diff = super::git_ops::get_diff(repo, &pathspec)?;
            sections.push(format!("===== {label} =====\n{diff}"));
        }

        let headline = format!("Selected Targets ({})", targets.len());
        let diff = if sections.is_empty() {
            "No selected targets for diff preview.".to_string()
        } else {
            sections.join("\n\n")
        };

        send_event(
            event_tx,
            EventEnvelope::new(
                Some(envelope.request_id),
                FrontendEvent::DiffLoaded {
                    request_id: envelope.request_id,
                    file_path: headline,
                    diff,
                },
            ),
        );
        Ok(())
    }
}

/// 获取 commit 文件列表处理器
pub struct GetCommitFilesHandler;
impl CommandHandler for GetCommitFilesHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let commit_id = if let crate::backend::BackendCommand::GetCommitFiles { commit_id } =
            &envelope.command
        {
            commit_id.clone()
        } else {
            return Ok(());
        };

        // 先找到对应的 commit
        let commits = super::git_ops::get_commits(repo, 100)?;
        if let Some(commit) = commits.iter().find(|c| c.id == commit_id) {
            match super::git_ops::get_commit_files(repo, commit) {
                Ok(files) => {
                    send_event(
                        event_tx,
                        EventEnvelope::new(
                            Some(envelope.request_id),
                            FrontendEvent::CommitFilesLoaded {
                                request_id: envelope.request_id,
                                commit_id,
                                files,
                            },
                        ),
                    );
                }
                Err(error) => {
                    send_error(event_tx, Some(envelope.request_id), "commit files", error)
                }
            }
        } else {
            send_error(
                event_tx,
                Some(envelope.request_id),
                "commit files",
                anyhow::anyhow!("Commit not found: {}", commit_id),
            );
        }
        Ok(())
    }
}

/// 获取 commit 某路径差异处理器（支持文件/目录）
pub struct GetCommitDiffHandler;
impl CommandHandler for GetCommitDiffHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let (commit_id, path, is_dir) = if let crate::backend::BackendCommand::GetCommitDiff {
            commit_id,
            path,
            is_dir,
        } = &envelope.command
        {
            (commit_id.clone(), path.clone(), *is_dir)
        } else {
            return Ok(());
        };

        match super::git_ops::get_commit_diff(repo, &commit_id, &path, is_dir) {
            Ok(diff) => {
                let short_id: String = commit_id.chars().take(8).collect();
                let target = if is_dir {
                    format!("{short_id}:{path}/")
                } else {
                    format!("{short_id}:{path}")
                };

                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::DiffLoaded {
                            request_id: envelope.request_id,
                            file_path: target,
                            diff,
                        },
                    ),
                );
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "commit diff", error),
        }
        Ok(())
    }
}

/// 获取 commit 多路径差异处理器（支持目录去重）
pub struct GetCommitDiffBatchHandler;
impl CommandHandler for GetCommitDiffBatchHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let (commit_id, targets) =
            if let crate::backend::BackendCommand::GetCommitDiffBatch { commit_id, targets } =
                &envelope.command
            {
                (commit_id.clone(), targets.clone())
            } else {
                return Ok(());
            };

        let deduped = dedupe_targets_parent_first(&targets);
        let mut sections = Vec::new();
        for target in deduped {
            let label = diff_target_label(&target);
            let diff =
                super::git_ops::get_commit_diff(repo, &commit_id, &target.path, target.is_dir)?;
            sections.push(format!("===== {label} =====\n{diff}"));
        }

        let short_id: String = commit_id.chars().take(8).collect();
        let headline = format!("Selected Commit Targets ({short_id})");
        let diff = if sections.is_empty() {
            "No selected targets for commit diff preview.".to_string()
        } else {
            sections.join("\n\n")
        };

        send_event(
            event_tx,
            EventEnvelope::new(
                Some(envelope.request_id),
                FrontendEvent::DiffLoaded {
                    request_id: envelope.request_id,
                    file_path: headline,
                    diff,
                },
            ),
        );
        Ok(())
    }
}

/// 获取分支提交列表处理器
pub struct GetBranchCommitsHandler;
impl CommandHandler for GetBranchCommitsHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let (branch_name, limit) =
            if let crate::backend::BackendCommand::GetBranchCommits { branch_name, limit } =
                &envelope.command
            {
                (branch_name.clone(), *limit)
            } else {
                return Ok(());
            };

        match super::git_ops::get_commits_for_branch(repo, &branch_name, limit) {
            Ok(commits) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::BranchCommitsLoaded {
                            request_id: envelope.request_id,
                            branch_name,
                            commits,
                        },
                    ),
                );
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "branch commits", error),
        }
        Ok(())
    }
}


/// 获取分支提交图处理器
pub struct GetBranchGraphHandler;
impl CommandHandler for GetBranchGraphHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let (branch_name, limit) =
            if let crate::backend::BackendCommand::GetBranchGraph { branch_name, limit } =
                &envelope.command
            {
                (branch_name.clone(), *limit)
            } else {
                return Ok(());
            };

        match super::git_ops::get_branch_graph(repo, &branch_name, limit) {
            Ok(graph) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::BranchGraphLoaded {
                            request_id: envelope.request_id,
                            branch_name,
                            graph,
                        },
                    ),
                );
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "branch graph", error),
        }
        Ok(())
    }
}

/// 暂存文件处理器
pub struct StageFileHandler;
impl CommandHandler for StageFileHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let file_path =
            if let crate::backend::BackendCommand::StageFile { file_path } = &envelope.command {
                file_path.clone()
            } else {
                return Ok(());
            };

        match super::git_ops::stage_file(repo, &file_path) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Staged: {file_path}"),
                        },
                    ),
                );
                // 自动刷新文件状态
                if let Ok(files) = super::git_ops::get_status_files(repo) {
                    send_event(
                        event_tx,
                        EventEnvelope::new(None, FrontendEvent::FilesUpdated { files }),
                    );
                }
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "stage", error),
        }
        Ok(())
    }
}

/// 批量暂存文件处理器
pub struct StageFilesHandler;
impl CommandHandler for StageFilesHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let file_paths =
            if let crate::backend::BackendCommand::StageFiles { file_paths } = &envelope.command {
                file_paths.clone()
            } else {
                return Ok(());
            };

        let (success, failed) = apply_paths(&file_paths, |path| {
            super::git_ops::stage_file(repo, path.as_str())
        });
        send_batch_action_result(
            event_tx,
            envelope.request_id,
            "stage",
            "Staged",
            &success,
            &failed,
        );
        refresh_files(event_tx, repo);
        Ok(())
    }
}

/// 取消暂存文件处理器
pub struct UnstageFileHandler;
impl CommandHandler for UnstageFileHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let file_path =
            if let crate::backend::BackendCommand::UnstageFile { file_path } = &envelope.command {
                file_path.clone()
            } else {
                return Ok(());
            };

        match super::git_ops::unstage_file(repo, &file_path) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Unstaged: {file_path}"),
                        },
                    ),
                );
                // 自动刷新文件状态
                if let Ok(files) = super::git_ops::get_status_files(repo) {
                    send_event(
                        event_tx,
                        EventEnvelope::new(None, FrontendEvent::FilesUpdated { files }),
                    );
                }
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "unstage", error),
        }
        Ok(())
    }
}

/// 批量取消暂存文件处理器
pub struct UnstageFilesHandler;
impl CommandHandler for UnstageFilesHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let file_paths = if let crate::backend::BackendCommand::UnstageFiles { file_paths } =
            &envelope.command
        {
            file_paths.clone()
        } else {
            return Ok(());
        };

        let (success, failed) = apply_paths(&file_paths, |path| {
            super::git_ops::unstage_file(repo, path.as_str())
        });
        send_batch_action_result(
            event_tx,
            envelope.request_id,
            "unstage",
            "Unstaged",
            &success,
            &failed,
        );
        refresh_files(event_tx, repo);
        Ok(())
    }
}

fn apply_paths<F>(paths: &[String], mut f: F) -> (Vec<String>, Vec<String>)
where
    F: FnMut(&String) -> Result<()>,
{
    let mut success = Vec::new();
    let mut failed = Vec::new();

    for path in paths {
        match f(path) {
            Ok(()) => success.push(path.clone()),
            Err(err) => failed.push(format!("{path} ({err})")),
        }
    }

    (success, failed)
}

fn send_batch_action_result(
    event_tx: &Sender<EventEnvelope>,
    request_id: u64,
    context: &str,
    verb: &str,
    success: &[String],
    failed: &[String],
) {
    if !success.is_empty() {
        send_event(
            event_tx,
            EventEnvelope::new(
                Some(request_id),
                FrontendEvent::ActionSucceeded {
                    request_id,
                    message: format!("{verb} {} files", success.len()),
                },
            ),
        );
    }

    if !failed.is_empty() {
        let details = failed
            .iter()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        let suffix = if failed.len() > 4 { ", ..." } else { "" };
        send_error(
            event_tx,
            Some(request_id),
            context,
            format!("{} files failed: {details}{suffix}", failed.len()),
        );
    }
}

fn refresh_files(event_tx: &Sender<EventEnvelope>, repo: &GitRepo) {
    if let Ok(files) = super::git_ops::get_status_files(repo) {
        send_event(
            event_tx,
            EventEnvelope::new(None, FrontendEvent::FilesUpdated { files }),
        );
    }
}

fn refresh_all(event_tx: &Sender<EventEnvelope>, repo: &GitRepo) {
    // Refresh files
    refresh_files(event_tx, repo);

    // Refresh branches
    if let Ok(branches) = super::git_ops::get_branches(repo) {
        send_event(
            event_tx,
            EventEnvelope::new(None, FrontendEvent::BranchesUpdated { branches }),
        );
    }

    // Refresh commits
    if let Ok(commits) = super::git_ops::get_commits(repo, 100) {
        send_event(
            event_tx,
            EventEnvelope::new(None, FrontendEvent::CommitsUpdated { commits }),
        );
    }
}

fn send_error(
    event_tx: &Sender<EventEnvelope>,
    request_id: Option<u64>,
    context: &str,
    error: impl std::fmt::Display,
) {
    send_event(
        event_tx,
        EventEnvelope::new(
            request_id,
            FrontendEvent::Error {
                request_id,
                message: format!("Failed to load {context}: {error}"),
            },
        ),
    );
}

/// Stage all files handler
pub struct StageAllHandler;
impl CommandHandler for StageAllHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::stage_all(repo) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: "Staged all files".to_string(),
                        },
                    ),
                );
                refresh_files(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "stage all", error),
        }
        Ok(())
    }
}

/// Discard files handler
pub struct DiscardFilesHandler;
impl CommandHandler for DiscardFilesHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let paths = if let crate::backend::BackendCommand::DiscardFiles { paths } = &envelope.command {
            paths.clone()
        } else {
            return Ok(());
        };

        match super::git_ops::discard_files(repo, &paths) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Discarded {} files", paths.len()),
                        },
                    ),
                );
                refresh_files(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "discard", error),
        }
        Ok(())
    }
}

/// Stash files handler
pub struct StashFilesHandler;
impl CommandHandler for StashFilesHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let (paths, message) = if let crate::backend::BackendCommand::StashFiles { paths, message } = &envelope.command {
            (paths.clone(), message.as_deref())
        } else {
            return Ok(());
        };

        match super::git_ops::stash_files(repo, &paths, message) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Stashed {} files", paths.len()),
                        },
                    ),
                );
                refresh_files(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "stash", error),
        }
        Ok(())
    }
}

/// Amend commit handler
pub struct AmendCommitHandler;
impl CommandHandler for AmendCommitHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let message = if let crate::backend::BackendCommand::AmendCommit { message } = &envelope.command {
            message.clone()
        } else {
            return Ok(());
        };

        match super::git_ops::amend_commit(repo, &message) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: "Amended commit".to_string(),
                        },
                    ),
                );
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "amend", error),
        }
        Ok(())
    }
}

/// Get commit message handler
pub struct GetCommitMessageHandler;
impl CommandHandler for GetCommitMessageHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let commit_id = if let crate::backend::BackendCommand::GetCommitMessage { commit_id } = &envelope.command {
            commit_id.clone()
        } else {
            return Ok(());
        };

        match super::git_ops::get_commit_message(repo, &commit_id) {
            Ok(message) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::CommitMessageLoaded {
                            request_id: envelope.request_id,
                            message,
                        },
                    ),
                );
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "get commit message", error),
        }
        Ok(())
    }
}

/// Amend commit with files handler
pub struct AmendCommitWithFilesHandler;
impl CommandHandler for AmendCommitWithFilesHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let (commit_id, message, paths) = if let crate::backend::BackendCommand::AmendCommitWithFiles { commit_id, message, paths } = &envelope.command {
            (commit_id.clone(), message.clone(), paths.clone())
        } else {
            return Ok(());
        };

        match super::git_ops::amend_commit_with_files(repo, &commit_id, &message, &paths) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: "Amended commit with selected files".to_string(),
                        },
                    ),
                );
                refresh_all(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "amend with files", error),
        }
        Ok(())
    }
}

/// Reset hard handler
pub struct ResetHardHandler;
impl CommandHandler for ResetHardHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let target = if let crate::backend::BackendCommand::ResetHard { target } = &envelope.command {
            target.clone()
        } else {
            return Ok(());
        };

        match super::git_ops::reset_hard(repo, &target) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Hard reset to {}", target),
                        },
                    ),
                );
                refresh_all(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "reset hard", error),
        }
        Ok(())
    }
}

/// Reset mixed handler
pub struct ResetMixedHandler;
impl CommandHandler for ResetMixedHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let target = if let crate::backend::BackendCommand::ResetMixed { target } = &envelope.command {
            target.clone()
        } else {
            return Ok(());
        };

        match super::git_ops::reset_mixed(repo, &target) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Mixed reset to {}", target),
                        },
                    ),
                );
                refresh_all(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "reset mixed", error),
        }
        Ok(())
    }
}

/// Reset soft handler
pub struct ResetSoftHandler;
impl CommandHandler for ResetSoftHandler {
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &Sender<EventEnvelope>,
    ) -> Result<()> {
        let target = if let crate::backend::BackendCommand::ResetSoft { target } = &envelope.command {
            target.clone()
        } else {
            return Ok(());
        };

        match super::git_ops::reset_soft(repo, &target) {
            Ok(()) => {
                send_event(
                    event_tx,
                    EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::ActionSucceeded {
                            request_id: envelope.request_id,
                            message: format!("Soft reset to {}", target),
                        },
                    ),
                );
                refresh_all(event_tx, repo);
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "reset soft", error),
        }
        Ok(())
    }
}
