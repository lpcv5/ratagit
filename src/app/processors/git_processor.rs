// src/app/processors/git_processor.rs
//
// GitProcessor converts GitEvent to BackendCommand(s).
//
// This processor is responsible for:
// - Converting high-level GitEvent to low-level BackendCommand(s)
// - Handling multi-select logic (stage/unstage multiple files)
// - Determining stage/unstage direction based on anchor file
// - Filtering out invalid targets (e.g., directories)
//
// GitProcessor is stateless - it reads from AppState but doesn't mutate it.
// All state changes happen via BackendCommand → Backend → FrontendEvent → AppState.

use crate::app::events::{BranchDeleteMode, GitEvent};
use crate::app::state::AppState;
use crate::backend::BackendCommand;

pub struct GitProcessor;

impl GitProcessor {
    /// Process a GitEvent and return zero or more BackendCommands
    ///
    /// This is the main entry point for converting events to commands.
    /// Returns a Vec because some events may generate multiple commands
    /// (e.g., multi-select operations).
    pub fn process(&self, event: GitEvent, state: &AppState) -> Vec<BackendCommand> {
        match event {
            GitEvent::ToggleStageFile => self.toggle_stage_file(state),
            GitEvent::StageAll => vec![BackendCommand::StageAll],
            GitEvent::CommitWithMessage(message) => vec![BackendCommand::Commit { message }],
            GitEvent::DiscardSelected => self.discard_selected(state),
            GitEvent::StashSelected => self.stash_selected(state),
            GitEvent::AmendCommit => self.amend_commit(state),
            GitEvent::ExecuteReset(index) => self.execute_reset(index),
            GitEvent::IgnoreSelected => self.ignore_selected(state),
            GitEvent::CheckoutBranch { branch_name, force } => {
                self.checkout_branch(branch_name, force)
            }
            GitEvent::CheckoutCommit { commit_id } => self.checkout_commit(commit_id),
            GitEvent::CherryPickCommits { commit_ids } => self.cherry_pick_commits(commit_ids),
            GitEvent::CreateBranch {
                new_name,
                from_branch,
            } => self.create_branch(new_name, from_branch),
            GitEvent::DeleteBranch {
                local_branch,
                remote,
                mode,
            } => self.delete_branch(local_branch, remote, mode),
            GitEvent::LoadBranchCommits { branch_name, limit } => {
                self.load_branch_commits(branch_name, limit)
            }
            GitEvent::RevertCommit { commit_id } => {
                vec![BackendCommand::RevertCommit { commit_id }]
            }
        }
    }

    fn toggle_stage_file(&self, state: &AppState) -> Vec<BackendCommand> {
        // Get selected targets (handles both single and multi-select)
        let selected_targets = state.components.selected_file_tree_targets();

        // Filter out directories, only keep files
        let selected_files: Vec<String> = selected_targets
            .into_iter()
            .filter(|(_, is_dir)| !is_dir)
            .map(|(path, _)| path)
            .collect();

        if selected_files.is_empty() {
            return vec![];
        }

        // Determine the anchor file (the file that determines stage/unstage direction)
        let anchor_file = state
            .components
            .selected_file_anchor_target()
            .and_then(|(path, is_dir)| (!is_dir).then_some(path))
            .or_else(|| selected_files.first().cloned());

        let Some(pivot_path) = anchor_file else {
            return vec![];
        };

        // Find the anchor file in the data cache to check its status
        let Some(file) = state.data_cache.files.iter().find(|e| e.path == pivot_path) else {
            return vec![];
        };

        let should_unstage = file.is_staged;

        // Generate the appropriate command
        let command = if selected_files.len() == 1 {
            let file_path = selected_files.into_iter().next().unwrap();
            if should_unstage {
                BackendCommand::UnstageFile { file_path }
            } else {
                BackendCommand::StageFile { file_path }
            }
        } else if should_unstage {
            BackendCommand::UnstageFiles {
                file_paths: selected_files,
            }
        } else {
            BackendCommand::StageFiles {
                file_paths: selected_files,
            }
        };

        vec![command]
    }

    fn discard_selected(&self, state: &AppState) -> Vec<BackendCommand> {
        let paths = if state.components.is_file_multi_select_active() {
            state
                .components
                .file_list_panel
                .selected_tree_targets()
                .into_iter()
                .map(|(path, _)| path)
                .collect()
        } else if let Some((path, _)) = state.components.file_list_panel.selected_tree_node() {
            vec![path]
        } else {
            return vec![];
        };

        if paths.is_empty() {
            return vec![];
        }

        vec![BackendCommand::DiscardFiles { paths }]
    }

