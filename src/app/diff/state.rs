use super::cache::{DiffCache, DiffCacheIdentity, DiffCacheKey, DiffCacheRevision, DiffCacheStats};
use super::position::HunkPosition;
use super::{DiffRefreshSource, DiffTarget};
use crate::file_tree::FileTreeNode;
use crate::git::patch::{DiffDocument, LineId, Patch, PatchRequest};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Unstaged,
    Staged,
}

/// A successful write relocated the same reviewed change in a fresh document.
#[derive(Debug, Clone, Copy)]
pub struct DiffCursorRelocation {
    pub revision: u64,
    pub previous_row: usize,
    pub row: usize,
}

#[derive(Debug, Clone, Default)]
pub struct DiffInteraction {
    pub side: Option<DiffSide>,
    pub focused: bool,
    /// First changed line of the selected contiguous edit block.
    pub cursor: Option<LineId>,
    pub selected: BTreeSet<LineId>,
    pub line_mode: bool,
    pub line_cursor: Option<LineId>,
    pub selected_lines: BTreeSet<LineId>,
    pub range_anchor: Option<LineId>,
    pub hunk_anchor: Option<LineId>,
    pub pending_write: Option<u64>,
    pub cursor_relocation: Option<DiffCursorRelocation>,
    pub(super) next_write: u64,
}

impl DiffInteraction {
    pub(super) fn reset_selection(&mut self) {
        self.cursor = None;
        self.line_cursor = None;
        self.clear_marks();
    }

    pub(super) fn clear_marks(&mut self) {
        self.selected.clear();
        self.selected_lines.clear();
        self.range_anchor = None;
        self.hunk_anchor = None;
    }

    pub fn has_selection(&self) -> bool {
        !self.selected.is_empty()
            || !self.selected_lines.is_empty()
            || self.range_anchor.is_some()
            || self.hunk_anchor.is_some()
    }
}

/// All mutable diff data lives here. Consumers receive read-only projections.
pub struct DiffState {
    pub(super) document: DiffDocument,
    // Display-only metadata stays paired with the retained text during reloads.
    pub(super) retained_patch: Option<Arc<Patch>>,
    pub(super) interaction: DiffInteraction,
    pub(super) scroll: usize,
    reload_position: Option<DiffPosition>,
    pub(super) write_position: Option<HunkPosition>,
    cache: DiffCache,
    last_key: Option<DiffCacheIdentity>,
    in_flight_key: Option<DiffCacheIdentity>,
    worktree_revision: u64,
    branches_revision: u64,
    stashes_revision: u64,
    pending_reload: bool,
    pending_reload_at: Option<Instant>,
}

struct DiffPosition {
    cursor: Option<LineId>,
    line_cursor: Option<LineId>,
    scroll: usize,
}

impl Default for DiffState {
    fn default() -> Self {
        Self {
            document: Vec::new().into(),
            retained_patch: None,
            interaction: DiffInteraction::default(),
            scroll: 0,
            reload_position: None,
            write_position: None,
            cache: DiffCache::new(),
            last_key: None,
            in_flight_key: None,
            worktree_revision: 0,
            branches_revision: 0,
            stashes_revision: 0,
            pending_reload: false,
            pending_reload_at: None,
        }
    }
}

pub(crate) enum DiffLoad {
    Unchanged,
    Ready,
    Request {
        cache_key: DiffCacheIdentity,
        target: DiffTarget,
    },
}

impl DiffState {
    pub fn document(&self) -> &DiffDocument {
        &self.document
    }

    pub fn interaction(&self) -> &DiffInteraction {
        &self.interaction
    }

    pub fn scroll(&self) -> usize {
        self.scroll
    }

    pub fn is_loading(&self) -> bool {
        self.pending_reload || self.in_flight_key.is_some()
    }

    pub fn has_pending_reload(&self) -> bool {
        self.pending_reload
    }

    pub fn pending_reload_at(&self) -> Option<Instant> {
        self.pending_reload_at
    }

    pub fn debounce_elapsed(&self, debounce: Duration, now: Instant) -> bool {
        self.pending_reload_at
            .is_some_and(|at| now.saturating_duration_since(at) >= debounce)
    }

    /// Keep the old preview while the new patch is loading, but disable writes.
    pub fn schedule_reload(&mut self, now: Instant) {
        self.remember_reload_position();
        self.interaction.reset_selection();
        self.retain_preview();
        self.pending_reload = true;
        self.pending_reload_at = Some(now);
    }

    pub fn cancel_load(&mut self) {
        self.in_flight_key = None;
    }

    fn retain_preview(&mut self) {
        if let Some(patch) = self.document.patch.take() {
            self.retained_patch = Some(patch);
        }
    }

