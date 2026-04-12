use std::collections::HashMap;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::git_ops::GitRepo;
use super::handlers::CommandHandler;
use super::handlers::{
    GetBranchGraphHandler, GetCommitDiffHandler, GetCommitFilesHandler, GetDiffHandler,
    RefreshBranchesHandler, RefreshCommitsHandler, RefreshStashesHandler, RefreshStatusHandler,
    StageFileHandler, UnstageFileHandler,
};
use super::{CommandEnvelope, EventEnvelope, FrontendEvent};

/// 命令类型枚举（用于映射）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum CommandKey {
    RefreshStatus,
    RefreshBranches,
    RefreshCommits,
    RefreshStashes,
    GetDiff,
    GetCommitFiles,
    GetCommitDiff,
    GetBranchGraph,
    StageFile,
    UnstageFile,
}

pub async fn run_backend(
    mut cmd_rx: UnboundedReceiver<CommandEnvelope>,
    event_tx: UnboundedSender<EventEnvelope>,
) {
    let mut repo = match GitRepo::discover() {
        Ok(repo) => Some(repo),
        Err(error) => {
            let _ = event_tx.send(EventEnvelope::new(
                None,
                FrontendEvent::Error {
                    request_id: None,
                    message: format!("Failed to open repository: {error}"),
                },
            ));
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
        CommandKey::StageFile,
        Box::new(StageFileHandler) as Box<dyn CommandHandler>,
    );
    handlers.insert(
        CommandKey::UnstageFile,
        Box::new(UnstageFileHandler) as Box<dyn CommandHandler>,
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
        CommandKey::GetBranchGraph,
        Box::new(GetBranchGraphHandler) as Box<dyn CommandHandler>,
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
            crate::backend::BackendCommand::GetCommitFiles { .. } => {
                Some(CommandKey::GetCommitFiles)
            }
            crate::backend::BackendCommand::GetCommitDiff { .. } => Some(CommandKey::GetCommitDiff),
            crate::backend::BackendCommand::GetBranchGraph { .. } => {
                Some(CommandKey::GetBranchGraph)
            }
            crate::backend::BackendCommand::StageFile { .. } => Some(CommandKey::StageFile),
            crate::backend::BackendCommand::UnstageFile { .. } => Some(CommandKey::UnstageFile),
            crate::backend::BackendCommand::Quit => None,
        };

        if let Some(key) = command_key {
            if let Some(ref mut repo_mut) = repo {
                if let Some(handler) = handlers.get(&key) {
                    if handler.needs_mut_repo() {
                        let _ = handler.handle_mut(&envelope, repo_mut, &event_tx);
                    } else {
                        let _ = handler.handle(&envelope, repo_mut, &event_tx);
                    }
                }
            }
        } else {
            // Quit 命令
            break;
        }
    }
}
