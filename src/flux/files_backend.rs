use crate::flux::action::{Action, DomainAction};
use crate::git::{FileStatus, GitStatus};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum FilesPanelNodeStatus {
    Unstaged(FileStatus),
    Staged(FileStatus),
    Untracked,
    Directory,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilesPanelNode {
    pub path: PathBuf,
    pub status: FilesPanelNodeStatus,
    pub depth: usize,
    pub is_dir: bool,
    pub is_expanded: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FilesPanelSelectionState {
    pub selected_index: Option<usize>,
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FilesPanelViewState {
    pub expanded_dirs: HashSet<PathBuf>,
    pub selection: FilesPanelSelectionState,
    pub nodes: Vec<FilesPanelNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilesPanelDiffRequest {
    None,
    File { path: PathBuf, staged: bool },
    Directory { path: PathBuf },
}

#[derive(Debug, Clone)]
pub enum FilesBackendCommand {
    RefreshFromStatus {
        status: GitStatus,
        expanded_dirs: HashSet<PathBuf>,
        selection: FilesPanelSelectionState,
    },
    ToggleSelectedDir,
    CollapseAll,
    ExpandAll,
    ReloadDiff,
    StagePath(PathBuf),
    UnstagePath(PathBuf),
    DiscardPaths(Vec<PathBuf>),
}

#[derive(Debug, Clone)]
pub enum FilesBackendEvent {
    ViewStateUpdated(FilesPanelViewState),
    StageFinished {
        path: PathBuf,
        result: Result<(), String>,
    },
    UnstageFinished {
        path: PathBuf,
        result: Result<(), String>,
    },
    DiscardFinished {
        paths: Vec<PathBuf>,
        result: Result<(), String>,
    },
}

impl FilesBackendEvent {
    pub fn into_action(self) -> Option<Action> {
        match self {
            FilesBackendEvent::ViewStateUpdated(_) => None,
            FilesBackendEvent::StageFinished { path, result } => {
                Some(Action::Domain(DomainAction::StageFileFinished {
                    path,
                    result,
                }))
            }
            FilesBackendEvent::UnstageFinished { path, result } => {
                Some(Action::Domain(DomainAction::UnstageFileFinished {
                    path,
                    result,
                }))
            }
            FilesBackendEvent::DiscardFinished { paths, result } => {
                Some(Action::Domain(DomainAction::DiscardPathsFinished {
                    paths,
                    result,
                }))
            }
        }
    }

    #[cfg(test)]
    fn expect_view_state(self) -> FilesPanelViewState {
        match self {
            FilesBackendEvent::ViewStateUpdated(view) => view,
            _ => panic!("expected a files-panel view-state event"),
        }
    }
}

pub struct FilesBackend;

impl FilesBackend {
    pub fn handle_command(command: FilesBackendCommand) -> FilesBackendEvent {
        match command {
            FilesBackendCommand::RefreshFromStatus {
                status,
                expanded_dirs,
                selection,
            } => FilesBackendEvent::ViewStateUpdated(Self::build_view_state(
                &status,
                expanded_dirs,
                selection,
            )),
            FilesBackendCommand::ToggleSelectedDir
            | FilesBackendCommand::CollapseAll
            | FilesBackendCommand::ExpandAll
            | FilesBackendCommand::ReloadDiff
            | FilesBackendCommand::StagePath(_)
            | FilesBackendCommand::UnstagePath(_)
            | FilesBackendCommand::DiscardPaths(_) => {
                panic!("runtime-only backend command should not be handled in pure mode")
            }
        }
    }

    pub fn build_view_state(
        status: &GitStatus,
        expanded_dirs: HashSet<PathBuf>,
        selection: FilesPanelSelectionState,
    ) -> FilesPanelViewState {
        let files = Self::collect_files(status);
        let all_dirs = collect_dirs(&files);
        let nodes = build_subtree(Path::new(""), &files, &all_dirs, &expanded_dirs, 0);
        let mut view = FilesPanelViewState {
            expanded_dirs,
            selection,
            nodes,
        };
        Self::clamp_selection(&mut view);
        view
    }

    pub fn selected_diff_request(view: &FilesPanelViewState) -> FilesPanelDiffRequest {
        let Some(index) = view.selection.selected_index else {
            return FilesPanelDiffRequest::None;
        };
        let Some(node) = view.nodes.get(index) else {
            return FilesPanelDiffRequest::None;
        };
        if node.is_dir {
            return FilesPanelDiffRequest::Directory {
                path: node.path.clone(),
            };
        }
        let staged = matches!(node.status, FilesPanelNodeStatus::Staged(_));
        FilesPanelDiffRequest::File {
            path: node.path.clone(),
            staged,
        }
    }

    pub fn all_dirs(status: &GitStatus) -> HashSet<PathBuf> {
        let files = Self::collect_files(status);
        collect_dirs(&files)
    }

    pub fn toggle_selected_dir(
        status: &GitStatus,
        current: FilesPanelViewState,
    ) -> FilesPanelViewState {
        let mut expanded_dirs = current.expanded_dirs.clone();
        if let Some(path) = current
            .selection
            .selected_index
            .and_then(|index| current.nodes.get(index))
            .filter(|node| node.is_dir)
            .map(|node| node.path.clone())
        {
            if !expanded_dirs.insert(path.clone()) {
                expanded_dirs.remove(&path);
            }
        }
        Self::build_view_state(status, expanded_dirs, current.selection)
    }

    pub fn collapse_all(status: &GitStatus, current: FilesPanelViewState) -> FilesPanelViewState {
        Self::build_view_state(status, HashSet::new(), current.selection)
    }

    pub fn expand_all(status: &GitStatus, current: FilesPanelViewState) -> FilesPanelViewState {
        Self::build_view_state(status, Self::all_dirs(status), current.selection)
    }

    fn clamp_selection(view: &mut FilesPanelViewState) {
        let count = view.nodes.len();
        if count == 0 {
            view.selection.selected_index = None;
            view.selection.visual_anchor = None;
            return;
        }
        view.selection.selected_index =
            Some(view.selection.selected_index.unwrap_or(0).min(count - 1));
        if let Some(anchor) = view.selection.visual_anchor {
            view.selection.visual_anchor = Some(anchor.min(count - 1));
        }
    }

    fn collect_files(status: &GitStatus) -> Vec<(PathBuf, FilesPanelNodeStatus)> {
        let mut files = Vec::new();
        for entry in &status.unstaged {
            files.push((
                entry.path.clone(),
                FilesPanelNodeStatus::Unstaged(entry.status.clone()),
            ));
        }
        for entry in &status.untracked {
            files.push((entry.path.clone(), FilesPanelNodeStatus::Untracked));
        }
        for entry in &status.staged {
            files.push((
                entry.path.clone(),
                FilesPanelNodeStatus::Staged(entry.status.clone()),
            ));
        }
        files
    }
}

fn collect_dirs(files: &[(PathBuf, FilesPanelNodeStatus)]) -> HashSet<PathBuf> {
    let mut dirs = HashSet::new();
    for (path, _) in files {
        let mut current = path.as_path();
        while let Some(parent) = current.parent() {
            if parent == Path::new("") {
                break;
            }
            dirs.insert(parent.to_path_buf());
            current = parent;
        }
    }
    dirs
}

fn build_subtree(
    dir: &Path,
    files: &[(PathBuf, FilesPanelNodeStatus)],
    all_dirs: &HashSet<PathBuf>,
    expanded: &HashSet<PathBuf>,
    depth: usize,
) -> Vec<FilesPanelNode> {
    let mut nodes = Vec::new();
    let mut child_dirs: Vec<PathBuf> = all_dirs
        .iter()
        .filter(|candidate| candidate.parent().unwrap_or(Path::new("")) == dir)
        .cloned()
        .collect();
    child_dirs.sort();

    for child_dir in child_dirs {
        let is_expanded = expanded.contains(&child_dir);
        nodes.push(FilesPanelNode {
            path: child_dir.clone(),
            status: FilesPanelNodeStatus::Directory,
            depth,
            is_dir: true,
            is_expanded,
        });
        if is_expanded {
            nodes.extend(build_subtree(
                &child_dir,
                files,
                all_dirs,
                expanded,
                depth + 1,
            ));
        }
    }

    let mut child_files = files
        .iter()
        .filter(|(path, _)| path.parent().unwrap_or(Path::new("")) == dir)
        .cloned()
        .collect::<Vec<_>>();
    child_files.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (path, status) in child_files {
        nodes.push(FilesPanelNode {
            path,
            status,
            depth,
            is_dir: false,
            is_expanded: false,
        });
    }

    nodes
}

#[cfg(test)]
mod tests {
    use super::{
        FilesBackend, FilesBackendCommand, FilesBackendEvent, FilesPanelDiffRequest,
        FilesPanelNode, FilesPanelNodeStatus, FilesPanelSelectionState, FilesPanelViewState,
    };
    use crate::flux::action::{Action, DomainAction};
    use crate::git::{FileEntry, FileStatus, GitStatus};
    use pretty_assertions::assert_eq;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn discard_event_converts_into_domain_action() {
        let action = FilesBackendEvent::DiscardFinished {
            paths: vec![PathBuf::from("foo.txt")],
            result: Ok(()),
        }
        .into_action();

        assert!(matches!(
            action,
            Some(Action::Domain(DomainAction::DiscardPathsFinished { paths, result }))
                if paths == vec![PathBuf::from("foo.txt")] && result == Ok(())
        ));
    }

    #[test]
    fn files_panel_view_state_carries_selection_without_widget_state() {
        let state = FilesPanelViewState {
            expanded_dirs: HashSet::from([PathBuf::from("src")]),
            selection: FilesPanelSelectionState {
                selected_index: Some(2),
                visual_mode: true,
                visual_anchor: Some(1),
            },
            nodes: vec![FilesPanelNode {
                path: PathBuf::from("src/main.rs"),
                status: FilesPanelNodeStatus::Unstaged(FileStatus::Modified),
                depth: 1,
                is_dir: false,
                is_expanded: false,
            }],
        };

        assert_eq!(state.selection.selected_index, Some(2));
        assert!(state.selection.visual_mode);
        assert_eq!(state.selection.visual_anchor, Some(1));
        assert!(state.expanded_dirs.contains(&PathBuf::from("src")));
    }

    #[test]
    fn build_view_state_preserves_selection_and_expanded_dirs() {
        let status = GitStatus {
            unstaged: vec![FileEntry {
                path: PathBuf::from("src/main.rs"),
                status: FileStatus::Modified,
            }],
            staged: vec![],
            untracked: vec![],
        };
        let expanded_dirs = HashSet::from([PathBuf::from("src")]);
        let selection = FilesPanelSelectionState {
            selected_index: Some(1),
            visual_mode: false,
            visual_anchor: None,
        };

        let view = FilesBackend::handle_command(FilesBackendCommand::RefreshFromStatus {
            status,
            expanded_dirs: expanded_dirs.clone(),
            selection,
        })
        .expect_view_state();

        assert_eq!(view.expanded_dirs, expanded_dirs);
        assert_eq!(view.selection.selected_index, Some(1));
        assert_eq!(view.nodes.len(), 2);
        assert_eq!(view.nodes[0].status, FilesPanelNodeStatus::Directory);
    }

    #[test]
    fn selected_diff_request_for_file_keeps_path_and_staged_flag() {
        let status = GitStatus {
            unstaged: vec![],
            staged: vec![FileEntry {
                path: PathBuf::from("src/lib.rs"),
                status: FileStatus::Modified,
            }],
            untracked: vec![],
        };
        let expanded_dirs = HashSet::from([PathBuf::from("src")]);
        let selection = FilesPanelSelectionState {
            selected_index: Some(1),
            visual_mode: false,
            visual_anchor: None,
        };

        let view = FilesBackend::handle_command(FilesBackendCommand::RefreshFromStatus {
            status,
            expanded_dirs,
            selection,
        })
        .expect_view_state();

        assert_eq!(
            FilesBackend::selected_diff_request(&view),
            FilesPanelDiffRequest::File {
                path: PathBuf::from("src/lib.rs"),
                staged: true,
            }
        );
    }

    #[test]
    fn collapse_all_rebuilds_without_losing_current_selection() {
        let status = GitStatus {
            unstaged: vec![
                FileEntry {
                    path: PathBuf::from("src/main.rs"),
                    status: FileStatus::Modified,
                },
                FileEntry {
                    path: PathBuf::from("src/lib.rs"),
                    status: FileStatus::Modified,
                },
            ],
            staged: vec![],
            untracked: vec![],
        };
        let expanded_dirs = HashSet::from([PathBuf::from("src")]);
        let selection = FilesPanelSelectionState {
            selected_index: Some(2),
            visual_mode: false,
            visual_anchor: None,
        };

        let initial = FilesBackend::handle_command(FilesBackendCommand::RefreshFromStatus {
            status: status.clone(),
            expanded_dirs,
            selection,
        })
        .expect_view_state();

        let collapsed = FilesBackend::collapse_all(&status, initial);

        assert!(collapsed.expanded_dirs.is_empty());
        assert_eq!(collapsed.selection.selected_index, Some(0));
        assert_eq!(collapsed.nodes.len(), 1);
    }
}
