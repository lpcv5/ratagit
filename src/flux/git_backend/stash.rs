use crate::flux::action::{Action, DomainAction};
use crate::git::{FileEntry, StashInfo};
use crate::ui::widgets::file_tree::{FileTree, FileTreeNode};
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
pub struct StashTreeViewState {
    pub active: bool,
    pub selected_source: Option<usize>,
    pub nodes: Vec<FileTreeNode>,
    pub files: Vec<FileEntry>,
    pub expanded_dirs: HashSet<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct StashPanelViewState {
    pub selected_index: Option<usize>,
    pub items: Vec<StashInfo>,
    pub tree_mode: StashTreeViewState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StashPanelDiffRequest {
    None,
    Stash { index: usize, path: Option<PathBuf> },
}

#[derive(Debug, Clone)]
pub enum StashBackendCommand {
    ApplyLoaded {
        items: Vec<StashInfo>,
    },
    OpenTreeOrToggleDir,
    OpenTree {
        index: usize,
        files: Vec<FileEntry>,
    },
    CloseTree,
    Push {
        message: String,
        paths: Vec<PathBuf>,
    },
    Apply(usize),
    Pop(usize),
    Drop(usize),
}

#[derive(Debug, Clone)]
pub enum StashBackendEvent {
    ViewStateUpdated(StashPanelViewState),
    PushFinished {
        message: String,
        result: Result<usize, String>,
    },
    ApplyFinished {
        index: usize,
        result: Result<(), String>,
    },
    PopFinished {
        index: usize,
        result: Result<(), String>,
    },
    DropFinished {
        index: usize,
        result: Result<(), String>,
    },
}

impl StashBackendEvent {
    pub fn into_action(self) -> Option<Action> {
        match self {
            StashBackendEvent::ViewStateUpdated(_) => None,
            StashBackendEvent::PushFinished { message, result } => {
                Some(Action::Domain(DomainAction::StashPushFinished {
                    message,
                    result,
                }))
            }
            StashBackendEvent::ApplyFinished { index, result } => {
                Some(Action::Domain(DomainAction::StashApplyFinished {
                    index,
                    result,
                }))
            }
            StashBackendEvent::PopFinished { index, result } => {
                Some(Action::Domain(DomainAction::StashPopFinished {
                    index,
                    result,
                }))
            }
            StashBackendEvent::DropFinished { index, result } => {
                Some(Action::Domain(DomainAction::StashDropFinished {
                    index,
                    result,
                }))
            }
        }
    }
}

pub struct StashBackend;

impl StashBackend {
    pub fn apply_loaded(current: StashPanelViewState, items: Vec<StashInfo>) -> StashBackendEvent {
        let selected_source = selected_stash_index(&current);
        let mut next = StashPanelViewState { items, ..current };
        if next.tree_mode.active
            && next
                .tree_mode
                .selected_source
                .is_some_and(|index| stash_position_by_index(&next.items, index).is_none())
        {
            next = Self::close_tree_view(next);
            return StashBackendEvent::ViewStateUpdated(next);
        }
        if !next.tree_mode.active {
            next.selected_index = selected_source
                .and_then(|index| stash_position_by_index(&next.items, index))
                .or(next.selected_index);
        }
        clamp_selection(&mut next);
        StashBackendEvent::ViewStateUpdated(next)
    }

    pub fn open_tree(
        mut current: StashPanelViewState,
        index: usize,
        files: Vec<FileEntry>,
    ) -> StashBackendEvent {
        let expanded_dirs = collect_dirs_from_entries(&files);
        let nodes = FileTree::from_git_status_with_expanded(&files, &[], &[], &expanded_dirs);
        current.tree_mode.active = true;
        current.tree_mode.selected_source = Some(index);
        current.tree_mode.files = files;
        current.tree_mode.expanded_dirs = expanded_dirs;
        current.tree_mode.nodes = nodes;
        current.selected_index = if current.tree_mode.nodes.is_empty() {
            None
        } else {
            Some(0)
        };
        StashBackendEvent::ViewStateUpdated(current)
    }

    pub fn toggle_tree_dir(mut current: StashPanelViewState) -> StashBackendEvent {
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
            current.selected_index = if current.tree_mode.nodes.is_empty() {
                None
            } else {
                current
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
                    })
            };
        }
        StashBackendEvent::ViewStateUpdated(current)
    }

    pub fn close_tree(mut current: StashPanelViewState) -> StashBackendEvent {
        current = Self::close_tree_view(current);
        StashBackendEvent::ViewStateUpdated(current)
    }

    pub fn selected_diff_request(view: &StashPanelViewState) -> StashPanelDiffRequest {
        let Some(index) = selected_stash_index(view) else {
            return StashPanelDiffRequest::None;
        };
        let path = if view.tree_mode.active {
            view.selected_index
                .and_then(|idx| view.tree_mode.nodes.get(idx))
                .map(|node| node.path.clone())
        } else {
            None
        };
        StashPanelDiffRequest::Stash { index, path }
    }

    pub fn selected_stash_index(view: &StashPanelViewState) -> Option<usize> {
        selected_stash_index(view)
    }

    fn close_tree_view(mut current: StashPanelViewState) -> StashPanelViewState {
        if !current.tree_mode.active {
            return current;
        }
        let selected_source_index = current
            .tree_mode
            .selected_source
            .and_then(|index| stash_position_by_index(&current.items, index));
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
        current
    }
}

