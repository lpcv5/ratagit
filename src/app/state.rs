use ratatui::widgets::ListState;

use crate::backend::git_ops::{BranchEntry, CommitEntry, StashEntry, StatusEntry};
use crate::backend::{CommandEnvelope, EventEnvelope};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

use super::components::AppComponents;
use super::{CachedData, UiState};

/// AppState：封装所有应用状态
pub struct AppState {
    pub ui_state: UiState,
    pub data_cache: CachedData,
    pub cmd_tx: UnboundedSender<CommandEnvelope>,
    pub event_rx: UnboundedReceiver<EventEnvelope>,
    pub should_quit: bool,
    pub components: AppComponents,
    next_request_id: u64,
}

impl AppState {
    pub fn new(
        cmd_tx: UnboundedSender<CommandEnvelope>,
        event_rx: UnboundedReceiver<EventEnvelope>,
    ) -> Self {
        Self {
            cmd_tx,
            event_rx,
            ui_state: UiState::default(),
            data_cache: CachedData::default(),
            should_quit: false,
            components: AppComponents::new(),
            next_request_id: 1,
        }
    }

    /// 分配新的请求 ID
    fn next_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        id
    }

    /// 发送命令（自动分配请求 ID）
    pub fn send_command(&mut self, command: crate::backend::BackendCommand) -> anyhow::Result<u64> {
        let request_id = self.next_request_id();
        self.cmd_tx
            .send(CommandEnvelope::new(request_id, command))?;
        Ok(request_id)
    }

    pub fn push_log(&mut self, entry: String) {
        self.data_cache.log_entries.push(entry);
        if self.data_cache.log_entries.len() > 200 {
            let overflow = self.data_cache.log_entries.len() - 200;
            self.data_cache.log_entries.drain(0..overflow);
        }
    }

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

    pub fn selected_commit(&self) -> Option<&CommitEntry> {
        self.components
            .commit_list_panel
            .selected_index()
            .and_then(|index| self.data_cache.commits.get(index))
    }

    pub fn selected_stash(&self) -> Option<&StashEntry> {
        self.components
            .stash_list_panel
            .selected_index()
            .and_then(|index| self.data_cache.stashes.get(index))
    }

    // 同步组件列表状态与数据长度
    pub fn sync_file_list_state(&mut self) {
        sync_list_state(
            self.components.file_list_panel.state_mut(),
            self.data_cache.files.len(),
        );
    }

    pub fn sync_branch_list_state(&mut self) {
        sync_list_state(
            self.components.branch_list_panel.state_mut(),
            self.data_cache.branches.len(),
        );
    }

    pub fn sync_commit_list_state(&mut self) {
        sync_list_state(
            self.components.commit_list_panel.state_mut(),
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
