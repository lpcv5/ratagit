use ratatui::widgets::ListState;

use crate::components::panels::{
    BranchListPanel, CommitListPanel, FileListPanel, LogPanel, MainViewPanel, StashListPanel,
};
use crate::components::Component;
use crate::components::Intent;
use crossterm::event::Event;

use super::{CachedData, Panel};

/// 所有面板组件的容器
pub struct AppComponents {
    pub file_list_panel: FileListPanel,
    pub branch_list_panel: BranchListPanel,
    pub commit_list_panel: CommitListPanel,
    pub stash_list_panel: StashListPanel,
    pub main_view_panel: MainViewPanel,
    pub log_panel: LogPanel,
}

impl AppComponents {
    pub fn new() -> Self {
        Self {
            file_list_panel: FileListPanel::new(),
            branch_list_panel: BranchListPanel::new(),
            commit_list_panel: CommitListPanel::new(),
            stash_list_panel: StashListPanel::new(),
            main_view_panel: MainViewPanel::new(),
            log_panel: LogPanel::new(),
        }
    }

    /// 分发事件到当前活动面板
    pub fn dispatch_event(
        &mut self,
        active_panel: Panel,
        event: &Event,
        data: &CachedData,
    ) -> Intent {
        match active_panel {
            Panel::Files => self.file_list_panel.handle_event(event, data),
            Panel::Branches => self.branch_list_panel.handle_event(event, data),
            Panel::Commits => self.commit_list_panel.handle_event(event, data),
            Panel::Stash => self.stash_list_panel.handle_event(event, data),
            Panel::MainView => self.main_view_panel.handle_event(event, data),
            Panel::Log => self.log_panel.handle_event(event, data),
        }
    }

    pub fn scroll_main_view_by(&mut self, delta: i16) {
        self.main_view_panel.scroll_by(delta);
    }

    pub fn scroll_log_by(&mut self, delta: i16) {
        self.log_panel.scroll_by(delta);
    }

    pub fn main_view_scroll_to(&mut self, offset: u16) {
        self.main_view_panel.scroll_to(offset);
    }

    #[allow(dead_code)]
    pub fn selected_file_index(&self) -> Option<usize> {
        self.file_list_panel.selected_index()
    }

    #[allow(dead_code)]
    pub fn selected_branch_index(&self) -> Option<usize> {
        self.branch_list_panel.selected_index()
    }

    #[allow(dead_code)]
    pub fn selected_commit_index(&self) -> Option<usize> {
        self.commit_list_panel.selected_index()
    }

    #[allow(dead_code)]
    pub fn selected_stash_index(&self) -> Option<usize> {
        self.stash_list_panel.selected_index()
    }

    pub fn file_list_state_mut(&mut self) -> &mut ListState {
        self.file_list_panel.state_mut()
    }

    pub fn branch_list_state_mut(&mut self) -> &mut ListState {
        self.branch_list_panel.state_mut()
    }

    pub fn commit_list_state_mut(&mut self) -> &mut ListState {
        self.commit_list_panel.state_mut()
    }

    pub fn stash_list_state_mut(&mut self) -> &mut ListState {
        self.stash_list_panel.state_mut()
    }
}

impl Default for AppComponents {
    fn default() -> Self {
        Self::new()
    }
}
