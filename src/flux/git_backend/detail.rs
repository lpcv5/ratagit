use crate::app::SidePanel;
use crate::app::{BranchesPanelState, CommitsPanelState, FilesPanelState, StashPanelState};
use crate::app::{
    branch_panel_adapter, commits_panel_adapter, files_panel_adapter, stash_panel_adapter,
};
use crate::app::diff_cache::DiffCacheKey;
use crate::flux::branch_backend::BranchPanelViewState;
use crate::flux::commits_backend::{CommitsBackend, CommitsPanelDiffRequest, CommitsPanelViewState};
use crate::flux::files_backend::{FilesBackend, FilesPanelDiffRequest, FilesPanelViewState};
use crate::flux::git_backend::stash::{StashBackend, StashPanelDiffRequest, StashPanelViewState};
use crate::git::DiffLine;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum DetailRequest {
    #[default]
    None,
    BranchLog { name: String },
    File {
        path: PathBuf,
        is_staged: bool,
        is_untracked: bool,
    },
    Directory { path: PathBuf, files_hash: u64 },
    Commit { oid: String, path: Option<PathBuf> },
    Stash { index: usize, path: Option<PathBuf> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailPanelMode {
    Diff,
    Log,
}

#[derive(Debug, Clone, Default)]
pub struct DetailPanelState {
    pub request: DetailRequest,
    pub lines: Vec<DiffLine>,
    pub is_loading: bool,
}

#[derive(Debug, Clone)]
pub struct DetailPanelViewState {
    #[allow(dead_code)]
    pub request: DetailRequest,
    pub mode: DetailPanelMode,
    pub lines: Vec<DiffLine>,
    pub scroll: usize,
    pub is_loading: bool,
    pub panel_title: String,
    pub empty_message: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DetailPanelCommand {
    Load(DetailRequest),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum DetailPanelEvent {
    ViewStateUpdated(DetailPanelState),
}

pub struct DetailBackend;

impl DetailBackend {
    pub fn request_from_shell_panels(
        active_panel: SidePanel,
        files: &FilesPanelState,
        branches: &BranchesPanelState,
        commits: &CommitsPanelState,
        stash: &StashPanelState,
    ) -> DetailRequest {
        let files_view = files_panel_adapter::view_state_from_shell(files);
        let branches_view = branch_panel_adapter::view_state_from_shell(branches);
        let commits_view = commits_panel_adapter::view_state_from_shell(commits);
        let stash_view = stash_panel_adapter::view_state_from_shell(stash);
        Self::request_from_views(active_panel, &files_view, &branches_view, &commits_view, &stash_view)
    }

    pub fn request_from_views(
        active_panel: SidePanel,
        files: &FilesPanelViewState,
        branches: &BranchPanelViewState,
        commits: &CommitsPanelViewState,
        stash: &StashPanelViewState,
    ) -> DetailRequest {
        match active_panel {
            SidePanel::Files => match FilesBackend::selected_diff_request(files) {
                FilesPanelDiffRequest::None => DetailRequest::None,
                FilesPanelDiffRequest::Directory { path } => DetailRequest::Directory {
                    files_hash: directory_hash(files, &path),
                    path,
                },
                FilesPanelDiffRequest::File { path, staged } => {
                    let is_untracked = files
                        .selection
                        .selected_index
                        .and_then(|index| files.nodes.get(index))
                        .is_some_and(|node| {
                            matches!(node.status, crate::flux::files_backend::FilesPanelNodeStatus::Untracked)
                        });
                    DetailRequest::File {
                        path,
                        is_staged: staged,
                        is_untracked,
                    }
                }
            },
            SidePanel::LocalBranches => {
                if branches.commits_subview.active {
                    branches
                        .commits_subview
                        .selected_index
                        .and_then(|index| branches.commits_subview.items.get(index))
                        .map(|commit| DetailRequest::Commit {
                            oid: commit.oid.clone(),
                            path: None,
                        })
                        .unwrap_or(DetailRequest::None)
                } else {
                    branches
                        .selection
                        .selected_index
                        .and_then(|index| branches.items.get(index))
                        .map(|branch| DetailRequest::BranchLog {
                            name: branch.name.clone(),
                        })
                        .unwrap_or(DetailRequest::None)
                }
            }
            SidePanel::Commits => match CommitsBackend::selected_diff_request(commits) {
                CommitsPanelDiffRequest::None => DetailRequest::None,
                CommitsPanelDiffRequest::Commit { oid, path } => DetailRequest::Commit { oid, path },
            },
            SidePanel::Stash => match StashBackend::selected_diff_request(stash) {
                StashPanelDiffRequest::None => DetailRequest::None,
                StashPanelDiffRequest::Stash { index, path } => DetailRequest::Stash { index, path },
            },
        }
    }

    pub fn build_view_state(state: &DetailPanelState, scroll: usize) -> DetailPanelViewState {
        DetailPanelViewState {
            request: state.request.clone(),
            mode: Self::mode_for_request(&state.request),
            lines: state.lines.clone(),
            scroll,
            is_loading: state.is_loading,
            panel_title: Self::panel_title(&state.request),
            empty_message: Self::empty_message(&state.request),
        }
    }

    pub fn mode_for_request(request: &DetailRequest) -> DetailPanelMode {
        match request {
            DetailRequest::BranchLog { .. } => DetailPanelMode::Log,
            _ => DetailPanelMode::Diff,
        }
    }

    pub fn panel_title(request: &DetailRequest) -> String {
        match Self::mode_for_request(request) {
            DetailPanelMode::Log => "Log".to_string(),
            DetailPanelMode::Diff => "Diff".to_string(),
        }
    }

    pub fn empty_message(request: &DetailRequest) -> String {
        match request {
            DetailRequest::None | DetailRequest::Directory { .. } => "Select a file to view diff".to_string(),
            DetailRequest::BranchLog { .. } => "Select a branch to view log".to_string(),
            DetailRequest::File { .. } => "Select a file to view diff".to_string(),
            DetailRequest::Commit { path: Some(_), .. } => "Select a commit/file to view diff".to_string(),
            DetailRequest::Commit { path: None, .. } => "Select a commit/file to view diff".to_string(),
            DetailRequest::Stash { .. } => "Select a stash entry/file to view diff".to_string(),
        }
    }

    pub fn cache_key(request: &DetailRequest) -> DiffCacheKey {
        match request {
            DetailRequest::None => DiffCacheKey::None,
            DetailRequest::BranchLog { name } => DiffCacheKey::Branch {
                name: name.clone(),
                limit: 100,
            },
            DetailRequest::File {
                path,
                is_staged,
                ..
            } => DiffCacheKey::File {
                path: path.clone(),
                is_staged: *is_staged,
            },
            DetailRequest::Directory { path, files_hash } => DiffCacheKey::Directory {
                path: path.clone(),
                files_hash: *files_hash,
            },
            DetailRequest::Commit { oid, path } => DiffCacheKey::Commit {
                oid: oid.clone(),
                path: path.clone(),
            },
            DetailRequest::Stash { index, path } => DiffCacheKey::Stash {
                index: *index,
                path: path.clone(),
            },
        }
    }

    pub fn cache_key_task_target(key: &DiffCacheKey) -> String {
        match key {
            DiffCacheKey::None => "none".to_string(),
            DiffCacheKey::File { path, is_staged } => format!("file:{}:{}", path.display(), is_staged),
            DiffCacheKey::Branch { name, limit } => format!("branch:{}:{}", name, limit),
            DiffCacheKey::Directory { path, files_hash } => {
                format!("dir:{}:{}", path.display(), files_hash)
            }
            DiffCacheKey::Commit { oid, path } => match path {
                Some(path) => format!("commit:{}:{}", oid, path.display()),
                None => format!("commit:{}:*", oid),
            },
            DiffCacheKey::Stash { index, path } => match path {
                Some(path) => format!("stash:{}:{}", index, path.display()),
                None => format!("stash:{}:*", index),
            },
        }
    }
}

fn directory_hash(view: &FilesPanelViewState, path: &std::path::Path) -> u64 {
    view
        .nodes
        .iter()
        .filter(|node| node.path.starts_with(path))
        .map(|node| node.path.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("|")
        .bytes()
        .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{BranchInfo, CommitInfo, CommitSyncState, DiffLineKind, FileStatus, GraphCell};
    use crate::ui::widgets::file_tree::FileTreeNode;

    fn commit(oid: &str) -> CommitInfo {
        CommitInfo {
            oid: oid.to_string(),
            message: format!("commit {}", oid),
            author: "tester".to_string(),
            graph: vec![GraphCell {
                text: "*".to_string(),
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

    #[test]
    fn branch_request_selects_log_mode() {
        let request = DetailRequest::BranchLog {
            name: "main".to_string(),
        };

        let view = DetailBackend::build_view_state(
            &DetailPanelState {
                request: request.clone(),
                lines: vec![],
                is_loading: false,
            },
            0,
        );

        assert_eq!(view.mode, DetailPanelMode::Log);
        assert_eq!(view.panel_title, "Log");
        assert_eq!(view.empty_message, "Select a branch to view log");
    }

    #[test]
    fn request_from_branch_commits_subview_prefers_selected_commit() {
        let mut branches = BranchesPanelState {
            items: vec![BranchInfo {
                name: "main".to_string(),
                is_current: true,
            }],
            ..Default::default()
        };
        branches.commits_subview_active = true;
        branches.commits_subview.items = vec![commit("abc123")];
        branches.commits_subview.panel.list_state.select(Some(0));

        let request = DetailBackend::request_from_shell_panels(
            SidePanel::LocalBranches,
            &FilesPanelState::default(),
            &branches,
            &CommitsPanelState::default(),
            &StashPanelState::default(),
        );

        assert_eq!(
            request,
            DetailRequest::Commit {
                oid: "abc123".to_string(),
                path: None,
            }
        );
    }

    #[test]
    fn request_from_files_directory_uses_directory_hash() {
        let mut files = FilesPanelState {
            tree_nodes: vec![FileTreeNode {
                path: "src".into(),
                status: crate::ui::widgets::file_tree::FileTreeNodeStatus::Directory,
                depth: 0,
                is_dir: true,
                is_expanded: true,
            }, FileTreeNode {
                path: "src/main.rs".into(),
                status: crate::ui::widgets::file_tree::FileTreeNodeStatus::Unstaged(FileStatus::Modified),
                depth: 1,
                is_dir: false,
                is_expanded: false,
            }],
            ..Default::default()
        };
        files.panel.list_state.select(Some(0));

        let request = DetailBackend::request_from_shell_panels(
            SidePanel::Files,
            &files,
            &BranchesPanelState::default(),
            &CommitsPanelState::default(),
            &StashPanelState::default(),
        );

        assert!(matches!(request, DetailRequest::Directory { .. }));
    }

    #[test]
    fn view_state_keeps_loaded_lines() {
        let view = DetailBackend::build_view_state(
            &DetailPanelState {
                request: DetailRequest::Commit {
                    oid: "abc123".to_string(),
                    path: None,
                },
                lines: vec![DiffLine {
                    kind: DiffLineKind::Header,
                    content: "diff".to_string(),
                }],
                is_loading: false,
            },
            3,
        );

        assert_eq!(view.lines.len(), 1);
        assert_eq!(view.scroll, 3);
        assert_eq!(view.panel_title, "Diff");
    }
}
