use crate::app::events::AppEvent;
use crate::app::Panel;
use crate::components::panels::{
    BranchListPanel, CommitPanel, FileListPanel, LogPanel, MainViewPanel, StashListPanel,
};

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

    pub fn scroll_main_view_by(&mut self, delta: i16, max_scroll: u16) {
        self.main_view_panel.scroll_by_clamped(delta, max_scroll);
    }

    pub fn main_view_scroll_to(&mut self, offset: u16) {
        self.main_view_panel.scroll_to(offset);
    }

    #[allow(dead_code)]
    pub fn selected_file_index(&self) -> Option<usize> {
        self.file_list_panel.selected_index()
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

    pub fn hide_branch_commits(&mut self) {
        self.branch_list_panel.hide_branch_commits();
    }

    pub fn handle_escape(&mut self, active_panel: Panel) -> AppEvent {
        match active_panel {
            Panel::Files => self.file_list_panel.handle_escape(),
            Panel::Branches => self.branch_list_panel.handle_escape(),
            Panel::Commits => self.commit_panel.handle_escape(),
            Panel::Stash | Panel::MainView | Panel::Log => AppEvent::None,
        }
    }

    #[allow(dead_code)]
    pub fn selected_branch_index(&self) -> Option<usize> {
        self.branch_list_panel.selected_index()
    }

    #[allow(dead_code)]
    pub fn selected_commit_index(&self) -> Option<usize> {
        self.commit_panel.selected_index()
    }

    pub fn commit_pending_commit_id(&self) -> Option<&str> {
        self.commit_panel.pending_commit_id()
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