    fn remember_reload_position(&mut self) {
        if self.interaction.focused
            && self.interaction.pending_write.is_none()
            && self.interaction.cursor.is_some()
        {
            self.reload_position = Some(DiffPosition {
                cursor: self.interaction.cursor,
                line_cursor: self.interaction.line_cursor,
                scroll: self.scroll,
            });
        }
    }

    pub(super) fn forget_reload_position(&mut self) {
        self.reload_position = None;
        self.write_position = None;
        self.interaction.cursor_relocation = None;
    }

    pub fn repository_refreshed(&mut self, include_refs: bool) {
        // Immutable commit previews do not reload after a status refresh.
        // Only invalidated sources need to suspend their navigation cursors.
        self.interaction.clear_marks();
        self.source_refreshed(DiffRefreshSource::Status);
        if include_refs {
            self.branches_revision = self
                .branches_revision
                .checked_add(1)
                .expect("diff revision exhausted");
            self.stashes_revision = self
                .stashes_revision
                .checked_add(1)
                .expect("diff revision exhausted");
            self.cache.invalidate_refs();
            self.clear_invalidated_load();
        }
    }

    /// Observations invalidate even when paths/status labels have not changed.
    pub fn source_refreshed(&mut self, source: DiffRefreshSource) {
        match source {
            DiffRefreshSource::Status => {
                self.worktree_revision = self
                    .worktree_revision
                    .checked_add(1)
                    .expect("diff revision exhausted");
                self.cache.invalidate_files();
            }
            DiffRefreshSource::Branches => {
                self.branches_revision = self
                    .branches_revision
                    .checked_add(1)
                    .expect("diff revision exhausted");
                self.cache.invalidate_branches();
            }
            DiffRefreshSource::Stashes => {
                self.stashes_revision = self
                    .stashes_revision
                    .checked_add(1)
                    .expect("diff revision exhausted");
                self.cache.invalidate_stashes();
            }
            DiffRefreshSource::Commits => return,
        }
        self.clear_invalidated_load();
    }

    fn clear_invalidated_load(&mut self) {
        let invalidated = self
            .last_key
            .as_ref()
            .is_some_and(|key| !self.is_fresh(key))
            || self
                .in_flight_key
                .as_ref()
                .is_some_and(|key| !self.is_fresh(key));
        if invalidated {
            self.remember_reload_position();
            self.interaction.reset_selection();
        }
        if self
            .last_key
            .as_ref()
            .is_some_and(|key| !self.is_fresh(key))
        {
            self.last_key = None;
            self.retain_preview();
        }
        if self
            .in_flight_key
            .as_ref()
            .is_some_and(|key| !self.is_fresh(key))
        {
            self.cancel_load();
        }
    }

    fn is_fresh(&self, key: &DiffCacheIdentity) -> bool {
        match key.revision {
            DiffCacheRevision::Immutable => true,
            DiffCacheRevision::Worktree(revision) => revision == self.worktree_revision,
            DiffCacheRevision::Branches(revision) => revision == self.branches_revision,
            DiffCacheRevision::Stashes(revision) => revision == self.stashes_revision,
        }
    }

    fn identity(&self, key: DiffCacheKey) -> DiffCacheIdentity {
        let revision = match key {
            DiffCacheKey::None | DiffCacheKey::Commit { .. } => 0,
            DiffCacheKey::File { .. } | DiffCacheKey::Directory { .. } => self.worktree_revision,
            DiffCacheKey::Branch { .. } => self.branches_revision,
            DiffCacheKey::Stash { .. } => self.stashes_revision,
        };
        key.at_revision(revision)
    }

    pub(crate) fn accepts_load(&self, key: &DiffCacheIdentity) -> bool {
        self.in_flight_key.as_ref() == Some(key) && self.is_fresh(key)
    }

    pub fn cache_stats(&self) -> DiffCacheStats {
        self.cache.stats()
    }

    pub(crate) fn reject_load(&mut self) {
        self.cache.reject();
    }

    #[cfg(test)]
    pub(crate) fn set_test_cache_limits(&mut self, entries: usize, bytes: usize) {
        self.cache = DiffCache::with_limits(entries, bytes);
    }

    pub fn selected_file_became_untracked(&mut self) {
        if self.interaction.focused {
            self.interaction.side = Some(DiffSide::Unstaged);
        }
    }

