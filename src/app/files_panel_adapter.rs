use crate::app::states::FilesPanelState;
use crate::flux::files_backend::{
    FilesPanelNode, FilesPanelNodeStatus, FilesPanelSelectionState, FilesPanelViewState,
};
use crate::ui::widgets::file_tree::{FileTreeNode, FileTreeNodeStatus};

pub fn selection_state_from_shell(state: &FilesPanelState) -> FilesPanelSelectionState {
    FilesPanelSelectionState {
        selected_index: state.panel.list_state.selected(),
        visual_mode: state.visual_mode,
        visual_anchor: state.visual_anchor,
    }
}

pub fn view_state_from_shell(state: &FilesPanelState) -> FilesPanelViewState {
    FilesPanelViewState {
        expanded_dirs: state.expanded_dirs.clone(),
        selection: selection_state_from_shell(state),
        nodes: state.tree_nodes.iter().map(node_from_shell).collect(),
    }
}

pub fn apply_view_state(state: &mut FilesPanelState, view: FilesPanelViewState) {
    state.expanded_dirs = view.expanded_dirs.clone();
    state.visual_mode = view.selection.visual_mode;
    state.visual_anchor = view.selection.visual_anchor;
    state.tree_nodes = view.nodes.iter().map(node_to_shell).collect();
    state.panel.list_state.select(view.selection.selected_index);
}

fn node_from_shell(node: &FileTreeNode) -> FilesPanelNode {
    FilesPanelNode {
        path: node.path.clone(),
        status: match &node.status {
            FileTreeNodeStatus::Unstaged(status) => FilesPanelNodeStatus::Unstaged(status.clone()),
            FileTreeNodeStatus::Staged(status) => FilesPanelNodeStatus::Staged(status.clone()),
            FileTreeNodeStatus::Untracked => FilesPanelNodeStatus::Untracked,
            FileTreeNodeStatus::Directory => FilesPanelNodeStatus::Directory,
        },
        depth: node.depth,
        is_dir: node.is_dir,
        is_expanded: node.is_expanded,
    }
}

fn node_to_shell(node: &FilesPanelNode) -> FileTreeNode {
    FileTreeNode {
        path: node.path.clone(),
        status: match &node.status {
            FilesPanelNodeStatus::Unstaged(status) => FileTreeNodeStatus::Unstaged(status.clone()),
            FilesPanelNodeStatus::Staged(status) => FileTreeNodeStatus::Staged(status.clone()),
            FilesPanelNodeStatus::Untracked => FileTreeNodeStatus::Untracked,
            FilesPanelNodeStatus::Directory => FileTreeNodeStatus::Directory,
        },
        depth: node.depth,
        is_dir: node.is_dir,
        is_expanded: node.is_expanded,
    }
}
