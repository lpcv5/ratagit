//! Cross-feature adapters for file state. Repository I/O remains in effects.
use super::files::{FileSelection, FilesAction, FilesUpdate};
use super::{AppState, SidePanel};
use crate::git::GitStatus;
use std::collections::HashSet;
use std::path::PathBuf;

impl AppState {
    pub(crate) fn reduce_files(&mut self, action: FilesAction<'_>) -> FilesUpdate {
        self.files
            .update(action, self.ui.active_panel == SidePanel::Files)
    }

    pub(super) fn apply_status_refresh(&mut self, status: GitStatus) {
        self.reduce_files(FilesAction::ReplaceStatus(status));
        if self
            .file_selection()
            .selected_tree_node()
            .is_some_and(|node| {
                matches!(node.status, crate::file_tree::FileTreeNodeStatus::Untracked)
            })
        {
            self.diff.selected_file_became_untracked();
        }
    }

    pub fn file_selection(&self) -> FileSelection<'_> {
        self.files
            .selection(self.ui.active_panel == SidePanel::Files)
    }

    pub fn visual_selected_indices(&self) -> HashSet<usize> {
        self.file_selection().visual_selected_indices()
    }

    pub fn prepare_stash_targets_from_selection(&self) -> Vec<PathBuf> {
        self.file_selection().prepare_stash_targets_from_selection()
    }

    #[cfg(test)]
    pub fn prepare_discard_targets_from_selection(&self) -> Vec<PathBuf> {
        self.file_selection()
            .prepare_discard_targets_from_selection()
    }

    #[cfg(test)]
    pub(super) fn partition_toggle_targets(
        &self,
        selected: &HashSet<usize>,
    ) -> (Vec<PathBuf>, Vec<PathBuf>) {
        self.file_selection().partition_toggle_targets(selected)
    }

    #[cfg(test)]
    pub(super) fn collect_commit_targets(&self, selected: &HashSet<usize>) -> Vec<PathBuf> {
        self.file_selection().collect_commit_targets(selected)
    }

    #[cfg(test)]
    pub fn toggle_visual_select_mode(&mut self) {
        self.reduce_files(FilesAction::ToggleVisual);
    }

    pub fn has_uncommitted_changes(&self) -> bool {
        self.files.change_count() != 0
    }

    pub fn toggle_selected_dir(&mut self) {
        self.reduce_files(FilesAction::ToggleDirectory);
    }

    pub fn collapse_all(&mut self) {
        self.reduce_files(FilesAction::CollapseAll);
    }

    pub fn expand_all(&mut self) {
        self.reduce_files(FilesAction::ExpandAll);
    }
}
