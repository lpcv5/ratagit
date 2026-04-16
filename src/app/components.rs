use crate::components::panels::{
    BranchListPanel, CommitModeView, CommitPanel, FileListPanel, LogPanel, MainViewPanel,
    StashListPanel,
};
use crate::components::Component;
use crate::components::component_v2::ComponentV2;
use crate::components::Intent;
use crate::app::events::AppEvent;
use crate::app::AppState;
use crossterm::event::{Event, KeyEvent};

use super::{CachedData, Panel};

/// 所有面板组件的容器
pub struct AppComponents {
    pub file_list_panel: FileListPanel,
    pub branch_list_panel: BranchListPanel,
    pub commit_panel: CommitPanel,
    pub stash_list_panel: StashListPanel,
    pub main_view_panel: MainViewPanel,
    pub log_panel: LogPanel,
}

impl AppComponents {
    pub fn new() -> Self {
        Self {
            file_list_panel: FileListPanel::new(),
            branch_list_panel: BranchListPanel::new(),
            commit_panel: CommitPanel::new(),
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
            Panel::Commits => self.commit_panel.handle_event(event, data),
            Panel::Stash => self.stash_list_panel.handle_event(event, data),
            Panel::MainView => self.main_view_panel.handle_event(event, data),
            Panel::Log => self.log_panel.handle_event(event, data),
        }
    }

    /// Dispatch key event to active panel using ComponentV2 trait (returns AppEvent)
    pub fn dispatch_key_event_v2(
        &mut self,
        active_panel: Panel,
        key: KeyEvent,
        state: &AppState,
    ) -> AppEvent {
        match active_panel {
            Panel::Files => self.file_list_panel.handle_key_event(key, state),
            Panel::Branches => self.branch_list_panel.handle_key_event(key, state),
            Panel::Commits => self.commit_panel.handle_key_event(key, state),
            Panel::Stash => self.stash_list_panel.handle_key_event(key, state),
            Panel::MainView => self.main_view_panel.handle_key_event(key, state),
            Panel::Log => self.log_panel.handle_key_event(key, state),
        }
    }

    pub fn scroll_main_view_by(&mut self, delta: i16, max_scroll: u16) {
        self.main_view_panel.scroll_by_clamped(delta, max_scroll);
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

    pub fn selected_file_tree_node(&self) -> Option<(String, bool)> {
        self.file_list_panel.selected_tree_node()
    }

    pub fn selected_file_tree_targets(&self) -> Vec<(String, bool)> {
        self.file_list_panel.selected_tree_targets()
    }

    pub fn selected_file_anchor_target(&self) -> Option<(String, bool)> {
        self.file_list_panel.anchor_tree_target()
    }

    pub fn is_file_multi_select_active(&self) -> bool {
        self.file_list_panel.is_multi_select_active()
    }

    pub fn show_branch_commits(&mut self) {
        self.branch_list_panel.show_branch_commits();
    }

    #[allow(dead_code)]
    pub fn selected_branch_index(&self) -> Option<usize> {
        self.branch_list_panel.selected_index()
    }

    #[allow(dead_code)]
    pub fn selected_commit_index(&self) -> Option<usize> {
        self.commit_panel.selected_index()
    }

    pub fn commit_mode_view(&self) -> CommitModeView {
        self.commit_panel.mode_view()
    }

    pub fn commit_pending_commit_id(&self) -> Option<&str> {
        self.commit_panel.pending_commit_id()
    }

    pub fn selected_commit_tree_node(&self) -> Option<(String, bool)> {
        self.commit_panel.selected_tree_node()
    }

    pub fn selected_commit_tree_targets(&self) -> Vec<(String, bool)> {
        self.commit_panel.selected_tree_targets()
    }

    pub fn is_commit_tree_multi_select_active(&self) -> bool {
        self.commit_panel.is_tree_multi_select_active()
    }

    pub fn is_commit_list_multi_select_active(&self) -> bool {
        self.commit_panel.is_list_multi_select_active()
    }

    pub fn refresh_commit_list_multi_range(
        &mut self,
        commits: &[crate::backend::git_ops::CommitEntry],
    ) {
        self.commit_panel.refresh_list_multi_range(commits);
    }

    #[allow(dead_code)]
    pub fn selected_stash_index(&self) -> Option<usize> {
        self.stash_list_panel.selected_index()
    }
}

impl Default for AppComponents {
    fn default() -> Self {
        Self::new()
    }
}
