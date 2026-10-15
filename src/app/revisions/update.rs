use super::{tree, RevisionState, TreeModeState};
use crate::app::SidePanel;
use crate::git::{BranchInfo, CommitInfo, FileEntry, StashInfo};
use crate::shared::Shared;

const COMMITS_LOAD_STEP: usize = 40;
const COMMITS_LOAD_AHEAD_THRESHOLD: usize = 8;

pub enum RevisionAction {
    OpenBranchCommits {
        limit: usize,
    },
    CloseBranchCommits,
    OpenCommitTree,
    CloseCommitTree,
    OpenStashTree,
    CloseStashTree,
    SelectIndex(usize),
    MarkCommitsDirty,
    TrackCommitsLoad(usize),
    ExtendCommits {
        loading: bool,
    },
    RecomputeHighlight,
    SetFetching(bool),
    ReplaceBranches(Shared<Vec<BranchInfo>>),
    ReplaceStashes(Shared<Vec<StashInfo>>),
    ReplaceCommits {
        items: Shared<Vec<CommitInfo>>,
        fast: bool,
    },
    BranchCommitsLoaded {
        branch: String,
        items: Shared<Vec<CommitInfo>>,
    },
    CommitFilesLoaded {
        oid: String,
        files: Shared<Vec<FileEntry>>,
    },
    StashFilesLoaded {
        index: usize,
        files: Shared<Vec<FileEntry>>,
    },
    BranchLoadFailed,
}

/// Owned read intentions. This feature has no repository or task-runtime handle.
#[derive(Debug, PartialEq, Eq)]
pub enum RevisionRead {
    Commits { limit: usize },
    BranchCommits { branch: String, limit: usize },
    CommitFiles { oid: String },
    StashFiles { index: usize },
}

#[derive(Debug, PartialEq, Eq)]
pub enum RevisionCancel {
    CommitFiles,
    StashFiles,
}

#[derive(Default)]
pub struct RevisionUpdate {
    pub read: Option<RevisionRead>,
    pub cancel: Option<RevisionCancel>,
    pub log: Option<(String, bool)>,
    pub accepted: bool,
}