    /// Decide whether to reuse content or issue an owned background request.
    /// The coordinator cancels the previous task for every non-Unchanged result.
    pub(crate) fn prepare_reload(
        &mut self,
        target: DiffTarget,
        visible_files: &[FileTreeNode],
    ) -> DiffLoad {
        if !self.pending_reload {
            return DiffLoad::Unchanged;
        }
        let cache_key = self.identity(target.cache_key(visible_files));
        if self.in_flight_key.as_ref() == Some(&cache_key) {
            self.clear_reload_request();
            return DiffLoad::Unchanged;
        }
        self.cancel_load();
        self.scroll = 0;
        if matches!(target, DiffTarget::None) {
            self.install_document(Vec::new().into());
            self.last_key = Some(cache_key);
            self.clear_reload_request();
            return DiffLoad::Ready;
        }
        if let Some(document) = self.cache.get_cloned(&cache_key) {
            self.install_document(document);
            self.last_key = Some(cache_key);
            self.clear_reload_request();
            return DiffLoad::Ready;
        }
        self.in_flight_key = Some(cache_key.clone());
        self.clear_reload_request();
        DiffLoad::Request { cache_key, target }
    }

    /// Generation checks happen at the task boundary before calling this method.
    /// A still-current generation can nevertheless target an obsolete selection.
    pub fn complete_load(
        &mut self,
        cache_key: DiffCacheIdentity,
        document: DiffDocument,
        selected_target: &DiffTarget,
        visible_files: &[FileTreeNode],
    ) -> bool {
        if !self.accepts_load(&cache_key) {
            self.cache.reject();
            return false;
        }
        self.finish_load();
        if self.identity(selected_target.cache_key(visible_files)) != cache_key {
            self.cache.reject();
            return false;
        }
        self.cache.insert(cache_key.clone(), document.clone());
        self.last_key = Some(cache_key);
        self.install_document(document);
        true
    }

    pub fn finish_load(&mut self) {
        self.cancel_load();
        if !self.pending_reload {
            self.pending_reload_at = None;
        }
    }

    fn clear_reload_request(&mut self) {
        self.pending_reload = false;
        self.pending_reload_at = None;
    }

    fn install_document(&mut self, document: DiffDocument) {
        // A periodic refresh must not move the cursor when the reviewed patch
        // is unchanged. Never restore marked ranges or positions into new text.
        let position = self.reload_position.take().filter(|_| {
            self.interaction.focused
                && document.patch.is_some()
                && self
                    .retained_patch
                    .as_ref()
                    .or(self.document.patch.as_ref())
                    .zip(document.patch.as_ref())
                    .is_some_and(|(previous, current)| same_preview(previous, current))
        });
        self.document = document;
        self.retained_patch = None;
        self.interaction.reset_selection();
        self.interaction.cursor = self.document.patch.as_ref().and_then(|patch| {
            let ids = patch.change_ids();
            if matches!(patch.request, crate::git::patch::PatchRequest::File(_)) {
                let preferred = if self.interaction.side == Some(DiffSide::Staged) {
                    crate::git::patch::PatchDirection::Unstage
                } else {
                    crate::git::patch::PatchDirection::Stage
                };
                ids.iter()
                    .copied()
                    .find(|id| patch.files[id.file].source.direction() == Some(preferred))
                    .or_else(|| ids.first().copied())
            } else {
                ids.first().copied()
            }
        });
        self.interaction.line_cursor = self.interaction.cursor;
        if let Some(position) = position {
            self.interaction.cursor = position.cursor;
            self.interaction.line_cursor = position.line_cursor;
            self.scroll = position.scroll;
        }
        let write_position = if self.interaction.pending_write.is_none() {
            self.write_position.take()
        } else {
            None
        };
        if let Some(location) = write_position
            .filter(|_| self.interaction.focused)
            .and_then(|position| position.find(self.document.patch.as_ref()?))
        {
            self.interaction.cursor = self
                .document
                .patch
                .as_ref()
                .and_then(|patch| patch.change_for_line(location.cursor));
            self.interaction.line_cursor = Some(location.cursor);
            self.interaction.cursor_relocation = Some(DiffCursorRelocation {
                revision: self.interaction.next_write,
                previous_row: location.previous_row,
                row: location.row,
            });
            // Keep the page-scroll baseline aligned with the relocated view,
            // so the next page movement continues from the pre-write position.
            self.scroll = location
                .scroll
                .min(self.document.lines.len().saturating_sub(1));
        }
        self.sync_cursor_side();
    }

    #[cfg(test)]
    pub(crate) fn set_test_document(&mut self, document: DiffDocument) {
        self.install_document(document);
    }

    #[cfg(test)]
    pub(crate) fn test_interaction_mut(&mut self) -> &mut DiffInteraction {
        &mut self.interaction
    }
}

fn same_preview(previous: &Patch, current: &Patch) -> bool {
    if previous == current {
        return true;
    }
    // Status can switch a new file between the combined and untracked
    // requests after the write's fresh patch has already been installed.
    // Identical reviewed bytes in that same path keep the saved position.
    let same_path = matches!((&previous.request, &current.request),
        (PatchRequest::File(a), PatchRequest::Untracked(b))
            | (PatchRequest::Untracked(a), PatchRequest::File(b)) if a == b);
    same_path && previous.truncated == current.truncated && previous.files == current.files
}
