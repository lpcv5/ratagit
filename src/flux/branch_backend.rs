use crate::flux::action::{Action, DomainAction};
use crate::git::{BranchInfo, CommitInfo};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchPanelListItem {
    pub name: String,
    pub is_current: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BranchPanelSelectionState {
    pub selected_index: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct BranchCommitsSubviewState {
    pub active: bool,
    pub loading: bool,
    pub source_branch: Option<String>,
    pub selected_index: Option<usize>,
    pub items: Vec<CommitInfo>,
    pub highlighted_oids: HashSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BranchPanelViewState {
    pub items: Vec<BranchPanelListItem>,
    pub selection: BranchPanelSelectionState,
    pub is_fetching_remote: bool,
    pub commits_subview: BranchCommitsSubviewState,
}

#[derive(Debug, Clone)]
pub enum BranchBackendCommand {
    CreateBranch(String),
    CheckoutBranch { name: String, auto_stash: bool },
    DeleteBranch(String),
    FetchRemote,
    OpenCommitsSubview { branch: String, limit: usize },
}

#[derive(Debug, Clone)]
#[allow(clippy::enum_variant_names)]
pub enum BranchBackendEvent {
    CreateFinished {
        name: String,
        result: Result<(), String>,
    },
    CheckoutFinished {
        name: String,
        auto_stash: bool,
        result: Result<(), String>,
    },
    DeleteFinished {
        name: String,
        result: Result<(), String>,
    },
    FetchFinished(Result<String, String>),
}

impl BranchBackendEvent {
    pub fn into_action(self) -> Action {
        match self {
            BranchBackendEvent::CreateFinished { name, result } => {
                Action::Domain(DomainAction::CreateBranchFinished { name, result })
            }
            BranchBackendEvent::CheckoutFinished {
                name,
                auto_stash,
                result,
            } => Action::Domain(DomainAction::CheckoutBranchFinished {
                name,
                auto_stash,
                result,
            }),
            BranchBackendEvent::DeleteFinished { name, result } => {
                Action::Domain(DomainAction::DeleteBranchFinished { name, result })
            }
            BranchBackendEvent::FetchFinished(result) => {
                Action::Domain(DomainAction::FetchRemoteFinished(result))
            }
        }
    }
}

pub struct BranchBackend;

impl BranchBackend {
    pub fn refresh_branches(
        branches: &[BranchInfo],
        current: BranchPanelViewState,
    ) -> BranchPanelViewState {
        let selected_name = current_selected_branch_name(&current);
        let source_branch = current.commits_subview.source_branch.clone();
        let items = branches.iter().map(item_from_branch).collect::<Vec<_>>();
        let mut next = BranchPanelViewState {
            items,
            selection: BranchPanelSelectionState {
                selected_index: selected_name
                    .as_deref()
                    .and_then(|name| branches.iter().position(|branch| branch.name == name)),
            },
            is_fetching_remote: current.is_fetching_remote,
            commits_subview: current.commits_subview,
        };

        if next.commits_subview.active
            && source_branch
                .as_deref()
                .is_some_and(|name| branch_index_by_name(&next.items, name).is_none())
        {
            next = Self::close_commits_subview(next);
        }

        clamp_selection(&mut next);
        next
    }

    pub fn set_fetching_remote(
        mut current: BranchPanelViewState,
        is_fetching_remote: bool,
    ) -> BranchPanelViewState {
        current.is_fetching_remote = is_fetching_remote;
        current
    }

    pub fn open_commits_subview(
        mut current: BranchPanelViewState,
        branch: String,
    ) -> BranchPanelViewState {
        current.commits_subview.active = true;
        current.commits_subview.loading = true;
        current.commits_subview.source_branch = Some(branch);
        current.commits_subview.selected_index = None;
        current.commits_subview.items.clear();
        current.commits_subview.highlighted_oids.clear();
        current
    }

    pub fn apply_commits_subview_loaded(
        mut current: BranchPanelViewState,
        branch: &str,
        items: Vec<CommitInfo>,
    ) -> BranchPanelViewState {
        if current.commits_subview.source_branch.as_deref() != Some(branch)
            || !current.commits_subview.active
        {
            return current;
        }
        current.commits_subview.loading = false;
        current.commits_subview.items = items;
        if current.commits_subview.selected_index.is_none() {
            current.commits_subview.selected_index =
                (!current.commits_subview.items.is_empty()).then_some(0);
        }
        current
    }

    pub fn fail_commits_subview_load(
        mut current: BranchPanelViewState,
        branch: &str,
    ) -> BranchPanelViewState {
        if current.commits_subview.source_branch.as_deref() != Some(branch)
            || !current.commits_subview.active
        {
            return current;
        }
        current.commits_subview.loading = false;
        current
    }

    pub fn close_commits_subview(mut current: BranchPanelViewState) -> BranchPanelViewState {
        let source_branch = current.commits_subview.source_branch.take();
        current.commits_subview = BranchCommitsSubviewState::default();
        current.selection.selected_index = source_branch
            .as_deref()
            .and_then(|name| branch_index_by_name(&current.items, name))
            .or_else(|| (!current.items.is_empty()).then_some(0));
        current
    }
}

fn item_from_branch(branch: &BranchInfo) -> BranchPanelListItem {
    BranchPanelListItem {
        name: branch.name.clone(),
        is_current: branch.is_current,
    }
}

fn current_selected_branch_name(view: &BranchPanelViewState) -> Option<String> {
    if view.commits_subview.active {
        return view.commits_subview.source_branch.clone();
    }
    view.selection
        .selected_index
        .and_then(|idx| view.items.get(idx))
        .map(|branch| branch.name.clone())
}

fn branch_index_by_name(items: &[BranchPanelListItem], name: &str) -> Option<usize> {
    items.iter().position(|branch| branch.name == name)
}

fn clamp_selection(view: &mut BranchPanelViewState) {
    if view.items.is_empty() {
        view.selection.selected_index = None;
        return;
    }
    view.selection.selected_index = Some(
        view.selection
            .selected_index
            .unwrap_or(0)
            .min(view.items.len() - 1),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommitSyncState, GraphCell};

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

    fn branches(names: &[(&str, bool)]) -> Vec<BranchInfo> {
        names
            .iter()
            .map(|(name, is_current)| BranchInfo {
                name: (*name).to_string(),
                is_current: *is_current,
            })
            .collect()
    }

    #[test]
    fn refresh_branches_preserves_selected_branch_by_name() {
        let current = BranchPanelViewState {
            items: vec![
                BranchPanelListItem {
                    name: "main".to_string(),
                    is_current: true,
                },
                BranchPanelListItem {
                    name: "feature/x".to_string(),
                    is_current: false,
                },
            ],
            selection: BranchPanelSelectionState {
                selected_index: Some(1),
            },
            is_fetching_remote: false,
            commits_subview: BranchCommitsSubviewState::default(),
        };

        let refreshed = BranchBackend::refresh_branches(
            &branches(&[("develop", false), ("feature/x", false), ("main", true)]),
            current,
        );

        assert_eq!(refreshed.selection.selected_index, Some(1));
        assert_eq!(refreshed.items[1].name, "feature/x");
    }

    #[test]
    fn open_and_close_commits_subview_restores_branch_selection() {
        let current = BranchPanelViewState {
            items: vec![
                BranchPanelListItem {
                    name: "main".to_string(),
                    is_current: true,
                },
                BranchPanelListItem {
                    name: "feature/x".to_string(),
                    is_current: false,
                },
            ],
            selection: BranchPanelSelectionState {
                selected_index: Some(0),
            },
            is_fetching_remote: false,
            commits_subview: BranchCommitsSubviewState::default(),
        };

        let opened = BranchBackend::open_commits_subview(current, "feature/x".to_string());
        assert!(opened.commits_subview.active);
        assert!(opened.commits_subview.loading);

        let closed = BranchBackend::close_commits_subview(opened);
        assert!(!closed.commits_subview.active);
        assert_eq!(closed.selection.selected_index, Some(1));
    }

    #[test]
    fn apply_commits_subview_loaded_selects_first_commit() {
        let current = BranchBackend::open_commits_subview(
            BranchPanelViewState {
                items: vec![BranchPanelListItem {
                    name: "main".to_string(),
                    is_current: true,
                }],
                selection: BranchPanelSelectionState {
                    selected_index: Some(0),
                },
                is_fetching_remote: false,
                commits_subview: BranchCommitsSubviewState::default(),
            },
            "main".to_string(),
        );

        let loaded =
            BranchBackend::apply_commits_subview_loaded(current, "main", vec![commit("abc123")]);

        assert!(!loaded.commits_subview.loading);
        assert_eq!(loaded.commits_subview.selected_index, Some(0));
        assert_eq!(loaded.commits_subview.items.len(), 1);
    }

    #[test]
    fn apply_commits_subview_loaded_preserves_selection_on_incremental_load() {
        let base = BranchPanelViewState {
            items: vec![BranchPanelListItem {
                name: "main".to_string(),
                is_current: true,
            }],
            selection: BranchPanelSelectionState {
                selected_index: Some(0),
            },
            is_fetching_remote: false,
            commits_subview: BranchCommitsSubviewState::default(),
        };
        let after_first_load = BranchBackend::apply_commits_subview_loaded(
            BranchBackend::open_commits_subview(base, "main".to_string()),
            "main",
            vec![commit("abc"), commit("def"), commit("ghi")],
        );
        // Simulate user scrolling to index 2
        let mut scrolled = after_first_load;
        scrolled.commits_subview.selected_index = Some(2);

        // Incremental load arrives with more commits
        let after_incremental = BranchBackend::apply_commits_subview_loaded(
            scrolled,
            "main",
            vec![commit("abc"), commit("def"), commit("ghi"), commit("jkl"), commit("mno")],
        );

        // Selection must NOT jump back to 0
        assert_eq!(after_incremental.commits_subview.selected_index, Some(2));
        assert_eq!(after_incremental.commits_subview.items.len(), 5);
    }

    #[test]
    fn fail_commits_subview_load_keeps_subview_open_but_clears_loading() {
        let current = BranchBackend::open_commits_subview(
            BranchPanelViewState {
                items: vec![BranchPanelListItem {
                    name: "main".to_string(),
                    is_current: true,
                }],
                selection: BranchPanelSelectionState {
                    selected_index: Some(0),
                },
                is_fetching_remote: false,
                commits_subview: BranchCommitsSubviewState::default(),
            },
            "main".to_string(),
        );

        let failed = BranchBackend::fail_commits_subview_load(current, "main");

        assert!(failed.commits_subview.active);
        assert!(!failed.commits_subview.loading);
    }
}
