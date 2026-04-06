use crate::git::{BranchInfo, CommitInfo, FileEntry, StashInfo};
use crate::ui::widgets::file_tree::FileTreeNode;
use ratatui::widgets::ListState;
use std::collections::HashSet;
use std::path::PathBuf;

/// The four side panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SidePanel {
    #[default]
    Files,
    LocalBranches,
    Commits,
    Stash,
}

/// Holds a `ratatui` `ListState` for a single scrollable panel.
#[derive(Clone)]
pub struct PanelState {
    pub list_state: ListState,
}

impl PanelState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));
        Self { list_state }
    }
}

impl Default for PanelState {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic tree-navigation state for commit and stash panels.
#[derive(Clone)]
pub struct TreeModeState<T> {
    pub active: bool,
    pub nodes: Vec<FileTreeNode>,
    pub files: Vec<FileEntry>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub selected_source: Option<T>,
}

impl<T> Default for TreeModeState<T> {
    fn default() -> Self {
        Self {
            active: false,
            nodes: Vec::new(),
            files: Vec::new(),
            expanded_dirs: HashSet::new(),
            selected_source: None,
        }
    }
}

#[derive(Default, Clone)]
pub struct FilesPanelState {
    pub panel: PanelState,
    pub tree_nodes: Vec<FileTreeNode>,
    pub expanded_dirs: HashSet<PathBuf>,
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,
}

#[derive(Default, Clone)]
pub struct BranchesPanelState {
    pub panel: PanelState,
    pub items: Vec<BranchInfo>,
    pub is_fetching_remote: bool,
    pub commits_subview_active: bool,
    pub commits_subview_loading: bool,
    pub commits_subview_source: Option<String>,
    pub commits_subview: CommitsPanelState,
}

#[derive(Default, Clone)]
pub struct CommitsPanelState {
    pub panel: PanelState,
    pub items: Vec<CommitInfo>,
    pub dirty: bool,
    pub tree_mode: TreeModeState<String>,
    pub highlighted_oids: HashSet<String>,
}

#[derive(Default, Clone)]
pub struct StashPanelState {
    pub panel: PanelState,
    pub items: Vec<StashInfo>,
    pub tree_mode: TreeModeState<usize>,
}

/// A single entry in the command/operation log shown in the UI.
#[derive(Clone)]
pub struct CommandLogEntry {
    pub command: String,
    pub success: bool,
}

/// Per-render cached values that are expensive to recompute each frame.
#[derive(Default, Clone)]
pub struct RenderCache {
    pub files_visual_selected_indices: HashSet<usize>,
    pub files_search_summary: Option<String>,
    pub branches_search_summary: Option<String>,
    pub commits_search_summary: Option<String>,
    pub stash_search_summary: Option<String>,
}
