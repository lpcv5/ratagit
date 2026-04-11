use crate::app::graph_highlight::compute_highlight_set;
use crate::app::SidePanel;
use crate::git::{CommitInfo, FileEntry};
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitsLoadMode {
    Full,
    Fast,
}

#[derive(Debug, Clone)]
pub enum CommitsBackendCommand {
    ApplyLoaded {
        items: Vec<CommitInfo>,
        mode: CommitsLoadMode,
    },
    CloseTree,
    OpenTreeOrToggleDir,
    RecomputeHighlight,
}

#[derive(Debug, Clone, Default)]
pub struct CommitsTreeViewState {
    pub active: bool,
    pub selected_source: Option<String>,
    pub nodes: Vec<FileTreeNode>,
    pub files: Vec<FileEntry>,
    pub expanded_dirs: HashSet<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct CommitsPanelViewState {
    pub selected_index: Option<usize>,
    pub items: Vec<CommitInfo>,
    pub tree_mode: CommitsTreeViewState,
    pub highlighted_oids: HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommitsPanelDiffRequest {
    None,
    Commit { oid: String, path: Option<PathBuf> },
}

#[derive(Debug, Clone)]
pub enum CommitsBackendEvent {
    ViewStateUpdated {
        view: CommitsPanelViewState,
        dirty: Option<bool>,
    },
}

pub struct CommitsBackend;

impl CommitsBackend {
    pub fn apply_loaded(
        mut current: CommitsPanelViewState,
        items: Vec<CommitInfo>,
        mode: CommitsLoadMode,
    ) -> CommitsBackendEvent {
        current.items = items;
        CommitsBackendEvent::ViewStateUpdated {
            view: current,
            dirty: Some(matches!(mode, CommitsLoadMode::Fast)),
        }
    }

    pub fn recompute_highlight(
        mut current: CommitsPanelViewState,
        active_panel: SidePanel,
    ) -> CommitsBackendEvent {
        if active_panel == SidePanel::Commits && !current.tree_mode.active {
            if let Some(commit) = current
                .selected_index
                .and_then(|idx| current.items.get(idx))
            {
                current.highlighted_oids = compute_highlight_set(&current.items, &commit.oid);
                return CommitsBackendEvent::ViewStateUpdated {
                    view: current,
                    dirty: None,
                };
            }
        }

        current.highlighted_oids.clear();
        CommitsBackendEvent::ViewStateUpdated {
            view: current,
            dirty: None,
        }
    }

    pub fn close_tree(mut current: CommitsPanelViewState) -> CommitsBackendEvent {
        if current.tree_mode.active {
            let selected_source_index = current
                .tree_mode
                .selected_source
                .as_ref()
                .and_then(|oid| current.items.iter().position(|commit| commit.oid == *oid));

            current.tree_mode.active = false;
            current.tree_mode.nodes.clear();
            current.tree_mode.files.clear();
            current.tree_mode.expanded_dirs.clear();
            current.tree_mode.selected_source = None;
            current.selected_index = selected_source_index.or({
                if current.items.is_empty() {
                    None
                } else {
                    Some(0)
                }
            });
        }

        CommitsBackendEvent::ViewStateUpdated {
            view: current,
            dirty: None,
        }
    }

    pub fn open_tree(
        mut current: CommitsPanelViewState,
        selected_source: String,
        files: Vec<FileEntry>,
    ) -> CommitsBackendEvent {
        let expanded_dirs = collect_dirs_from_entries(&files);
        let nodes = FileTree::from_git_status_with_expanded(&files, &[], &[], &expanded_dirs);
        current.tree_mode.active = true;
        current.tree_mode.selected_source = Some(selected_source);
        current.tree_mode.files = files;
        current.tree_mode.expanded_dirs = expanded_dirs;
        current.tree_mode.nodes = nodes;
        current.selected_index = if current.tree_mode.nodes.is_empty() {
            None
        } else {
            Some(0)
        };

        CommitsBackendEvent::ViewStateUpdated {
            view: current,
            dirty: None,
        }
    }

    pub fn toggle_tree_dir(mut current: CommitsPanelViewState) -> CommitsBackendEvent {
        let selected_node = current
            .selected_index
            .and_then(|idx| current.tree_mode.nodes.get(idx))
            .cloned();
        if let Some(node) = selected_node.filter(|node| node.is_dir) {
            if !current.tree_mode.expanded_dirs.insert(node.path.clone()) {
                current.tree_mode.expanded_dirs.remove(&node.path);
            }
            current.tree_mode.nodes = FileTree::from_git_status_with_expanded(
                &current.tree_mode.files,
                &[],
                &[],
                &current.tree_mode.expanded_dirs,
            );
            if current.tree_mode.nodes.is_empty() {
                current.selected_index = None;
            } else {
                current.selected_index = current
                    .tree_mode
                    .nodes
                    .iter()
                    .position(|candidate| {
                        candidate.path == node.path && candidate.is_dir == node.is_dir
                    })
                    .or_else(|| {
                        Some(
                            current
                                .selected_index
                                .unwrap_or(0)
                                .min(current.tree_mode.nodes.len() - 1),
                        )
                    });
            }
        }

        CommitsBackendEvent::ViewStateUpdated {
            view: current,
            dirty: None,
        }
    }

    pub fn selected_diff_request(view: &CommitsPanelViewState) -> CommitsPanelDiffRequest {
        if view.tree_mode.active {
            let Some(oid) = view.tree_mode.selected_source.clone() else {
                return CommitsPanelDiffRequest::None;
            };
            let path = view
                .selected_index
                .and_then(|idx| view.tree_mode.nodes.get(idx))
                .map(|node| node.path.clone());
            return CommitsPanelDiffRequest::Commit { oid, path };
        }

        view.selected_index
            .and_then(|idx| view.items.get(idx))
            .map(|commit| CommitsPanelDiffRequest::Commit {
                oid: commit.oid.clone(),
                path: None,
            })
            .unwrap_or(CommitsPanelDiffRequest::None)
    }

    pub fn load_ahead_limit(
        view: &CommitsPanelViewState,
        active_panel: SidePanel,
        has_commits_task: bool,
        dirty: bool,
        current_limit: usize,
        threshold: usize,
        step: usize,
    ) -> Option<usize> {
        if active_panel != SidePanel::Commits
            || view.tree_mode.active
            || has_commits_task
            || dirty
            || view.items.is_empty()
        {
            return None;
        }
        let selected = view.selected_index?;
        if selected + threshold < view.items.len() {
            return None;
        }
        Some(current_limit.saturating_add(step))
    }
}

fn collect_dirs_from_entries(entries: &[FileEntry]) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for entry in entries {
        let mut path = entry.path.as_path();
        while let Some(parent) = path.parent() {
            if parent == std::path::Path::new("") {
                break;
            }
            dirs.insert(parent.to_path_buf());
            path = parent;
        }
    }
    dirs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::SidePanel;
    use crate::git::{CommitInfo, CommitSyncState, FileEntry, FileStatus, GraphCell};
    use pretty_assertions::assert_eq;

    fn commit(oid: &str, parents: &[&str]) -> CommitInfo {
        CommitInfo {
            oid: oid.to_string(),
            message: format!("commit {}", oid),
            author: "tester".to_string(),
            graph: vec![GraphCell {
                text: "*".to_string(),
                lane: 0,
                pipe_oid: Some(oid.to_string()),
                pipe_oids: vec![oid.to_string()],
            }],
            time: "2026-04-11 00:00".to_string(),
            parent_count: parents.len(),
            sync_state: CommitSyncState::DefaultBranch,
            parent_oids: parents.iter().map(|parent| (*parent).to_string()).collect(),
        }
    }

    #[test]
    fn fast_load_result_replaces_items_and_keeps_commits_dirty() {
        let event = CommitsBackend::apply_loaded(
            CommitsPanelViewState::default(),
            vec![commit("abc123", &[])],
            CommitsLoadMode::Fast,
        );

        match event {
            CommitsBackendEvent::ViewStateUpdated { view, dirty } => {
                assert_eq!(view.items.len(), 1);
                assert_eq!(dirty, Some(true));
            }
        }
    }

    #[test]
    fn highlight_derivation_preserves_existing_visible_graph_behavior() {
        let mut view = CommitsPanelViewState {
            selected_index: Some(1),
            items: vec![
                commit("newer", &["selected"]),
                commit("selected", &["older"]),
                commit("older", &[]),
            ],
            ..Default::default()
        };

        let event = CommitsBackend::recompute_highlight(view.clone(), SidePanel::Commits);

        match event {
            CommitsBackendEvent::ViewStateUpdated { view: next, dirty } => {
                assert_eq!(dirty, None);
                assert!(next.highlighted_oids.contains("newer"));
                assert!(next.highlighted_oids.contains("selected"));
                assert!(next.highlighted_oids.contains("older"));
            }
        }

        view.tree_mode.active = true;
        let event = CommitsBackend::recompute_highlight(view, SidePanel::Commits);
        match event {
            CommitsBackendEvent::ViewStateUpdated { view: next, .. } => {
                assert!(next.highlighted_oids.is_empty());
            }
        }
    }

    #[test]
    fn close_tree_restores_source_commit_selection() {
        let view = CommitsPanelViewState {
            selected_index: Some(0),
            items: vec![commit("source", &[]), commit("other", &[])],
            tree_mode: CommitsTreeViewState {
                active: true,
                selected_source: Some("source".to_string()),
                nodes: vec![],
                files: vec![],
                expanded_dirs: Default::default(),
            },
            ..Default::default()
        };

        let event = CommitsBackend::close_tree(view);

        match event {
            CommitsBackendEvent::ViewStateUpdated { view: next, dirty } => {
                assert_eq!(dirty, None);
                assert!(!next.tree_mode.active);
                assert_eq!(next.tree_mode.selected_source, None);
                assert_eq!(next.selected_index, Some(0));
            }
        }
    }

    #[test]
    fn open_tree_sets_source_and_tree_selection_from_commit_files() {
        let view = CommitsPanelViewState {
            selected_index: Some(0),
            items: vec![commit("source", &[])],
            ..Default::default()
        };

        let event = CommitsBackend::open_tree(
            view,
            "source".to_string(),
            vec![FileEntry {
                path: "src/main.rs".into(),
                status: FileStatus::Modified,
            }],
        );

        match event {
            CommitsBackendEvent::ViewStateUpdated { view: next, dirty } => {
                assert_eq!(dirty, None);
                assert!(next.tree_mode.active);
                assert_eq!(next.tree_mode.selected_source.as_deref(), Some("source"));
                assert_eq!(next.selected_index, Some(0));
                assert!(!next.tree_mode.nodes.is_empty());
            }
        }
    }

    #[test]
    fn toggle_tree_dir_rebuilds_nodes_without_leaving_backend_boundary() {
        let event = CommitsBackend::open_tree(
            CommitsPanelViewState {
                selected_index: Some(0),
                items: vec![commit("source", &[])],
                ..Default::default()
            },
            "source".to_string(),
            vec![FileEntry {
                path: "src/main.rs".into(),
                status: FileStatus::Modified,
            }],
        );
        let CommitsBackendEvent::ViewStateUpdated { view, .. } = event;

        let event = CommitsBackend::toggle_tree_dir(view);

        match event {
            CommitsBackendEvent::ViewStateUpdated { view: next, dirty } => {
                assert_eq!(dirty, None);
                assert_eq!(next.selected_index, Some(0));
                assert_eq!(next.tree_mode.nodes.len(), 1);
                assert!(!next.tree_mode.nodes[0].is_expanded);
            }
        }
    }

    #[test]
    fn selected_diff_request_uses_tree_source_and_selected_path() {
        let event = CommitsBackend::open_tree(
            CommitsPanelViewState {
                selected_index: Some(0),
                items: vec![commit("source", &[])],
                ..Default::default()
            },
            "source".to_string(),
            vec![FileEntry {
                path: "src/main.rs".into(),
                status: FileStatus::Modified,
            }],
        );
        let CommitsBackendEvent::ViewStateUpdated { view, .. } = event;

        assert_eq!(
            CommitsBackend::selected_diff_request(&view),
            CommitsPanelDiffRequest::Commit {
                oid: "source".to_string(),
                path: Some("src".into()),
            }
        );
    }

    #[test]
    fn load_ahead_limit_requests_next_step_near_end_of_visible_commits() {
        let view = CommitsPanelViewState {
            selected_index: Some(1),
            items: vec![commit("a", &[]), commit("b", &[])],
            ..Default::default()
        };

        assert_eq!(
            CommitsBackend::load_ahead_limit(&view, SidePanel::Commits, false, false, 100, 1, 50),
            Some(150)
        );
    }
}
