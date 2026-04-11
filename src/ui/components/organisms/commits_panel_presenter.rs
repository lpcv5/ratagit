use crate::git::CommitInfo;
use std::collections::HashSet;

#[derive(Debug, Clone, Default)]
pub struct CommitsTreeViewState {
    pub active: bool,
    pub selected_source: Option<String>,
    pub nodes: Vec<crate::ui::widgets::file_tree::FileTreeNode>,
}

#[derive(Debug, Clone, Default)]
pub struct CommitsPanelViewState {
    pub selected_index: Option<usize>,
    pub items: Vec<CommitInfo>,
    pub tree_mode: CommitsTreeViewState,
    pub highlighted_oids: HashSet<String>,
}
