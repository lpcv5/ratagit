use ratatui::widgets::ListState;

use crate::backend::git_ops::{BranchEntry, StatusEntry};
use crate::backend::{CommandEnvelope, EventEnvelope};
use crate::components::dialogs::ModalDialogV2;
use tokio::sync::mpsc::{Receiver, Sender};

use super::components::AppComponents;
use super::{CachedData, UiState};

/// AppState：封装所有应用状态
pub struct AppState {
    pub ui_state: UiState,
    pub data_cache: CachedData,
    pub cmd_tx: Sender<CommandEnvelope>,
    pub event_rx: Receiver<EventEnvelope>,
    pub should_quit: bool,
    pub components: AppComponents,
    pub active_modal: Option<ModalDialogV2>,
    next_request_id: u64,
}

impl AppState {
    pub fn new(cmd_tx: Sender<CommandEnvelope>, event_rx: Receiver<EventEnvelope>) -> Self {
        Self {
            cmd_tx,
            event_rx,
            ui_state: UiState::default(),
            data_cache: CachedData::default(),
            should_quit: false,
            components: AppComponents::new(),
            active_modal: None,
            next_request_id: 1,
        }
    }

    /// 分配新的请求 ID
    fn next_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        id
    }

    /// 发送命令（自动分配请求 ID）。队列满时记录日志并返回 Ok(0)（哨兵值，不追踪）。
    pub fn send_command(&mut self, command: crate::backend::BackendCommand) -> anyhow::Result<u64> {
        let request_id = self.next_request_id();
        match self
            .cmd_tx
            .try_send(CommandEnvelope::new(request_id, command))
        {
            Ok(()) => Ok(request_id),
            Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
                self.push_log("Backend busy: command dropped (queue full)".to_string());
                Ok(0)
            }
            Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
                self.should_quit = true;
                Ok(0)
            }
        }
    }

    pub fn push_log(&mut self, entry: String) {
        self.data_cache.log_entries.push(entry);
        if self.data_cache.log_entries.len() > 200 {
            let overflow = self.data_cache.log_entries.len() - 200;
            self.data_cache.log_entries.drain(0..overflow);
        }
    }

    #[allow(dead_code)]
    pub fn selected_file(&self) -> Option<&StatusEntry> {
        self.components
            .file_list_panel
            .selected_index()
            .and_then(|index| self.data_cache.files.get(index))
    }

    pub fn selected_branch(&self) -> Option<&BranchEntry> {
        self.components
            .branch_list_panel
            .selected_index()
            .and_then(|index| self.data_cache.branches.get(index))
    }

    // 同步组件列表状态与数据长度
    pub fn sync_file_list_state(&mut self) {
        self.components
            .file_list_panel
            .update_files(&self.data_cache.files);
        self.components.file_list_panel.clear_multi_select();
    }

    pub fn sync_branch_list_state(&mut self) {
        sync_list_state(
            self.components.branch_list_panel.state_mut(),
            self.data_cache.branches.len(),
        );
    }

    pub fn sync_commit_list_state(&mut self) {
        self.components.commit_panel.clear_list_multi_select();
        sync_list_state(
            self.components.commit_panel.state_mut(),
            self.data_cache.commits.len(),
        );
    }

    pub fn sync_stash_list_state(&mut self) {
        sync_list_state(
            self.components.stash_list_panel.state_mut(),
            self.data_cache.stashes.len(),
        );
    }
}

