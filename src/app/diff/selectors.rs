use super::{DiffInteraction, DiffState, DiffTarget};
use crate::file_tree::FileTreeNodeStatus;
use crate::git::patch::{DiffDocument, HunkId, LineId, Patch, PatchDirection};
use std::collections::BTreeSet;

/// Read model shared by snapshots and application command selectors.
pub struct DiffView<'a> {
    pub document: &'a DiffDocument,
    pub retained_patch: Option<&'a std::sync::Arc<Patch>>,
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
            retained_patch: self.retained_patch.as_ref(),
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
    pub fn cursor_position(&self) -> (usize, usize) {
        fn position<T: PartialEq>(ids: Vec<T>, cursor: Option<T>) -> (usize, usize) {
            let current = cursor
                .and_then(|cursor| ids.iter().position(|id| *id == cursor))
                .map(|index| index + 1)
                .unwrap_or(0);
            (current, ids.len())
        }
        if self.interaction.line_mode {
            position(self.line_ids(), self.interaction.line_cursor)
        } else {
            position(self.ids(), self.interaction.cursor)
        }
    }

    pub fn cursor_is_read_only(&self) -> bool {
        self.patch.is_some_and(|patch| {
            self.interaction.cursor.is_some_and(|id| {
                patch
                    .validate_file_hunks(id.file, &[id.hunk], PatchDirection::Stage)
                    .is_err()
                    && patch
                        .validate_file_hunks(id.file, &[id.hunk], PatchDirection::Unstage)
                        .is_err()
            })
        })
    }

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
                !self.change_targets(direction).is_empty()
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
            ordered_range(self.line_ids(), anchor, cursor)
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

    /// Block starts in display order, including multiple edits in one Git hunk.
    pub fn ids(&self) -> Vec<LineId> {
        self.patch
            .filter(|_| self.ready)
            .map(Patch::change_ids)
            .unwrap_or_default()
    }

    /// Native hunk targets only when the selection covers all their edits.
    pub fn targets(&self, direction: PatchDirection) -> Vec<HunkId> {
        let lines = self.change_targets(direction);
        let selected: BTreeSet<_> = lines.iter().copied().collect();
        let mut hunks = Vec::new();
        for line in &lines {
            if !hunks.contains(&line.hunk()) {
                hunks.push(line.hunk());
            }
        }
        // Keep the native hunk fast path only when all its edits are selected.
        // Partial blocks must use the reviewed line-patch backend.
        if self.patch.is_some_and(|patch| {
            patch
                .line_ids()
                .iter()
                .any(|id| hunks.contains(&id.hunk()) && !selected.contains(id))
        }) {
            return Vec::new();
        }
        hunks
    }

    /// Exact changed lines of the selected blocks, excluding context/EOF markers.
    pub fn change_targets(&self, direction: PatchDirection) -> Vec<LineId> {
        if !self.ready || !self.files_context || self.interaction.pending_write.is_some() {
            return Vec::new();
        }
        let marked = self.marked_hunks();
        let ids: Vec<_> = if marked.is_empty() {
            self.interaction.cursor.into_iter().collect()
        } else {
            marked
        };
        let Some(patch) = self.patch else {
            return Vec::new();
        };
        let Some(lines) = patch.change_lines(&ids) else {
            return Vec::new();
        };
        let hunks: BTreeSet<_> = ids.iter().map(|id| id.hunk()).collect();
        // Expansion only yields reviewed changed lines. Validate each parent
        // once for availability instead of rechecking its metadata per line.
        if patch
            .validate_selection(&hunks.into_iter().collect::<Vec<_>>(), direction)
            .is_ok()
        {
            lines
        } else {
            Vec::new()
        }
    }

    pub fn marked_hunks(&self) -> Vec<LineId> {
        if let Some((anchor, cursor)) = self.interaction.hunk_anchor.zip(self.interaction.cursor) {
            ordered_range(self.ids(), anchor, cursor)
        } else {
            self.interaction.selected.iter().copied().collect()
        }
    }
}

fn ordered_range<T: PartialEq>(ids: Vec<T>, anchor: T, cursor: T) -> Vec<T> {
    let Some((start, end)) = ids
        .iter()
        .position(|id| *id == anchor)
        .zip(ids.iter().position(|id| *id == cursor))
    else {
        return Vec::new();
    };
    let (start, end) = (start.min(end), start.max(end));
    ids.into_iter().skip(start).take(end - start + 1).collect()
}
