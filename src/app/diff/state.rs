use super::cache::{DiffCache, DiffCacheKey};
use super::DiffTarget;
use crate::file_tree::FileTreeNode;
use crate::git::patch::{DiffDocument, HunkId};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffSide {
    Unstaged,
    Staged,
}

#[derive(Debug, Clone, Default)]
pub struct DiffInteraction {
    pub side: Option<DiffSide>,
    pub focused: bool,
    pub cursor: Option<HunkId>,
    pub selected: BTreeSet<HunkId>,
    pub pending_write: Option<u64>,
    pub(super) next_write: u64,
}

impl DiffInteraction {
    pub(super) fn reset_selection(&mut self) {
        self.cursor = None;
        self.selected.clear();
    }
}

/// All mutable diff data lives here. Consumers receive read-only projections.
pub struct DiffState {
    pub(super) document: DiffDocument,
    pub(super) interaction: DiffInteraction,
    pub(super) scroll: usize,
    cache: DiffCache,
    last_key: Option<DiffCacheKey>,
    in_flight_key: Option<DiffCacheKey>,
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
            pending_reload: false,
            pending_reload_at: None,
        }
    }
}

pub(crate) enum DiffLoad {
    Unchanged,
    Ready,
    Request {
        cache_key: DiffCacheKey,
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
        self.cache.invalidate_files();
        if include_refs {
            self.cache.invalidate_refs();
        }
        let invalidated = matches!(
            self.last_key,
            Some(DiffCacheKey::File { .. } | DiffCacheKey::Directory { .. })
        ) || (include_refs
            && matches!(
                self.last_key,
                Some(DiffCacheKey::Branch { .. } | DiffCacheKey::Stash { .. })
            ));
        if invalidated {
            self.last_key = None;
            self.document.patch = None;
        }
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
        let cache_key = target.cache_key(visible_files);
        if self.in_flight_key.as_ref() == Some(&cache_key) {
            self.clear_reload_request();
            return DiffLoad::Unchanged;
        }
        self.cancel_load();
        self.scroll = 0;
        if let Some(document) = self.cache.get_cloned(&cache_key) {
            self.install_document(document);
            self.last_key = Some(cache_key);
            self.clear_reload_request();
            return DiffLoad::Ready;
        }
        if matches!(target, DiffTarget::None) {
            self.document = Vec::new().into();
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
        cache_key: DiffCacheKey,
        document: DiffDocument,
        selected_target: &DiffTarget,
        visible_files: &[FileTreeNode],
    ) -> bool {
        self.finish_load();
        if selected_target.cache_key(visible_files) != cache_key {
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
