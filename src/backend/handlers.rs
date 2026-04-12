use anyhow::Result;

use super::git_ops::GitRepo;
use super::{CommandEnvelope, EventEnvelope, FrontendEvent};
use tokio::sync::mpsc::UnboundedSender;

/// 命令处理器 trait
pub trait CommandHandler: Send + Sync {
    /// 执行命令并发送事件
    fn handle(
        &self,
        envelope: &CommandEnvelope,
        repo: &GitRepo,
        event_tx: &UnboundedSender<EventEnvelope>,
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
        _event_tx: &UnboundedSender<EventEnvelope>,
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::get_status_files(repo) {
            Ok(files) => {
                let _ = event_tx.send(EventEnvelope::new(
                    None,
                    FrontendEvent::FilesUpdated { files },
                ));
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::get_branches(repo) {
            Ok(branches) => {
                let _ = event_tx.send(EventEnvelope::new(
                    None,
                    FrontendEvent::BranchesUpdated { branches },
                ));
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        let limit =
            if let crate::backend::BackendCommand::RefreshCommits { limit } = &envelope.command {
                *limit
            } else {
                30
            };

        match super::git_ops::get_commits(repo, limit) {
            Ok(commits) => {
                let _ = event_tx.send(EventEnvelope::new(
                    None,
                    FrontendEvent::CommitsUpdated { commits },
                ));
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
        _event_tx: &UnboundedSender<EventEnvelope>,
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        match super::git_ops::get_stashes(repo) {
            Ok(stashes) => {
                let _ = event_tx.send(EventEnvelope::new(
                    None,
                    FrontendEvent::StashesUpdated { stashes },
                ));
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        let file_path =
            if let crate::backend::BackendCommand::GetDiff { file_path } = &envelope.command {
                file_path.clone()
            } else {
                return Ok(());
            };

        match super::git_ops::get_diff(repo, &file_path) {
            Ok(diff) => {
                let _ = event_tx.send(EventEnvelope::new(
                    Some(envelope.request_id),
                    FrontendEvent::DiffLoaded {
                        request_id: envelope.request_id,
                        file_path,
                        diff,
                    },
                ));
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "diff", error),
        }
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
        event_tx: &UnboundedSender<EventEnvelope>,
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
                    let _ = event_tx.send(EventEnvelope::new(
                        Some(envelope.request_id),
                        FrontendEvent::CommitFilesLoaded {
                            request_id: envelope.request_id,
                            commit_id,
                            files,
                        },
                    ));
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
        event_tx: &UnboundedSender<EventEnvelope>,
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

                let _ = event_tx.send(EventEnvelope::new(
                    Some(envelope.request_id),
                    FrontendEvent::DiffLoaded {
                        request_id: envelope.request_id,
                        file_path: target,
                        diff,
                    },
                ));
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "commit diff", error),
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
        event_tx: &UnboundedSender<EventEnvelope>,
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
                let _ = event_tx.send(EventEnvelope::new(
                    Some(envelope.request_id),
                    FrontendEvent::BranchGraphLoaded {
                        request_id: envelope.request_id,
                        branch_name,
                        graph,
                    },
                ));
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        let file_path =
            if let crate::backend::BackendCommand::StageFile { file_path } = &envelope.command {
                file_path.clone()
            } else {
                return Ok(());
            };

        match super::git_ops::stage_file(repo, &file_path) {
            Ok(()) => {
                let _ = event_tx.send(EventEnvelope::new(
                    Some(envelope.request_id),
                    FrontendEvent::ActionSucceeded {
                        request_id: envelope.request_id,
                        message: format!("Staged: {file_path}"),
                    },
                ));
                // 自动刷新文件状态
                if let Ok(files) = super::git_ops::get_status_files(repo) {
                    let _ = event_tx.send(EventEnvelope::new(
                        None,
                        FrontendEvent::FilesUpdated { files },
                    ));
                }
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "stage", error),
        }
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
        event_tx: &UnboundedSender<EventEnvelope>,
    ) -> Result<()> {
        let file_path =
            if let crate::backend::BackendCommand::UnstageFile { file_path } = &envelope.command {
                file_path.clone()
            } else {
                return Ok(());
            };

        match super::git_ops::unstage_file(repo, &file_path) {
            Ok(()) => {
                let _ = event_tx.send(EventEnvelope::new(
                    Some(envelope.request_id),
                    FrontendEvent::ActionSucceeded {
                        request_id: envelope.request_id,
                        message: format!("Unstaged: {file_path}"),
                    },
                ));
                // 自动刷新文件状态
                if let Ok(files) = super::git_ops::get_status_files(repo) {
                    let _ = event_tx.send(EventEnvelope::new(
                        None,
                        FrontendEvent::FilesUpdated { files },
                    ));
                }
            }
            Err(error) => send_error(event_tx, Some(envelope.request_id), "unstage", error),
        }
        Ok(())
    }
}

fn send_error(
    event_tx: &UnboundedSender<EventEnvelope>,
    request_id: Option<u64>,
    context: &str,
    error: impl std::fmt::Display,
) {
    let _ = event_tx.send(EventEnvelope::new(
        request_id,
        FrontendEvent::Error {
            request_id,
            message: format!("Failed to load {context}: {error}"),
        },
    ));
}
