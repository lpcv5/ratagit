use super::{DiffInteraction, DiffState, DiffTarget};
use crate::file_tree::FileTreeNodeStatus;
use crate::git::patch::{DiffDocument, HunkId, Patch, PatchDirection};

/// Read model shared by snapshots and application command selectors.
pub struct DiffView<'a> {
    pub document: &'a DiffDocument,
    pub interaction: &'a DiffInteraction,
    pub scroll: usize,
    pub loading: bool,
}

impl<'a> DiffView<'a> {
    pub fn hunks(&self, files_context: bool) -> HunkView<'a> {
        HunkView {
            patch: self.document.patch.as_deref(),
            interaction: self.interaction,
            ready: !self.loading,
            files_context,
        }
    }

    pub fn show_loading(&self) -> bool {
        self.loading && self.document.lines.is_empty()
    }
}

impl DiffState {
    pub fn view(&self) -> super::DiffView<'_> {
        DiffView {
            document: self.document(),
            interaction: self.interaction(),
            scroll: self.scroll(),
            loading: self.is_loading(),
        }
    }
}

/// The same immutable projection controls input, palette availability and hints.
pub struct HunkView<'a> {
    pub patch: Option<&'a Patch>,
    pub interaction: &'a DiffInteraction,
    pub ready: bool,
    pub files_context: bool,
}

impl HunkView<'_> {
    pub fn can_toggle_focus(&self) -> bool {
        self.interaction.focused || !self.ids().is_empty()
    }

    pub fn can_navigate(&self) -> bool {
        self.interaction.focused && !self.ids().is_empty()
    }

    pub fn can_switch_side(&self, target: &DiffTarget) -> bool {
        self.interaction.focused
            && self.ready
            && matches!(
                target,
                DiffTarget::File {
                    status: FileTreeNodeStatus::Staged(_) | FileTreeNodeStatus::Unstaged(_),
                    ..
                }
            )
    }

    pub fn can_apply(&self, direction: PatchDirection) -> bool {
        self.interaction.focused && !self.targets(direction).is_empty()
    }

    pub fn ids(&self) -> Vec<HunkId> {
        self.patch
            .filter(|_| self.ready)
            .map(Patch::hunk_ids)
            .unwrap_or_default()
    }

    pub fn targets(&self, direction: PatchDirection) -> Vec<HunkId> {
        if !self.ready || !self.files_context || self.interaction.pending_write.is_some() {
            return Vec::new();
        }
        let ids: Vec<_> = if self.interaction.selected.is_empty() {
            self.interaction.cursor.into_iter().collect()
        } else {
            self.interaction.selected.iter().copied().collect()
        };
        match self.patch {
            Some(patch) if patch.validate_selection(&ids, direction).is_ok() => ids,
            _ => Vec::new(),
        }
    }
}
