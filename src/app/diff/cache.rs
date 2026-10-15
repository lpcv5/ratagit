use crate::git::patch::DiffDocument;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;

const MAX_ENTRIES: usize = 50;
const MAX_BYTES: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileDiffSource {
    Worktree,
    Index,
    Untracked,
    Directory,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiffCacheKey {
    /// Represents a missing or empty diff target (no selection).
    None,
    File {
        path: PathBuf,
        source: FileDiffSource,
    },
    Branch {
        name: String,
        limit: usize,
    },
    Directory {
        path: PathBuf,
        files_hash: u64,
    },
    Commit {
        oid: String,
        path: Option<PathBuf>,
    },
    Stash {
        index: usize,
        path: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiffCacheRevision {
    Immutable,
    Worktree(u64),
    Branches(u64),
    Stashes(u64),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DiffCacheIdentity {
    pub target: DiffCacheKey,
    pub revision: DiffCacheRevision,
}

impl DiffCacheKey {
    pub fn at_revision(self, revision: u64) -> DiffCacheIdentity {
        let revision = match &self {
            Self::None | Self::Commit { .. } => DiffCacheRevision::Immutable,
            Self::File { .. } | Self::Directory { .. } => DiffCacheRevision::Worktree(revision),
            Self::Branch { .. } => DiffCacheRevision::Branches(revision),
            Self::Stash { .. } => DiffCacheRevision::Stashes(revision),
        };
        DiffCacheIdentity {
            target: self,
            revision,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct DiffCacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub oversized: u64,
    pub invalidations: u64,
    pub rejected: u64,
    pub entries: usize,
    pub retained_bytes: usize,
    pub max_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum PayloadId {
    Lines(usize),
    Patch(usize),
}

struct PayloadUse {
    bytes: usize,
    entries: usize,
}

pub struct DiffCache {
    cache: HashMap<DiffCacheIdentity, DiffDocument>,
    usage_order: VecDeque<DiffCacheIdentity>,
    max_entries: usize,
    max_bytes: usize,
    payloads: HashMap<PayloadId, PayloadUse>,
    retained_bytes: usize,
    stats: DiffCacheStats,
}

impl DiffCache {
    pub fn new() -> Self {
        Self::with_limits(MAX_ENTRIES, MAX_BYTES)
    }

    pub(super) fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            cache: HashMap::new(),
            usage_order: VecDeque::new(),
            max_entries,
            max_bytes,
            payloads: HashMap::new(),
            retained_bytes: 0,
            stats: DiffCacheStats::default(),
        }
    }

    pub fn get_cloned(&mut self, key: &DiffCacheIdentity) -> Option<DiffDocument> {
        let Some(diff) = self.cache.get(key).cloned() else {
            self.stats.misses = self.stats.misses.saturating_add(1);
            return None;
        };
        self.stats.hits = self.stats.hits.saturating_add(1);
        self.touch(key);
        Some(diff)
    }

    pub fn insert(&mut self, key: DiffCacheIdentity, diff: impl Into<DiffDocument>) {
        let diff = diff.into();
        // A rejected replacement must not leave older content under the same key.
        self.remove(&key);
        let components = self.components(&diff);
        let own_bytes = components
            .iter()
            .flatten()
            .fold(0usize, |sum, (_, bytes)| sum.saturating_add(*bytes));
        if self.max_entries == 0 || own_bytes > self.max_bytes {
            self.stats.oversized = self.stats.oversized.saturating_add(1);
            return;
        }
        while self.cache.len() >= self.max_entries
            || self
                .retained_bytes
                .saturating_add(self.additional_bytes(&components))
                > self.max_bytes
        {
            self.evict_lru();
        }
        for (id, bytes) in components.into_iter().flatten() {
            let usage = self.payloads.entry(id).or_insert_with(|| {
                self.retained_bytes += bytes;
                PayloadUse { bytes, entries: 0 }
            });
            usage.entries += 1;
        }
        self.cache.insert(key.clone(), diff);
        self.usage_order.push_back(key);
    }

    pub fn invalidate_files(&mut self) {
        self.invalidate(|key| {
            matches!(
                key,
                DiffCacheKey::File { .. } | DiffCacheKey::Directory { .. }
            )
        });
    }

    pub fn invalidate_refs(&mut self) {
        self.invalidate(|key| {
            matches!(
                key,
                DiffCacheKey::Branch { .. } | DiffCacheKey::Stash { .. }
            )
        });
    }

    pub fn invalidate_branches(&mut self) {
        self.invalidate(|key| matches!(key, DiffCacheKey::Branch { .. }));
    }

    pub fn invalidate_stashes(&mut self) {
        self.invalidate(|key| matches!(key, DiffCacheKey::Stash { .. }));
    }

    fn invalidate(&mut self, predicate: impl Fn(&DiffCacheKey) -> bool) {
        for _ in 0..self.usage_order.len() {
            let key = self.usage_order.pop_front().expect("tracked cache entry");
            if predicate(&key.target) {
                self.remove(&key);
                self.stats.invalidations = self.stats.invalidations.saturating_add(1);
            } else {
                self.usage_order.push_back(key);
            }
        }
    }

    fn touch(&mut self, key: &DiffCacheIdentity) {
        self.usage_order.retain(|existing| existing != key);
        self.usage_order.push_back(key.clone());
    }

    fn evict_lru(&mut self) {
        while let Some(oldest) = self.usage_order.pop_front() {
            if self.remove(&oldest) {
                self.stats.evictions = self.stats.evictions.saturating_add(1);
                break;
            }
        }
    }

    fn payload_ids(diff: &DiffDocument) -> [Option<PayloadId>; 2] {
        [
            Some(PayloadId::Lines(diff.lines.allocation_id())),
            diff.patch
                .as_ref()
                .map(|patch| PayloadId::Patch(Arc::as_ptr(patch) as usize)),
        ]
    }

    fn components(&self, diff: &DiffDocument) -> [Option<(PayloadId, usize)>; 2] {
        Self::payload_ids(diff).map(|id| {
            id.map(|id| {
                let bytes = self.payloads.get(&id).map_or_else(
                    || match id {
                        PayloadId::Lines(_) => super::memory::lines_bytes(&diff.lines),
                        PayloadId::Patch(_) => {
                            super::memory::patch_bytes(diff.patch.as_ref().expect("patch identity"))
                        }
                    },
                    |usage| usage.bytes,
                );
                (id, bytes)
            })
        })
    }

    fn additional_bytes(&self, components: &[Option<(PayloadId, usize)>; 2]) -> usize {
        components
            .iter()
            .flatten()
            .filter(|(id, _)| !self.payloads.contains_key(id))
            .fold(0usize, |sum, (_, bytes)| sum.saturating_add(*bytes))
    }

    fn remove(&mut self, key: &DiffCacheIdentity) -> bool {
        let Some(diff) = self.cache.remove(key) else {
            return false;
        };
        self.usage_order.retain(|existing| existing != key);
        for id in Self::payload_ids(&diff).into_iter().flatten() {
            let usage = self.payloads.get_mut(&id).expect("accounted payload");
            usage.entries -= 1;
            if usage.entries == 0 {
                self.retained_bytes -= usage.bytes;
                self.payloads.remove(&id);
            }
        }
        true
    }

    pub fn reject(&mut self) {
        self.stats.rejected = self.stats.rejected.saturating_add(1);
    }

    pub fn stats(&self) -> DiffCacheStats {
        DiffCacheStats {
            entries: self.cache.len(),
            retained_bytes: self.retained_bytes,
            max_bytes: self.max_bytes,
            ..self.stats
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{DiffLine, DiffLineKind};
    use pretty_assertions::assert_eq;

    fn make_diff(lines: usize) -> Vec<DiffLine> {
        (0..lines)
            .map(|i| DiffLine {
                kind: DiffLineKind::Context,
                content: format!("line {}", i),
            })
            .collect()
    }

    #[test]
    fn test_get_cloned_miss_returns_none() {
        let mut cache = DiffCache::new();
        let key = DiffCacheKey::File {
            path: "foo.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        assert!(cache.get_cloned(&key).is_none());
    }

    #[test]
    fn test_insert_and_get_cloned() {
        let mut cache = DiffCache::new();
        let key = DiffCacheKey::File {
            path: "foo.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        let diff = make_diff(3);
        cache.insert(key.clone(), diff.clone());
        let result = cache.get_cloned(&key).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_insert_updates_existing() {
        let mut cache = DiffCache::new();
        let key = DiffCacheKey::File {
            path: "foo.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        cache.insert(key.clone(), make_diff(2));
        cache.insert(key.clone(), make_diff(5));
        let result = cache.get_cloned(&key).unwrap();
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_invalidate_files_removes_file_entries() {
        let mut cache = DiffCache::new();
        let file_key = DiffCacheKey::File {
            path: "foo.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        let commit_key = DiffCacheKey::Commit {
            oid: "abc".to_string(),
            path: None,
        }
        .at_revision(0);
        cache.insert(file_key.clone(), make_diff(1));
        cache.insert(commit_key.clone(), make_diff(1));
        cache.invalidate_files();
        assert!(cache.get_cloned(&file_key).is_none());
        assert!(cache.get_cloned(&commit_key).is_some());
    }

    #[test]
    fn test_evict_lru_on_overflow() {
        let mut cache = DiffCache::with_limits(2, MAX_BYTES);
        let k1 = DiffCacheKey::File {
            path: "a.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        let k2 = DiffCacheKey::File {
            path: "b.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        let k3 = DiffCacheKey::File {
            path: "c.txt".into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0);
        cache.insert(k1.clone(), make_diff(1));
        cache.insert(k2.clone(), make_diff(1));
        cache.insert(k3.clone(), make_diff(1)); // should evict k1
        assert!(cache.get_cloned(&k1).is_none());
        assert!(cache.get_cloned(&k2).is_some());
        assert!(cache.get_cloned(&k3).is_some());
    }

    #[test]
    fn test_stash_and_branch_keys() {
        let mut cache = DiffCache::new();
        let stash_key = DiffCacheKey::Stash {
            index: 0,
            path: None,
        }
        .at_revision(0);
        let branch_key = DiffCacheKey::Branch {
            name: "main".to_string(),
            limit: 100,
        }
        .at_revision(0);
        cache.insert(stash_key.clone(), make_diff(2));
        cache.insert(branch_key.clone(), make_diff(3));
        assert_eq!(cache.get_cloned(&stash_key).unwrap().len(), 2);
        assert_eq!(cache.get_cloned(&branch_key).unwrap().len(), 3);
    }

    #[test]
    fn test_directory_key() {
        let mut cache = DiffCache::new();
        let key = DiffCacheKey::Directory {
            path: "src/".into(),
            files_hash: 42,
        }
        .at_revision(0);
        cache.insert(key.clone(), make_diff(4));
        assert_eq!(cache.get_cloned(&key).unwrap().len(), 4);
    }
    #[test]
    fn test_ref_refresh_invalidates_stash_indices_but_keeps_commit_patches() {
        let mut cache = DiffCache::new();
        let stash = DiffCacheKey::Stash {
            index: 0,
            path: None,
        }
        .at_revision(0);
        let branch = DiffCacheKey::Branch {
            name: "feature".into(),
            limit: 20,
        }
        .at_revision(0);
        let commit = DiffCacheKey::Commit {
            oid: "immutable".into(),
            path: None,
        }
        .at_revision(0);
        for key in [&stash, &branch, &commit] {
            cache.insert(key.clone(), Vec::<DiffLine>::new());
        }
        cache.invalidate_refs();
        assert!(cache.get_cloned(&stash).is_none());
        assert!(cache.get_cloned(&branch).is_none());
        assert!(cache.get_cloned(&commit).is_some());
    }

    fn file(name: &str) -> DiffCacheIdentity {
        DiffCacheKey::File {
            path: name.into(),
            source: FileDiffSource::Worktree,
        }
        .at_revision(0)
    }

    fn text(bytes: usize) -> DiffDocument {
        vec![DiffLine {
            kind: DiffLineKind::Context,
            content: "x".repeat(bytes),
        }]
        .into()
    }

    #[test]
    fn test_byte_budget_evicts_least_recently_used_below_entry_limit() {
        let mut cache = DiffCache::with_limits(50, 800);
        cache.insert(file("a"), text(256));
        cache.insert(file("b"), text(256));
        assert!(cache.get_cloned(&file("a")).is_some());
        cache.insert(file("c"), text(256));
        assert!(cache.get_cloned(&file("b")).is_none());
        assert!(cache.get_cloned(&file("a")).is_some());
        assert!(cache.get_cloned(&file("c")).is_some());
        let stats = cache.stats();
        assert_eq!(stats.entries, 2);
        assert_eq!(stats.evictions, 1);
        assert_eq!(stats.hits, 3);
        assert_eq!(stats.misses, 1);
        assert!(stats.retained_bytes <= stats.max_bytes);
    }

    #[test]
    fn test_growing_replacement_evicts_other_entries_and_shrinking_releases_bytes() {
        let mut cache = DiffCache::with_limits(50, 800);
        cache.insert(file("a"), text(256));
        cache.insert(file("b"), text(256));
        cache.insert(file("a"), text(600));
        assert!(cache.get_cloned(&file("b")).is_none());
        assert_eq!(cache.get_cloned(&file("a")).unwrap()[0].content.len(), 600);
        let grown = cache.stats().retained_bytes;
        cache.insert(file("a"), text(10));
        assert!(cache.stats().retained_bytes < grown);
        assert_eq!(cache.stats().evictions, 1);
        cache.invalidate_files();
        assert_eq!(cache.stats().retained_bytes, 0);
        assert!(cache.payloads.is_empty());
        assert!(cache.usage_order.is_empty());
    }

    #[test]
    fn test_oversized_replacement_removes_old_value_without_evicting_other_keys() {
        let mut cache = DiffCache::with_limits(50, 800);
        cache.insert(file("a"), text(128));
        cache.insert(file("b"), text(128));
        let retained = cache.get_cloned(&file("a")).unwrap();
        cache.insert(file("a"), text(4096));
        assert!(cache.get_cloned(&file("a")).is_none());
        assert!(cache.get_cloned(&file("b")).is_some());
        assert_eq!(cache.stats().oversized, 1);
        assert_eq!(cache.stats().evictions, 0);
        assert_eq!(retained[0].content.len(), 128);
        let mut reserved = String::with_capacity(4096);
        reserved.push('x');
        cache.insert(
            file("reserved"),
            vec![DiffLine {
                kind: DiffLineKind::Context,
                content: reserved,
            }],
        );
        assert!(cache.get_cloned(&file("reserved")).is_none());
        assert_eq!(cache.stats().oversized, 2);
    }

    #[test]
    fn test_shared_payloads_are_counted_once_until_last_cache_reference_is_removed() {
        use super::super::tests::{document, patch};
        use crate::git::patch::PatchSource;
        let mut cache = DiffCache::new();
        let original = document(patch(&[PatchSource::Worktree]));
        let commit = DiffCacheKey::Commit {
            oid: "a".repeat(40),
            path: None,
        }
        .at_revision(0);
        cache.insert(file("a"), original.clone());
        let bytes = cache.stats().retained_bytes;
        cache.insert(commit.clone(), original.clone());
        assert_eq!(cache.stats().retained_bytes, bytes);
        cache.invalidate_files();
        assert_eq!(cache.stats().retained_bytes, bytes);
        assert_eq!(cache.stats().invalidations, 1);
        let mut detached = cache.get_cloned(&commit).unwrap();
        detached.lines[0].content = "detached".into();
        assert_ne!(original.lines[0].content, "detached");
        assert_eq!(cache.stats().retained_bytes, bytes);
        assert!(cache.remove(&commit));
        assert_eq!(cache.stats().retained_bytes, 0);
        assert!(!original.lines.is_empty());
        assert!(Arc::ptr_eq(
            original.patch.as_ref().unwrap(),
            detached.patch.as_ref().unwrap()
        ));
    }

    #[test]
    fn test_partially_shared_payloads_and_empty_vecs_have_distinct_accounting() {
        use super::super::tests::{document, patch};
        use crate::git::patch::PatchSource;
        let mut cache = DiffCache::new();
        let original = document(patch(&[PatchSource::Worktree]));
        cache.insert(file("a"), original.clone());
        let first = cache.stats().retained_bytes;
        let additional = text(200);
        let mut text_cache = DiffCache::new();
        text_cache.insert(file("text"), additional.clone());
        cache.insert(
            file("b"),
            DiffDocument {
                lines: additional.lines,
                patch: original.patch.clone(),
            },
        );
        let combined = cache.stats().retained_bytes;
        assert_eq!(combined, first + text_cache.stats().retained_bytes);
        cache.remove(&file("a"));
        assert!(cache.stats().retained_bytes < combined);
        assert!(cache.get_cloned(&file("b")).unwrap().patch.is_some());
        cache.invalidate_files();
        cache.insert(file("empty-a"), Vec::<DiffLine>::new());
        let empty = cache.stats().retained_bytes;
        cache.insert(file("empty-b"), Vec::<DiffLine>::new());
        assert_eq!(cache.stats().retained_bytes, empty * 2);
    }

    #[test]
    fn test_lossless_patch_buffers_and_vector_capacity_contribute_to_budget() {
        use super::super::tests::{document, patch};
        use crate::git::patch::PatchSource;
        for component in 0..6 {
            let mut original = patch(&[PatchSource::Worktree]);
            let mut cache = DiffCache::new();
            cache.insert(file("before"), document(original.clone()));
            let before = cache.stats().retained_bytes;
            cache.invalidate_files();
            let owned = Arc::make_mut(&mut original);
            match component {
                0 => owned.files[0].encoded.as_mut().unwrap().reserve(8192),
                1 => owned.files[0].hunks[0].header.reserve(8192),
                2 => owned.files[0].hunks[0].lines[0].content.reserve(8192),
                3 => owned.files.reserve(8192),
                4 => owned.files[0].hunks.reserve(8192),
                _ => owned.files[0].hunks[0].lines.reserve(8192),
            }
            cache.insert(file("after"), document(original));
            assert!(
                cache.stats().retained_bytes >= before + 8000,
                "component {component}"
            );
        }
    }

    #[test]
    fn test_exact_budget_and_zero_limits_terminate_without_losing_accounting() {
        let diff = text(200);
        let mut measured = DiffCache::new();
        measured.insert(file("a"), diff.clone());
        let bytes = measured.stats().retained_bytes;
        let mut exact = DiffCache::with_limits(1, bytes);
        exact.insert(file("a"), diff.clone());
        exact.insert(file("b"), diff.clone());
        assert_eq!(exact.stats().entries, 1);
        assert_eq!(exact.stats().retained_bytes, bytes);
        for (entries, budget) in [(0, bytes), (50, 0), (50, bytes - 1)] {
            let mut disabled = DiffCache::with_limits(entries, budget);
            disabled.insert(file("a"), diff.clone());
            assert_eq!(disabled.stats().entries, 0);
            assert_eq!(disabled.stats().retained_bytes, 0);
        }
    }

    #[test]
    fn test_scoped_reference_invalidation_releases_bytes_and_preserves_survivor_lru() {
        let mut cache = DiffCache::with_limits(3, MAX_BYTES);
        let branch = DiffCacheKey::Branch {
            name: "branch".into(),
            limit: 20,
        }
        .at_revision(0);
        let stash = DiffCacheKey::Stash {
            index: 0,
            path: None,
        }
        .at_revision(0);
        let commit = DiffCacheKey::Commit {
            oid: "a".repeat(40),
            path: None,
        }
        .at_revision(0);
        cache.insert(branch.clone(), text(128));
        cache.insert(stash.clone(), text(128));
        cache.insert(commit.clone(), text(128));
        let bytes = cache.stats().retained_bytes;
        cache.get_cloned(&stash).unwrap();
        cache.invalidate_branches();
        assert_eq!(cache.stats().entries, 2);
        assert!(cache.stats().retained_bytes < bytes);
        assert!(cache.get_cloned(&branch).is_none());
        cache.insert(file("new"), text(128));
        cache.insert(branch.clone(), text(128));
        assert!(
            cache.get_cloned(&commit).is_none(),
            "invalidation must preserve survivor recency"
        );
        assert!(cache.get_cloned(&stash).is_some());
        cache.invalidate_stashes();
        assert!(cache.get_cloned(&stash).is_none());
        assert!(cache.get_cloned(&branch).is_some());
        assert!(cache.get_cloned(&file("new")).is_some());
        assert_eq!(cache.stats().invalidations, 2);
    }
}
