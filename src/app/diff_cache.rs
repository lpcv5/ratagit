use crate::git::DiffLine;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiffCacheKey {
    File { path: PathBuf, is_staged: bool },
    Directory { path: PathBuf, files_hash: u64 },
    Commit { oid: String, path: Option<PathBuf> },
    Stash { index: usize, path: Option<PathBuf> },
}

pub struct DiffCache {
    cache: HashMap<DiffCacheKey, Vec<DiffLine>>,
    max_entries: usize,
}

impl DiffCache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            max_entries: 50,
        }
    }

    pub fn get(&self, key: &DiffCacheKey) -> Option<&Vec<DiffLine>> {
        self.cache.get(key)
    }

    pub fn insert(&mut self, key: DiffCacheKey, diff: Vec<DiffLine>) {
        if self.cache.len() >= self.max_entries {
            self.cache.clear();
        }
        self.cache.insert(key, diff);
    }

    pub fn invalidate_files(&mut self) {
        self.cache
            .retain(|k, _| matches!(k, DiffCacheKey::Commit { .. } | DiffCacheKey::Stash { .. }));
    }
}
