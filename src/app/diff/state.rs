use super::cache::{DiffCache, DiffCacheIdentity, DiffCacheKey, DiffCacheRevision, DiffCacheStats};
use super::{DiffRefreshSource, DiffTarget};
use crate::file_tree::FileTreeNode;
use crate::git::patch::{DiffDocument, HunkId, LineId};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Unstaged,
    Staged,
}

#[derive(Debug, Clone)]
pub struct DiffInteraction {
    pub side: Option<DiffSide>,
    pub focused: bool,
    pub cursor: Option<HunkId>,
    pub selected: BTreeSet<HunkId>,
    pub line_mode: bool,
    pub line_cursor: Option<LineId>,
    pub selected_lines: BTreeSet<LineId>,
    pub range_anchor: Option<LineId>,
    pub hunk_anchor: Option<HunkId>,
    pub pending_write: Option<u64>,
    pub(super) next_write: u64,
}

impl Default for DiffInteraction {
    fn default() -> Self {
        Self {
            side: None,
            focused: false,
            cursor: None,
            selected: BTreeSet::new(),
            line_mode: true,
            line_cursor: None,
            selected_lines: BTreeSet::new(),
            range_anchor: None,
            hunk_anchor: None,
            pending_write: None,
            next_write: 0,
        }
    }
}

impl DiffInteraction {
    pub(super) fn reset_selection(&mut self) {
        self.cursor = None;
        self.selected.clear();
        self.line_cursor = None;
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
    pub(super) interaction: DiffInteraction,
    pub(super) scroll: usize,
    cache: DiffCache,
    last_key: Option<DiffCacheIdentity>,
    in_flight_key: Option<DiffCacheIdentity>,
    worktree_revision: u64,
    branches_revision: u64,
    stashes_revision: u64,
    pending_reload: bool,
    pending_reload_at: Option<Instant>,
}

impl Default for DiffState {
    fn default() -> Self {
        Self {
            document: Vec::new().into(),
            interaction: DiffInteraction::default(),
            scroll: 0,
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
        self.interaction.reset_selection();
        self.document.patch = None;
        self.pending_reload = true;
        self.pending_reload_at = Some(now);
    }

    pub fn cancel_load(&mut self) {
        self.in_flight_key = None;
    }

    pub fn repository_refreshed(&mut self, include_refs: bool) {
        self.interaction.reset_selection();
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
            self.interaction.reset_selection();
        }
        if self
            .last_key
            .as_ref()
            .is_some_and(|key| !self.is_fresh(key))
        {
            self.last_key = None;
            self.document.patch = None;
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
            self.document = Vec::new().into();
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
        self.document = document;
        self.interaction.reset_selection();
        self.interaction.cursor = self
            .document
            .patch
            .as_ref()
            .and_then(|patch| patch.hunk_ids().first().copied());
        self.interaction.line_cursor = self
            .document
            .patch
            .as_ref()
            .and_then(|patch| patch.line_ids().first().copied());
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
