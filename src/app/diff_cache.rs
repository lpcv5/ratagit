use crate::git::DiffLine;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiffCacheKey {
    File { path: PathBuf, is_staged: bool },
    Branch { name: String, limit: usize },
    Directory { path: PathBuf, files_hash: u64 },
    Commit { oid: String, path: Option<PathBuf> },
    Stash { index: usize, path: Option<PathBuf> },
}

pub struct DiffCache {
    cache: HashMap<DiffCacheKey, Vec<DiffLine>>,
    usage_order: VecDeque<DiffCacheKey>,
    max_entries: usize,
}

impl DiffCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            usage_order: VecDeque::new(),
            max_entries: 50,
        }
    }

    pub fn get_cloned(&mut self, key: &DiffCacheKey) -> Option<Vec<DiffLine>> {
        let diff = self.cache.get(key)?.clone();
        self.touch(key);
        Some(diff)
    }

    pub fn insert(&mut self, key: DiffCacheKey, diff: Vec<DiffLine>) {
        if self.cache.contains_key(&key) {
            self.cache.insert(key.clone(), diff);
            self.touch(&key);
            return;
        }

        if self.cache.len() >= self.max_entries {
            self.evict_lru();
        }

        self.cache.insert(key.clone(), diff);
        self.usage_order.push_back(key);
    }

    pub fn invalidate_files(&mut self) {
        self.cache.retain(|k, _| {
            matches!(
                k,
                DiffCacheKey::Commit { .. }
                    | DiffCacheKey::Stash { .. }
                    | DiffCacheKey::Branch { .. }
            )
        });
        self.usage_order.retain(|k| self.cache.contains_key(k));
    }

    fn touch(&mut self, key: &DiffCacheKey) {
        self.usage_order.retain(|existing| existing != key);
        self.usage_order.push_back(key.clone());
    }

    fn evict_lru(&mut self) {
        while let Some(oldest) = self.usage_order.pop_front() {
            if self.cache.remove(&oldest).is_some() {
                break;
            }
        }
    }
}
