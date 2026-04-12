use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use crate::git::{BranchEntry, CommitEntry, GitRepo, StashEntry, StatusEntry};

#[derive(Debug)]
pub enum BackendCommand {
    RefreshStatus,
    RefreshBranches,
    RefreshCommits { limit: usize },
    RefreshStashes,
    GetDiff { file_path: String },
    Quit,
}

#[derive(Debug)]
pub enum FrontendEvent {
    StatusUpdated { files: Vec<StatusEntry> },
    BranchesUpdated { branches: Vec<BranchEntry> },
    CommitsUpdated { commits: Vec<CommitEntry> },
    StashesUpdated { stashes: Vec<StashEntry> },
    DiffLoaded { file_path: String, diff: String },
    Error(String),
}

pub async fn run_backend(
    mut cmd_rx: UnboundedReceiver<BackendCommand>,
    event_tx: UnboundedSender<FrontendEvent>,
) {
    let mut repo = match GitRepo::discover() {
        Ok(repo) => Some(repo),
        Err(error) => {
            let _ = event_tx.send(FrontendEvent::Error(format!(
                "Failed to open repository: {error}"
            )));
            None
        }
    };

    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            BackendCommand::RefreshStatus => {
                if let Some(repo) = repo.as_ref() {
                    match repo.get_status_files() {
                        Ok(files) => {
                            let _ = event_tx.send(FrontendEvent::StatusUpdated { files });
                        }
                        Err(error) => send_error(&event_tx, "status", error),
                    }
                }
            }
            BackendCommand::RefreshBranches => {
                if let Some(repo) = repo.as_ref() {
                    match repo.get_branches() {
                        Ok(branches) => {
                            let _ = event_tx.send(FrontendEvent::BranchesUpdated { branches });
                        }
                        Err(error) => send_error(&event_tx, "branches", error),
                    }
                }
            }
            BackendCommand::RefreshCommits { limit } => {
                if let Some(repo) = repo.as_ref() {
                    match repo.get_commits(limit) {
                        Ok(commits) => {
                            let _ = event_tx.send(FrontendEvent::CommitsUpdated { commits });
                        }
                        Err(error) => send_error(&event_tx, "commits", error),
                    }
                }
            }
            BackendCommand::RefreshStashes => {
                if let Some(repo) = repo.as_mut() {
                    match repo.get_stashes() {
                        Ok(stashes) => {
                            let _ = event_tx.send(FrontendEvent::StashesUpdated { stashes });
                        }
                        Err(error) => send_error(&event_tx, "stashes", error),
                    }
                }
            }
            BackendCommand::GetDiff { file_path } => {
                if let Some(repo) = repo.as_ref() {
                    match repo.get_diff(&file_path) {
                        Ok(diff) => {
                            let _ = event_tx.send(FrontendEvent::DiffLoaded { file_path, diff });
                        }
                        Err(error) => send_error(&event_tx, "diff", error),
                    }
                }
            }
            BackendCommand::Quit => break,
        }
    }
}

fn send_error(
    event_tx: &UnboundedSender<FrontendEvent>,
    context: &str,
    error: impl std::fmt::Display,
) {
    let _ = event_tx.send(FrontendEvent::Error(format!(
        "Failed to load {context}: {error}"
    )));
}
