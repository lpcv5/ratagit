use crate::app::states::CommitsPanelState;
use crate::flux::commits_backend::{CommitsPanelViewState, CommitsTreeViewState};

pub fn view_state_from_shell(state: &CommitsPanelState) -> CommitsPanelViewState {
    CommitsPanelViewState {
        selected_index: state.panel.list_state.selected(),
        items: state.items.clone(),
        tree_mode: CommitsTreeViewState {
            active: state.tree_mode.active,
            selected_source: state.tree_mode.selected_source.clone(),
            nodes: state.tree_mode.nodes.clone(),
            files: state.tree_mode.files.clone(),
            expanded_dirs: state.tree_mode.expanded_dirs.clone(),
        },
        highlighted_oids: state.highlighted_oids.clone(),
    }
}

pub fn apply_view_state(state: &mut CommitsPanelState, view: CommitsPanelViewState) {
    state.items = view.items;
    state.tree_mode.active = view.tree_mode.active;
    state.tree_mode.selected_source = view.tree_mode.selected_source;
    state.tree_mode.nodes = view.tree_mode.nodes;
    state.tree_mode.files = view.tree_mode.files;
    state.tree_mode.expanded_dirs = view.tree_mode.expanded_dirs;
    state.highlighted_oids = view.highlighted_oids;
    state.panel.list_state.select(view.selected_index);
}
