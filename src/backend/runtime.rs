use std::collections::HashMap;
use std::time::{Duration, Instant};

use tokio::sync::mpsc::{Receiver, Sender};

use super::git_ops::GitRepo;
use super::handlers::CommandHandler;
use super::handlers::{
    send_event, AmendCommitHandler, AmendCommitWithFilesHandler, DiscardFilesHandler, GetBranchCommitsHandler, GetBranchGraphHandler,
    GetCommitDiffBatchHandler, GetCommitDiffHandler, GetCommitFilesHandler, GetCommitMessageHandler, GetDiffBatchHandler,
    GetDiffHandler, RefreshBranchesHandler, RefreshCommitsHandler, RefreshStashesHandler,
    RefreshStatusHandler, ResetHardHandler, ResetMixedHandler, ResetSoftHandler, StageAllHandler,
    StageFileHandler, StageFilesHandler, StashFilesHandler, UnstageFileHandler, UnstageFilesHandler,
};
use super::{CommandEnvelope, EventEnvelope, FrontendEvent};

const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

/// 命令类型枚举（用于映射）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CommandKey {
    RefreshStatus,
    RefreshBranches,
    RefreshCommits,
    RefreshStashes,
    GetDiff,
    GetDiffBatch,
    GetCommitFiles,
    GetCommitDiff,
    GetCommitDiffBatch,
    GetBranchGraph,
    GetBranchCommits,
    StageFile,
    StageFiles,
    UnstageFile,
    UnstageFiles,
    StageAll,
    DiscardFiles,
    StashFiles,
    AmendCommit,
    GetCommitMessage,
    AmendCommitWithFiles,
    ResetHard,
    ResetMixed,
    ResetSoft,
}

pub async fn run_backend(mut cmd_rx: Receiver<CommandEnvelope>, event_tx: Sender<EventEnvelope>) {
    let mut repo = match GitRepo::discover() {
        Ok(repo) => Some(repo),
        Err(error) => {
            send_event(
                &event_tx,
                EventEnvelope::new(
                    None,
                    FrontendEvent::Error {
                        request_id: None,
                        message: format!("Failed to open repository: {error}"),
                    },
                ),
            );
            None
        }
    };

    // 构建命令处理器映射表
    let mut handlers: HashMap<CommandKey, Box<dyn CommandHandler>> = HashMap::new();
    handlers.insert(
        CommandKey::RefreshStatus,
        Box::new(RefreshStatusHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::RefreshBranches,
        Box::new(RefreshBranchesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::RefreshCommits,
        Box::new(RefreshCommitsHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::RefreshStashes,
        Box::new(RefreshStashesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetDiff,
        Box::new(GetDiffHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetDiffBatch,
        Box::new(GetDiffBatchHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::StageFile,
        Box::new(StageFileHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::UnstageFile,
        Box::new(UnstageFileHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::StageFiles,
        Box::new(StageFilesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::UnstageFiles,
        Box::new(UnstageFilesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetCommitFiles,
        Box::new(GetCommitFilesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetCommitDiff,
        Box::new(GetCommitDiffHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetCommitDiffBatch,
        Box::new(GetCommitDiffBatchHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetBranchGraph,
        Box::new(GetBranchGraphHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetBranchCommits,
        Box::new(GetBranchCommitsHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::StageAll,
        Box::new(StageAllHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::DiscardFiles,
        Box::new(DiscardFilesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::StashFiles,
        Box::new(StashFilesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::AmendCommit,
        Box::new(AmendCommitHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::GetCommitMessage,
        Box::new(GetCommitMessageHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::AmendCommitWithFiles,
        Box::new(AmendCommitWithFilesHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::ResetHard,
        Box::new(ResetHardHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::ResetMixed,
        Box::new(ResetMixedHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::ResetSoft,
        Box::new(ResetSoftHandler) as Box<dyn CommandHandler>,
    );

    while let Some(envelope) = cmd_rx.recv().await {
        let command_key = match &envelope.command {
            crate::backend::BackendCommand::RefreshStatus => Some(CommandKey::RefreshStatus),
            crate::backend::BackendCommand::RefreshBranches => Some(CommandKey::RefreshBranches),
            crate::backend::BackendCommand::RefreshCommits { .. } => {
                Some(CommandKey::RefreshCommits)
            }
            crate::backend::BackendCommand::RefreshStashes => Some(CommandKey::RefreshStashes),
            crate::backend::BackendCommand::GetDiff { .. } => Some(CommandKey::GetDiff),
            crate::backend::BackendCommand::GetDiffBatch { .. } => Some(CommandKey::GetDiffBatch),
            crate::backend::BackendCommand::GetCommitFiles { .. } => {
                Some(CommandKey::GetCommitFiles)
            }
            crate::backend::BackendCommand::GetCommitDiff { .. } => Some(CommandKey::GetCommitDiff),
            crate::backend::BackendCommand::GetCommitDiffBatch { .. } => {
                Some(CommandKey::GetCommitDiffBatch)
            }
            crate::backend::BackendCommand::GetBranchGraph { .. } => {
                Some(CommandKey::GetBranchGraph)
            }
            crate::backend::BackendCommand::GetBranchCommits { .. } => {
                Some(CommandKey::GetBranchCommits)
            }
            crate::backend::BackendCommand::StageFile { .. } => Some(CommandKey::StageFile),
            crate::backend::BackendCommand::StageFiles { .. } => Some(CommandKey::StageFiles),
            crate::backend::BackendCommand::UnstageFile { .. } => Some(CommandKey::UnstageFile),
            crate::backend::BackendCommand::UnstageFiles { .. } => Some(CommandKey::UnstageFiles),
            crate::backend::BackendCommand::StageAll => Some(CommandKey::StageAll),
            crate::backend::BackendCommand::DiscardFiles { .. } => Some(CommandKey::DiscardFiles),
            crate::backend::BackendCommand::StashFiles { .. } => Some(CommandKey::StashFiles),
            crate::backend::BackendCommand::AmendCommit { .. } => Some(CommandKey::AmendCommit),
            crate::backend::BackendCommand::GetCommitMessage { .. } => Some(CommandKey::GetCommitMessage),
            crate::backend::BackendCommand::AmendCommitWithFiles { .. } => Some(CommandKey::AmendCommitWithFiles),
            crate::backend::BackendCommand::ResetHard { .. } => Some(CommandKey::ResetHard),
            crate::backend::BackendCommand::ResetMixed { .. } => Some(CommandKey::ResetMixed),
            crate::backend::BackendCommand::ResetSoft { .. } => Some(CommandKey::ResetSoft),
            crate::backend::BackendCommand::Quit => None,
        };

        if let Some(key) = command_key {
            if let Some(ref mut repo_mut) = repo {
                if let Some(handler) = handlers.get(&key) {
                    let start = Instant::now();
                    tokio::task::block_in_place(|| {
                        if handler.needs_mut_repo() {
                            handler.handle_mut(&envelope, repo_mut, &event_tx)
                        } else {
                            handler.handle(&envelope, repo_mut, &event_tx)
                        }
                    })
                    .ok();
                    if start.elapsed() > OPERATION_TIMEOUT {
                        send_event(
                            &event_tx,
                            EventEnvelope::new(
                                Some(envelope.request_id),
                                FrontendEvent::Error {
                                    request_id: Some(envelope.request_id),
                                    message: format!(
                                        "Operation took {:.1}s (limit {}s)",
                                        start.elapsed().as_secs_f32(),
                                        OPERATION_TIMEOUT.as_secs()
                                    ),
                                },
                            ),
                        );
                    }
                }
            }
        } else {
            break;
        }
    }
}