impl RevisionState {
    pub fn update(&mut self, action: RevisionAction, active: SidePanel) -> RevisionUpdate {
        let mut output = RevisionUpdate::default();
        match action {
            RevisionAction::OpenBranchCommits { limit } if active == SidePanel::LocalBranches => {
                let Some(branch) = self
                    .selection(active)
                    .selected_branch_name()
                    .map(str::to_owned)
                else {
                    return output;
                };
                let state = &mut self.panels.branches;
                state.commits_subview_active = true;
                state.commits_subview_loading = true;
                state.commits_subview_source = Some(branch.clone());
                state.commits_subview.items = Default::default();
                state.commits_subview.dirty = false;
                state.commits_subview.highlighted_oids = Default::default();
                state.commits_subview.tree_mode = TreeModeState::default();
                state.commits_subview.panel.select(None);
                output.log = Some((format!("branch commits: {} (Esc to back)", branch), true));
                output.read = Some(RevisionRead::BranchCommits { branch, limit });
            }
            RevisionAction::CloseBranchCommits => {
                let state = &mut self.panels.branches;
                if !state.commits_subview_active {
                    return output;
                }
                let source = state.commits_subview_source.take();
                state.commits_subview_active = false;
                state.commits_subview_loading = false;
                state.commits_subview = Default::default();
                let selected = source
                    .and_then(|name| state.items.iter().position(|branch| branch.name == name));
                state.panel.select(if state.items.is_empty() {
                    None
                } else {
                    selected.or(Some(0))
                });
            }
            RevisionAction::OpenCommitTree if active == SidePanel::Commits => {
                if self.panels.commits.dirty {
                    output.read = Some(RevisionRead::Commits {
                        limit: self.commits_requested_limit,
                    });
                    return output;
                }
                if !self.panels.commits.tree_mode.active {
                    let Some(oid) = self
                        .selection(active)
                        .selected_commit_oid()
                        .map(str::to_owned)
                    else {
                        return output;
                    };
                    self.panels.commits.tree_mode.selected_source = Some(oid.clone());
                    output.read = Some(RevisionRead::CommitFiles { oid });
                } else {
                    let path = self
                        .selection(active)
                        .selected_revision_node()
                        .filter(|node| node.is_dir)
                        .map(|node| node.path.clone());
                    let state = &mut self.panels.commits;
                    tree::toggle_tree_dir(path, &mut state.tree_mode, &mut state.panel);
                }
            }
            RevisionAction::OpenStashTree if active == SidePanel::Stash => {
                if !self.panels.stash.tree_mode.active {
                    let Some(index) = self.selection(active).selected_stash_index() else {
                        return output;
                    };
                    self.panels.stash.tree_mode.selected_source = Some(index);
                    output.read = Some(RevisionRead::StashFiles { index });
                } else {
                    let path = self
                        .selection(active)
                        .selected_revision_node()
                        .filter(|node| node.is_dir)
                        .map(|node| node.path.clone());
                    let state = &mut self.panels.stash;
                    tree::toggle_tree_dir(path, &mut state.tree_mode, &mut state.panel);
                }
            }
            RevisionAction::CloseCommitTree => {
                output.cancel = Some(RevisionCancel::CommitFiles);
                let state = &mut self.panels.commits;
                let selected = state
                    .tree_mode
                    .selected_source
                    .as_ref()
                    .and_then(|oid| state.items.iter().position(|commit| commit.oid == *oid));
                tree::close_tree_mode(
                    &mut state.tree_mode,
                    &mut state.panel,
                    selected,
                    state.items.len(),
                );
                state.tree_mode.selected_source = None;
            }
            RevisionAction::CloseStashTree => {
                output.cancel = Some(RevisionCancel::StashFiles);
                let state = &mut self.panels.stash;
                let selected = state
                    .tree_mode
                    .selected_source
                    .and_then(|index| state.items.iter().position(|stash| stash.index == index));
                tree::close_tree_mode(
                    &mut state.tree_mode,
                    &mut state.panel,
                    selected,
                    state.items.len(),
                );
                state.tree_mode.selected_source = None;
            }
            RevisionAction::SelectIndex(index) => {
                let panel = match active {
                    SidePanel::LocalBranches if self.panels.branches.commits_subview_active => {
                        &mut self.panels.branches.commits_subview.panel
                    }
                    SidePanel::LocalBranches => &mut self.panels.branches.panel,
                    SidePanel::Commits => &mut self.panels.commits.panel,
                    SidePanel::Stash => &mut self.panels.stash.panel,
                    SidePanel::Files => return output,
                };
                panel.select(Some(index));
            }
            RevisionAction::MarkCommitsDirty => self.panels.commits.dirty = true,
            RevisionAction::TrackCommitsLoad(limit) => {
                self.commits_requested_limit = self.commits_requested_limit.max(limit)
            }
            RevisionAction::ExtendCommits { loading } => {
                let state = &mut self.panels.commits;
                if active != SidePanel::Commits
                    || state.tree_mode.active
                    || state.dirty
                    || loading
                    || state.items.is_empty()
                {
                    return output;
                }
                let Some(selected) = state.panel.selected() else {
                    return output;
                };
                if selected + COMMITS_LOAD_AHEAD_THRESHOLD < state.items.len() {
                    return output;
                }
                self.commits_requested_limit = self
                    .commits_requested_limit
                    .saturating_add(COMMITS_LOAD_STEP);
                state.dirty = true;
            }
            RevisionAction::RecomputeHighlight => {
                let state = &mut self.panels.commits;
                state.highlighted_oids = if active == SidePanel::Commits && !state.tree_mode.active
                {
                    state
                        .panel
                        .selected()
                        .and_then(|index| state.items.get(index))
                        .map(|commit| {
                            super::highlight::compute_highlight_set(&state.items, &commit.oid)
                        })
                        .unwrap_or_default()
                        .into()
                } else {
                    Default::default()
                };
            }
            RevisionAction::SetFetching(fetching) => {
                self.panels.branches.is_fetching_remote = fetching
            }
            RevisionAction::ReplaceBranches(items) => self.panels.branches.items = items,
            RevisionAction::ReplaceStashes(items) => self.panels.stash.items = items,
            RevisionAction::ReplaceCommits { items, fast } => {
                self.panels.commits.items = items;
                self.panels.commits.dirty = fast;
            }
            RevisionAction::BranchCommitsLoaded { branch, items } => {
                let state = &mut self.panels.branches;
                if state.commits_subview_source.as_deref() != Some(branch.as_str())
                    || !state.commits_subview_active
                {
                    return output;
                }
                state.commits_subview.items = items;
                state.commits_subview_loading = false;
                state
                    .commits_subview
                    .panel
                    .select((!state.commits_subview.items.is_empty()).then_some(0));
            }
            RevisionAction::CommitFilesLoaded { oid, files } => {
                let state = &mut self.panels.commits;
                if state.tree_mode.selected_source.as_deref() != Some(&oid) {
                    return output;
                }
                tree::enter_tree_mode(oid, files, &mut state.tree_mode, &mut state.panel);
            }
            RevisionAction::StashFilesLoaded { index, files } => {
                let state = &mut self.panels.stash;
                if state.tree_mode.selected_source != Some(index) {
                    return output;
                }
                tree::enter_tree_mode(index, files, &mut state.tree_mode, &mut state.panel);
            }
            RevisionAction::BranchLoadFailed => {
                self.panels.branches.commits_subview_loading = false
            }
            _ => return output,
        }
        output.accepted = true;
        output
    }
}
