use crate::app::states::{BranchesPanelState, CommitsPanelState};
use crate::flux::branch_backend::{
    BranchCommitsSubviewState, BranchPanelListItem, BranchPanelSelectionState, BranchPanelViewState,
};

pub fn selection_state_from_shell(state: &BranchesPanelState) -> BranchPanelSelectionState {
    BranchPanelSelectionState {
        selected_index: state.panel.list_state.selected(),
    }
}

pub fn view_state_from_shell(state: &BranchesPanelState) -> BranchPanelViewState {
    BranchPanelViewState {
        items: state.items.iter().map(item_from_shell).collect(),
        selection: selection_state_from_shell(state),
        is_fetching_remote: state.is_fetching_remote,
        commits_subview: commits_subview_from_shell(state),
    }
}

pub fn apply_view_state(state: &mut BranchesPanelState, view: BranchPanelViewState) {
    state.items = view.items.iter().map(item_to_shell).collect();
    state.is_fetching_remote = view.is_fetching_remote;
    state.panel.list_state.select(view.selection.selected_index);
    state.commits_subview_active = view.commits_subview.active;
    state.commits_subview_loading = view.commits_subview.loading;
    state.commits_subview_source = view.commits_subview.source_branch.clone();
    apply_commits_subview_state(&mut state.commits_subview, &view.commits_subview);
}

fn item_from_shell(item: &crate::git::BranchInfo) -> BranchPanelListItem {
    BranchPanelListItem {
        name: item.name.clone(),
        is_current: item.is_current,
    }
}

fn item_to_shell(item: &BranchPanelListItem) -> crate::git::BranchInfo {
    crate::git::BranchInfo {
        name: item.name.clone(),
        is_current: item.is_current,
    }
}

fn commits_subview_from_shell(state: &BranchesPanelState) -> BranchCommitsSubviewState {
    BranchCommitsSubviewState {
        active: state.commits_subview_active,
        loading: state.commits_subview_loading,
        source_branch: state.commits_subview_source.clone(),
        selected_index: state.commits_subview.panel.list_state.selected(),
        items: state.commits_subview.items.clone(),
        highlighted_oids: state.commits_subview.highlighted_oids.clone(),
    }
}

fn apply_commits_subview_state(target: &mut CommitsPanelState, source: &BranchCommitsSubviewState) {
    target.items = source.items.clone();
    target.dirty = false;
    target.highlighted_oids = source.highlighted_oids.clone();
    target.tree_mode = Default::default();
    target.panel.list_state.select(source.selected_index);
}
