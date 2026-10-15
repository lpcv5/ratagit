use super::{FileSelection, FilesAction};
use crate::app::states::PanelState;
use crate::file_tree::{build_file_tree, collect_dirs, FileTreeNode, TreeCursor};
use crate::git::GitStatus;
use crate::shared::Shared;
use std::collections::HashSet;
use std::path::PathBuf;

/// Domain file-tree state. Render projections omit expansion bookkeeping.
#[derive(Default, Clone)]
pub struct FilesPanelState {
    pub panel: PanelState,
    pub tree_nodes: Shared<Vec<FileTreeNode>>,
    pub expanded_dirs: Shared<HashSet<PathBuf>>,
    pub visual_mode: bool,
    pub visual_anchor: Option<usize>,
}

#[derive(Default)]
pub struct FilesState {
    pub(super) status: Shared<GitStatus>,
    pub(super) panel: FilesPanelState,
}

impl FilesState {
    pub fn new(status: GitStatus, expand: bool) -> Self {
        let expanded_dirs = if expand {
            collect_all_dirs(&status)
        } else {
            HashSet::new()
        };
        let tree_nodes = build_file_tree(
            &status.unstaged,
            &status.untracked,
            &status.staged,
            &expanded_dirs,
        );
        Self {
            status: status.into(),
            panel: FilesPanelState {
                tree_nodes: tree_nodes.into(),
                expanded_dirs: expanded_dirs.into(),
                ..Default::default()
            },
        }
    }

    pub fn view(&self) -> &FilesPanelState {
        &self.panel
    }

    pub fn status(&self) -> &GitStatus {
        &self.status
    }

    pub fn selection(&self, active: bool) -> FileSelection<'_> {
        FileSelection::new(self.view().render_view(), active)
    }

    pub fn change_count(&self) -> usize {
        self.status.staged.len() + self.status.unstaged.len() + self.status.untracked.len()
    }

    pub fn all_file_paths(&self) -> Vec<PathBuf> {
        self.status
            .staged
            .iter()
            .chain(&self.status.unstaged)
            .chain(&self.status.untracked)
            .map(|entry| entry.path.clone())
            .collect()
    }

    pub(super) fn replace_status(&mut self, status: Shared<GitStatus>) {
        let old_dirs = collect_all_dirs(&self.status);
        self.status = status;
        let new_dirs = collect_all_dirs(&self.status);
        self.panel
            .expanded_dirs
            .retain(|dir| new_dirs.contains(dir));
        self.panel
            .expanded_dirs
            .extend(new_dirs.difference(&old_dirs).cloned());
        self.rebuild_tree();
    }

    pub(super) fn rebuild_tree(&mut self) {
        let cursor = TreeCursor::capture(&self.panel.tree_nodes, self.panel.panel.selected());
        let anchor = self
            .panel
            .visual_anchor
            .map(|index| TreeCursor::capture(&self.panel.tree_nodes, Some(index)));
        self.panel.tree_nodes = build_file_tree(
            &self.status.unstaged,
            &self.status.untracked,
            &self.status.staged,
            &self.panel.expanded_dirs,
        )
        .into();
        self.panel
            .panel
            .select(cursor.restore(&self.panel.tree_nodes));
        self.panel.visual_anchor = anchor.and_then(|cursor| cursor.restore(&self.panel.tree_nodes));
    }

    pub(super) fn expand_all(&mut self) {
        self.panel.expanded_dirs = collect_all_dirs(&self.status).into();
        self.rebuild_tree();
    }

    pub(super) fn clear_visual_selection(&mut self) {
        self.panel.visual_mode = false;
        self.panel.visual_anchor = None;
    }

    #[cfg(test)]
    pub(crate) fn test_view_mut(&mut self) -> &mut FilesPanelState {
        &mut self.panel
    }

    #[cfg(test)]
    pub(crate) fn test_status_mut(&mut self) -> &mut GitStatus {
        &mut self.status
    }

    pub fn select_index(&mut self, index: usize) {
        self.update(FilesAction::SelectIndex(index), true);
    }
}

pub(super) fn collect_all_dirs(status: &GitStatus) -> HashSet<PathBuf> {
    collect_dirs(
        status
            .unstaged
            .iter()
            .chain(&status.untracked)
            .chain(&status.staged)
            .map(|entry| entry.path.as_path()),
    )
}