    fn stash_selected(&self, state: &AppState) -> Vec<BackendCommand> {
        let paths = if state.components.is_file_multi_select_active() {
            state
                .components
                .file_list_panel
                .selected_tree_targets()
                .into_iter()
                .map(|(path, _)| path)
                .collect()
        } else if let Some((path, _)) = state.components.file_list_panel.selected_tree_node() {
            vec![path]
        } else {
            return vec![];
        };

        if paths.is_empty() {
            return vec![];
        }

        vec![BackendCommand::StashFiles {
            paths,
            message: None,
        }]
    }

    fn amend_commit(&self, state: &AppState) -> Vec<BackendCommand> {
        // Get selected files from Files panel
        let paths = state
            .components
            .file_list_panel
            .selected_tree_targets()
            .into_iter()
            .map(|(path, _)| path)
            .collect::<Vec<_>>();

        if paths.is_empty() {
            return vec![];
        }

        // Get selected commit from Commits panel
        let selected_commit = state
            .components
            .commit_panel
            .selected_commit(&state.data_cache.commits);

        let Some(commit) = selected_commit else {
            return vec![];
        };

        // Reconstruct the full message from summary and body
        let message = match &commit.body {
            Some(body) if !body.is_empty() => format!("{}\n\n{}", commit.summary, body),
            _ => commit.summary.clone(),
        };

        vec![BackendCommand::AmendCommitWithFiles {
            commit_id: commit.id.clone(),
            message,
            paths,
        }]
    }

    fn execute_reset(&self, index: usize) -> Vec<BackendCommand> {
        match index {
            0 => vec![BackendCommand::ResetHard {
                target: "HEAD".to_string(),
            }],
            1 => vec![BackendCommand::ResetMixed {
                target: "HEAD".to_string(),
            }],
            2 => vec![BackendCommand::ResetSoft {
                target: "HEAD".to_string(),
            }],
            3 => vec![BackendCommand::ResetHard {
                target: "HEAD~1".to_string(),
            }],
            4 => vec![BackendCommand::ResetSoft {
                target: "HEAD~1".to_string(),
            }],
            5 => {
                // Nuke repo - not implemented yet, return empty
                vec![]
            }
            _ => vec![],
        }
    }

    fn ignore_selected(&self, state: &AppState) -> Vec<BackendCommand> {
        let targets = state.components.file_list_panel.selected_tree_targets();
        if targets.is_empty() {
            return vec![];
        }

        let paths: Vec<String> = targets.into_iter().map(|(path, _)| path).collect();
        vec![BackendCommand::IgnoreFiles { paths }]
    }

    fn checkout_branch(&self, branch_name: String, force: bool) -> Vec<BackendCommand> {
        vec![BackendCommand::CheckoutBranch { branch_name, force }]
    }

    fn checkout_commit(&self, commit_id: String) -> Vec<BackendCommand> {
        vec![BackendCommand::CheckoutCommit { commit_id }]
    }

    fn cherry_pick_commits(&self, commit_ids: Vec<String>) -> Vec<BackendCommand> {
        if commit_ids.is_empty() {
            return vec![];
        }
        vec![BackendCommand::CherryPickCommits { commit_ids }]
    }

    fn create_branch(&self, new_name: String, from_branch: String) -> Vec<BackendCommand> {
        vec![BackendCommand::CreateBranch {
            new_name,
            from_branch,
        }]
    }

    fn delete_branch(
        &self,
        local_branch: String,
        remote: Option<crate::app::events::BranchRemoteRef>,
        mode: BranchDeleteMode,
    ) -> Vec<BackendCommand> {
        match mode {
            BranchDeleteMode::Local => vec![BackendCommand::DeleteLocalBranch {
                branch_name: local_branch,
            }],
            BranchDeleteMode::Remote => {
                let Some(remote_ref) = remote else {
                    return vec![];
                };
                vec![BackendCommand::DeleteRemoteBranch {
                    remote_name: remote_ref.remote_name,
                    branch_name: remote_ref.branch_name,
                }]
            }
            BranchDeleteMode::LocalAndRemote => {
                let Some(remote_ref) = remote else {
                    return vec![];
                };
                vec![
                    BackendCommand::DeleteRemoteBranch {
                        remote_name: remote_ref.remote_name,
                        branch_name: remote_ref.branch_name,
                    },
                    BackendCommand::DeleteLocalBranch {
                        branch_name: local_branch,
                    },
                ]
            }
        }
    }

