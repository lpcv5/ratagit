use super::{DiffInteraction, DiffState, DiffTarget};
use crate::file_tree::FileTreeNodeStatus;
use crate::git::patch::{DiffDocument, HunkId, LineId, Patch, PatchDirection};

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
        self.interaction.focused || (!self.ready && self.files_context) || !self.ids().is_empty()
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
        self.interaction.focused
            && if self.interaction.line_mode {
                !self.line_targets(direction).is_empty()
            } else {
                !self.targets(direction).is_empty()
            }
    }

    pub fn line_ids(&self) -> Vec<LineId> {
        self.patch
            .filter(|_| self.ready)
            .map(Patch::cursor_line_ids)
            .unwrap_or_default()
    }

    /// Explicit selections/ranges for rendering; the cursor alone is not marked.
    pub fn marked_lines(&self) -> Vec<LineId> {
        if let Some((anchor, cursor)) = self
            .interaction
            .range_anchor
            .zip(self.interaction.line_cursor)
        {
            let (start, end) = (anchor.min(cursor), anchor.max(cursor));
            self.line_ids()
                .into_iter()
                .filter(|id| *id >= start && *id <= end)
                .collect()
        } else {
            self.interaction.selected_lines.iter().copied().collect()
        }
    }

    pub fn line_targets(&self, direction: PatchDirection) -> Vec<LineId> {
        if !self.ready || !self.files_context || self.interaction.pending_write.is_some() {
            return Vec::new();
        }
        let mut ids = self.marked_lines();
        if ids.is_empty() {
            ids.extend(self.interaction.line_cursor);
        }
        if let Some(patch) = self.patch {
            ids.retain(|id| {
                patch
                    .files
                    .get(id.file)
                    .and_then(|file| file.hunks.get(id.hunk))
                    .and_then(|hunk| hunk.lines.get(id.line))
                    .is_some_and(|line| {
                        matches!(
                            line.kind,
                            crate::git::patch::PatchLineKind::Added
                                | crate::git::patch::PatchLineKind::Removed
                        )
                    })
            });
        }
        match self.patch {
            Some(patch) if patch.validate_lines(&ids, direction).is_ok() => ids,
            _ => Vec::new(),
        }
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
        let marked = self.marked_hunks();
        let ids: Vec<_> = if marked.is_empty() {
            self.interaction.cursor.into_iter().collect()
        } else {
            marked
        };
        match self.patch {
            Some(patch) if patch.validate_selection(&ids, direction).is_ok() => ids,
            _ => Vec::new(),
        }
    }

    pub fn marked_hunks(&self) -> Vec<HunkId> {
        if let Some((anchor, cursor)) = self.interaction.hunk_anchor.zip(self.interaction.cursor) {
            let (start, end) = (anchor.min(cursor), anchor.max(cursor));
            self.ids()
                .into_iter()
                .filter(|id| *id >= start && *id <= end)
                .collect()
        } else {
            self.interaction.selected.iter().copied().collect()
        }
    }
}
