use crate::app::states::StashPanelState;
use crate::flux::git_backend::stash::{StashPanelViewState, StashTreeViewState};

pub fn view_state_from_shell(state: &StashPanelState) -> StashPanelViewState {
    StashPanelViewState {
        selected_index: state.panel.list_state.selected(),
        items: state.items.clone(),
        tree_mode: StashTreeViewState {
            active: state.tree_mode.active,
            selected_source: state.tree_mode.selected_source,
            nodes: state.tree_mode.nodes.clone(),
            files: state.tree_mode.files.clone(),
            expanded_dirs: state.tree_mode.expanded_dirs.clone(),
        },
    }
}

pub fn apply_view_state(state: &mut StashPanelState, view: StashPanelViewState) {
    state.items = view.items;
    state.tree_mode.active = view.tree_mode.active;
    state.tree_mode.selected_source = view.tree_mode.selected_source;
    state.tree_mode.nodes = view.tree_mode.nodes;
    state.tree_mode.files = view.tree_mode.files;
    state.tree_mode.expanded_dirs = view.tree_mode.expanded_dirs;
    state.panel.list_state.select(view.selected_index);
}