fn selected_stash_index(view: &StashPanelViewState) -> Option<usize> {
    if view.tree_mode.active {
        return view.tree_mode.selected_source;
    }
    view.selected_index
        .and_then(|idx| view.items.get(idx))
        .map(|stash| stash.index)
}

fn stash_position_by_index(items: &[StashInfo], index: usize) -> Option<usize> {
    items.iter().position(|stash| stash.index == index)
}

fn clamp_selection(view: &mut StashPanelViewState) {
    if view.items.is_empty() {
        if !view.tree_mode.active {
            view.selected_index = None;
        }
        return;
    }
    if !view.tree_mode.active {
        view.selected_index = Some(view.selected_index.unwrap_or(0).min(view.items.len() - 1));
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
    use crate::git::{FileStatus, StashInfo};
    use crate::ui::widgets::file_tree::FileTreeNodeStatus;
    use pretty_assertions::assert_eq;

    fn stash(index: usize) -> StashInfo {
        StashInfo {
            index,
            message: format!("stash {}", index),
        }
    }

    #[test]
    fn close_tree_restores_source_stash_selection() {
        let event = StashBackend::close_tree(StashPanelViewState {
            selected_index: Some(0),
            items: vec![stash(0), stash(2)],
            tree_mode: StashTreeViewState {
                active: true,
                selected_source: Some(2),
                nodes: vec![],
                files: vec![],
                expanded_dirs: Default::default(),
            },
        });

        let StashBackendEvent::ViewStateUpdated(view) = event else {
            panic!("expected view update");
        };
        assert!(!view.tree_mode.active);
        assert_eq!(view.selected_index, Some(1));
    }

    #[test]
    fn selected_diff_request_for_list_mode_uses_selected_stash_index() {
        let view = StashPanelViewState {
            selected_index: Some(1),
            items: vec![stash(0), stash(3)],
            ..Default::default()
        };

        assert_eq!(
            StashBackend::selected_diff_request(&view),
            StashPanelDiffRequest::Stash {
                index: 3,
                path: None
            }
        );
    }

    #[test]
    fn selected_diff_request_for_tree_mode_uses_tree_source_and_path() {
        let view = StashPanelViewState {
            selected_index: Some(0),
            items: vec![stash(2)],
            tree_mode: StashTreeViewState {
                active: true,
                selected_source: Some(2),
                nodes: vec![FileTreeNode {
                    path: "src/main.rs".into(),
                    status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
                    depth: 0,
                    is_dir: false,
                    is_expanded: false,
                }],
                files: vec![],
                expanded_dirs: Default::default(),
            },
        };

        assert_eq!(
            StashBackend::selected_diff_request(&view),
            StashPanelDiffRequest::Stash {
                index: 2,
                path: Some("src/main.rs".into())
            }
        );
    }

    #[test]
    fn apply_loaded_keeps_tree_selection_when_source_stash_still_exists() {
        let event = StashBackend::apply_loaded(
            StashPanelViewState {
                selected_index: Some(1),
                items: vec![stash(2)],
                tree_mode: StashTreeViewState {
                    active: true,
                    selected_source: Some(2),
                    nodes: vec![
                        FileTreeNode {
                            path: "src".into(),
                            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
                            depth: 0,
                            is_dir: true,
                            is_expanded: true,
                        },
                        FileTreeNode {
                            path: "src/main.rs".into(),
                            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
                            depth: 1,
                            is_dir: false,
                            is_expanded: false,
                        },
                    ],
                    files: vec![],
                    expanded_dirs: Default::default(),
                },
            },
            vec![stash(2), stash(5)],
        );

        let StashBackendEvent::ViewStateUpdated(view) = event else {
            panic!("expected view update");
        };

        assert!(view.tree_mode.active);
        assert_eq!(view.tree_mode.selected_source, Some(2));
        assert_eq!(view.selected_index, Some(1));
    }
}
