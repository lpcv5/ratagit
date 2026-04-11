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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flux::branch_backend::BranchBackend;
    use crate::git::{BranchInfo, CommitInfo, CommitSyncState, GraphCell};
    use pretty_assertions::assert_eq;

    fn commit(oid: &str) -> CommitInfo {
        CommitInfo {
            oid: oid.to_string(),
            message: format!("commit {}", oid),
            author: "tester".to_string(),
            graph: vec![GraphCell {
                text: "●".to_string(),
                lane: 0,
                pipe_oid: None,
                pipe_oids: vec![],
            }],
            time: "2026-04-11 00:00".to_string(),
            parent_count: 1,
            sync_state: CommitSyncState::DefaultBranch,
            parent_oids: vec![],
        }
    }

    fn branch_state() -> BranchesPanelState {
        let mut state = BranchesPanelState {
            items: vec![
                BranchInfo {
                    name: "main".to_string(),
                    is_current: true,
                },
                BranchInfo {
                    name: "feature/x".to_string(),
                    is_current: false,
                },
            ],
            ..Default::default()
        };
        state.panel.list_state.select(Some(1));
        state
    }

    #[test]
    fn apply_view_state_can_close_commits_subview_and_restore_branch_selection() {
        let mut state = branch_state();
        state.commits_subview_active = true;
        state.commits_subview_loading = true;
        state.commits_subview_source = Some("feature/x".to_string());
        state.commits_subview.items = vec![commit("abc123")];

        let view = BranchBackend::close_commits_subview(view_state_from_shell(&state));
        apply_view_state(&mut state, view);

        assert!(!state.commits_subview_active);
        assert_eq!(state.panel.list_state.selected(), Some(1));
    }

    #[test]
    fn apply_view_state_can_mark_commits_subview_load_failure_without_closing() {
        let mut state = branch_state();
        state.commits_subview_active = true;
        state.commits_subview_loading = true;
        state.commits_subview_source = Some("feature/x".to_string());

        let mut view = view_state_from_shell(&state);
        view.commits_subview.loading = false;
        apply_view_state(&mut state, view);

        assert!(state.commits_subview_active);
        assert!(!state.commits_subview_loading);
        assert_eq!(state.commits_subview_source.as_deref(), Some("feature/x"));
    }
}