fn sync_list_state(state: &mut ListState, len: usize) {
    if len == 0 {
        state.select(None);
        return;
    }

    let current = state.selected().unwrap_or(0);
    state.select(Some(current.min(len.saturating_sub(1))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_file_list_state_keeps_tree_selection_index() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        state.data_cache.files = vec![
            StatusEntry {
                path: "src/components/mod.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
            StatusEntry {
                path: "src/components/core/tree.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
            StatusEntry {
                path: "src/components/panels/file_list.rs".to_string(),
                is_staged: false,
                is_unstaged: true,
                is_untracked: false,
            },
        ];

        state.sync_file_list_state();
        state.components.file_list_panel.state_mut().select(Some(6));
        let selected_before_sync = state.components.file_list_panel.selected_tree_node();

        state.sync_file_list_state();

        assert_eq!(state.components.file_list_panel.selected_index(), Some(6));
        assert_eq!(
            state.components.file_list_panel.selected_tree_node(),
            selected_before_sync
        );
    }

    #[test]
    fn test_push_log_basic() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        state.push_log("Test message 1".to_string());
        state.push_log("Test message 2".to_string());

        assert_eq!(state.data_cache.log_entries.len(), 2);
        assert_eq!(state.data_cache.log_entries[0], "Test message 1");
        assert_eq!(state.data_cache.log_entries[1], "Test message 2");
    }

    #[test]
    fn test_push_log_capacity_limit() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        // Add 250 messages (exceeds 200 limit)
        for i in 0..250 {
            state.push_log(format!("Message {}", i));
        }

        // Should only keep last 200
        assert_eq!(state.data_cache.log_entries.len(), 200);
        assert_eq!(state.data_cache.log_entries[0], "Message 50"); // First 50 dropped
        assert_eq!(state.data_cache.log_entries[199], "Message 249");
    }

    #[test]
    fn test_sync_branch_list_state() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        state.sync_branch_list_state();

        // Should select first item
        assert_eq!(state.components.branch_list_panel.selected_index(), Some(0));
    }

    #[test]
    fn test_sync_commits_list_state() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        state.data_cache.commits = vec![
            crate::backend::git_ops::CommitEntry {
                short_id: "abc123".to_string(),
                id: "abc123def456".to_string(),
                summary: "Commit 1".to_string(),
                body: None,
                author: "Author".to_string(),
                timestamp: 1234567890,
            },
        ];

        state.sync_commit_list_state();

        // Should select first item
        assert_eq!(state.components.commit_panel.selected_index(), Some(0));
    }

    #[test]
    fn test_sync_stash_list_state() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        state.data_cache.stashes = vec![
            crate::backend::git_ops::StashEntry {
                index: 0,
                id: "stash@{0}".to_string(),
                message: "Stash 1".to_string(),
            },
        ];

        state.sync_stash_list_state();

        // Should select first item
        assert_eq!(state.components.stash_list_panel.selected_index(), Some(0));
    }

    #[test]
    fn test_sync_empty_list() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        // Empty data
        state.data_cache.branches = vec![];
        state.data_cache.commits = vec![];
        state.data_cache.stashes = vec![];

        state.sync_branch_list_state();
        state.sync_commit_list_state();
        state.sync_stash_list_state();

        // Should have no selection
        assert_eq!(state.components.branch_list_panel.selected_index(), None);
        assert_eq!(state.components.commit_panel.selected_index(), None);
        assert_eq!(state.components.stash_list_panel.selected_index(), None);
    }

    #[test]
    fn test_sync_selected_out_of_bounds() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        // Set selection to index 5
        state.components.branch_list_panel.state_mut().select(Some(5));

        // But only have 2 branches
        state.data_cache.branches = vec![
            crate::backend::git_ops::BranchEntry {
                name: "main".to_string(),
                is_head: true,
                upstream: None,
            },
            crate::backend::git_ops::BranchEntry {
                name: "feature".to_string(),
                is_head: false,
                upstream: None,
            },
        ];

        state.sync_branch_list_state();

        // Should clamp to last valid index
        assert_eq!(state.components.branch_list_panel.selected_index(), Some(1));
    }

    #[test]
    fn test_new_state_initialization() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let state = AppState::new(cmd_tx, event_rx);

        // Verify initial state
        assert_eq!(state.data_cache.log_entries.len(), 0);
        assert_eq!(state.data_cache.files.len(), 0);
        assert_eq!(state.data_cache.branches.len(), 0);
        assert_eq!(state.data_cache.commits.len(), 0);
        assert_eq!(state.data_cache.stashes.len(), 0);
        assert!(state.active_modal.is_none());
    }

    #[test]
    fn test_modal_state_management() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::channel(4);
        let (_event_tx, event_rx) = tokio::sync::mpsc::channel(4);
        let mut state = AppState::new(cmd_tx, event_rx);

        // Initially no modal
        assert!(state.active_modal.is_none());

        // Set a modal
        let modal = crate::components::dialogs::ModalDialogV2::help(
            "Test Help".to_string(),
            vec![],
        );
        state.active_modal = Some(modal);

        assert!(state.active_modal.is_some());

        // Clear modal
        state.active_modal = None;

        assert!(state.active_modal.is_none());
    }
}