    fn load_branch_commits(&self, branch_name: String, limit: usize) -> Vec<BackendCommand> {
        vec![BackendCommand::GetBranchCommits { branch_name, limit }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::events::{BranchDeleteMode, BranchRemoteRef};
    use crate::backend::git_ops::StatusEntry;

    fn mock_state() -> AppState {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(100);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(100);
        AppState::new(cmd_tx, event_rx)
    }

    fn mock_state_with_files(files: Vec<StatusEntry>) -> AppState {
        let mut state = mock_state();
        state.data_cache.files = files;
        state.sync_file_list_state();
        state
    }

    #[test]
    fn test_stage_all() {
        let processor = GitProcessor;
        let state = mock_state();
        let commands = processor.process(GitEvent::StageAll, &state);

        assert_eq!(commands.len(), 1);
        assert!(matches!(commands[0], BackendCommand::StageAll));
    }

    #[test]
    fn test_commit_with_message() {
        let processor = GitProcessor;
        let state = mock_state();
        let commands = processor.process(
            GitEvent::CommitWithMessage("test commit".to_string()),
            &state,
        );

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::Commit { message } => {
                assert_eq!(message, "test commit");
            }
            _ => panic!("Expected Commit command"),
        }
    }

    #[test]
    fn test_toggle_stage_single_unstaged_file() {
        let processor = GitProcessor;
        let files = vec![StatusEntry {
            path: "file.txt".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        }];
        let mut state = mock_state_with_files(files);
        state.components.file_list_panel.state_mut().select(Some(0));

        let commands = processor.process(GitEvent::ToggleStageFile, &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::StageFile { file_path } => {
                assert_eq!(file_path, "file.txt");
            }
            _ => panic!("Expected StageFile command"),
        }
    }

    #[test]
    fn test_toggle_stage_single_staged_file() {
        let processor = GitProcessor;
        let files = vec![StatusEntry {
            path: "file.txt".to_string(),
            is_staged: true,
            is_unstaged: false,
            is_untracked: false,
        }];
        let mut state = mock_state_with_files(files);
        state.components.file_list_panel.state_mut().select(Some(0));

        let commands = processor.process(GitEvent::ToggleStageFile, &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::UnstageFile { file_path } => {
                assert_eq!(file_path, "file.txt");
            }
            _ => panic!("Expected UnstageFile command"),
        }
    }

    #[test]
    fn test_toggle_stage_no_selection() {
        let processor = GitProcessor;
        let files = vec![StatusEntry {
            path: "file.txt".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        }];
        let mut state = mock_state_with_files(files);
        // Explicitly clear selection
        state.components.file_list_panel.state_mut().select(None);

        let commands = processor.process(GitEvent::ToggleStageFile, &state);

        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_discard_selected_single_file() {
        let processor = GitProcessor;
        let files = vec![StatusEntry {
            path: "file.txt".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        }];
        let mut state = mock_state_with_files(files);
        state.components.file_list_panel.state_mut().select(Some(0));

        let commands = processor.process(GitEvent::DiscardSelected, &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::DiscardFiles { paths } => {
                assert_eq!(paths.len(), 1);
                assert_eq!(paths[0], "file.txt");
            }
            _ => panic!("Expected DiscardFiles command"),
        }
    }

    #[test]
    fn test_discard_selected_no_selection() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::DiscardSelected, &state);

        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_stash_selected_single_file() {
        let processor = GitProcessor;
        let files = vec![StatusEntry {
            path: "file.txt".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        }];
        let mut state = mock_state_with_files(files);
        state.components.file_list_panel.state_mut().select(Some(0));

        let commands = processor.process(GitEvent::StashSelected, &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::StashFiles { paths, message } => {
                assert_eq!(paths.len(), 1);
                assert_eq!(paths[0], "file.txt");
                assert_eq!(message, &None);
            }
            _ => panic!("Expected StashFiles command"),
        }
    }

    #[test]
    fn test_ignore_selected_single_file() {
        let processor = GitProcessor;
        let files = vec![StatusEntry {
            path: "file.txt".to_string(),
            is_staged: false,
            is_unstaged: true,
            is_untracked: false,
        }];
        let mut state = mock_state_with_files(files);
        state.components.file_list_panel.state_mut().select(Some(0));

        let commands = processor.process(GitEvent::IgnoreSelected, &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::IgnoreFiles { paths } => {
                assert_eq!(paths.len(), 1);
                assert_eq!(paths[0], "file.txt");
            }
            _ => panic!("Expected IgnoreFiles command"),
        }
    }

    #[test]
    fn test_execute_reset_hard() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::ExecuteReset(0), &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::ResetHard { target } => {
                assert_eq!(target, "HEAD");
            }
            _ => panic!("Expected ResetHard command"),
        }
    }

    #[test]
    fn test_execute_reset_mixed() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::ExecuteReset(1), &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::ResetMixed { target } => {
                assert_eq!(target, "HEAD");
            }
            _ => panic!("Expected ResetMixed command"),
        }
    }

    #[test]
    fn test_execute_reset_soft() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::ExecuteReset(2), &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::ResetSoft { target } => {
                assert_eq!(target, "HEAD");
            }
            _ => panic!("Expected ResetSoft command"),
        }
    }

    #[test]
    fn test_execute_reset_hard_head_tilde_1() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::ExecuteReset(3), &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::ResetHard { target } => {
                assert_eq!(target, "HEAD~1");
            }
            _ => panic!("Expected ResetHard command"),
        }
    }

    #[test]
    fn test_execute_reset_soft_head_tilde_1() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::ExecuteReset(4), &state);

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::ResetSoft { target } => {
                assert_eq!(target, "HEAD~1");
            }
            _ => panic!("Expected ResetSoft command"),
        }
    }

    #[test]
    fn test_execute_reset_invalid_index() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::ExecuteReset(99), &state);

        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_amend_commit_returns_empty() {
        // AmendCommit is complex and requires commit selection
        // For now, return empty vec (can be implemented incrementally)
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(GitEvent::AmendCommit, &state);

        assert_eq!(commands.len(), 0);
    }

    #[test]
    fn test_checkout_branch_event_maps_to_backend_command() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(
            GitEvent::CheckoutBranch {
                branch_name: "feature".to_string(),
                force: false,
            },
            &state,
        );

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            BackendCommand::CheckoutBranch { branch_name, force }
                if branch_name == "feature" && !force
        ));
    }

    #[test]
    fn test_load_branch_commits_event_maps_to_backend_command() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(
            GitEvent::LoadBranchCommits {
                branch_name: "main".to_string(),
                limit: 100,
            },
            &state,
        );

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            BackendCommand::GetBranchCommits { branch_name, limit }
                if branch_name == "main" && *limit == 100
        ));
    }

    #[test]
    fn test_checkout_commit_event_maps_to_backend_command() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(
            GitEvent::CheckoutCommit {
                commit_id: "abc123".to_string(),
            },
            &state,
        );

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            BackendCommand::CheckoutCommit { commit_id } if commit_id == "abc123"
        ));
    }

    #[test]
    fn test_cherry_pick_commits_event_maps_to_backend_command() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(
            GitEvent::CherryPickCommits {
                commit_ids: vec!["abc123".to_string(), "def456".to_string()],
            },
            &state,
        );

        assert_eq!(commands.len(), 1);
        assert!(matches!(
            &commands[0],
            BackendCommand::CherryPickCommits { commit_ids }
                if commit_ids == &vec!["abc123".to_string(), "def456".to_string()]
        ));
    }

    #[test]
    fn test_cherry_pick_commits_event_with_empty_ids_returns_no_command() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands =
            processor.process(GitEvent::CherryPickCommits { commit_ids: vec![] }, &state);

        assert!(commands.is_empty());
    }

    #[test]
    fn test_delete_branch_local_and_remote_maps_to_two_backend_commands() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(
            GitEvent::DeleteBranch {
                local_branch: "feature".to_string(),
                remote: Some(BranchRemoteRef {
                    remote_name: "origin".to_string(),
                    branch_name: "feature".to_string(),
                }),
                mode: BranchDeleteMode::LocalAndRemote,
            },
            &state,
        );

        assert_eq!(commands.len(), 2);
        assert!(matches!(
            &commands[0],
            BackendCommand::DeleteRemoteBranch {
                remote_name,
                branch_name
            } if remote_name == "origin" && branch_name == "feature"
        ));
        assert!(matches!(
            &commands[1],
            BackendCommand::DeleteLocalBranch { branch_name } if branch_name == "feature"
        ));
    }

    #[test]
    fn test_revert_commit_event() {
        let processor = GitProcessor;
        let state = mock_state();

        let commands = processor.process(
            GitEvent::RevertCommit {
                commit_id: "abc123".to_string(),
            },
            &state,
        );

        assert_eq!(commands.len(), 1);
        match &commands[0] {
            BackendCommand::RevertCommit { commit_id } => {
                assert_eq!(commit_id, "abc123");
            }
            _ => panic!("Expected RevertCommit command"),
        }
    }
}
