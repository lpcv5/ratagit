use crate::git::DiffLine;
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DiffCacheKey {
    /// Represents a missing or empty diff target (no selection).
    None,
    File {
        path: PathBuf,
        is_staged: bool,
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
                DiffCacheKey::None
                    | DiffCacheKey::Commit { .. }
                    | DiffCacheKey::Stash { .. }
                    | DiffCacheKey::Branch { .. }
            )
        });
        self.usage_order.retain(|k| self.cache.contains_key(k));
    }

    pub fn invalidate_branches(&mut self) {
        self.cache
            .retain(|k, _| !matches!(k, DiffCacheKey::Branch { .. }));
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
            is_staged: false,
        };
        assert!(cache.get_cloned(&key).is_none());
    }

    #[test]
    fn test_insert_and_get_cloned() {
        let mut cache = DiffCache::new();
        let key = DiffCacheKey::File {
            path: "foo.txt".into(),
            is_staged: false,
        };
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
            is_staged: false,
        };
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
            is_staged: false,
        };
        let commit_key = DiffCacheKey::Commit {
            oid: "abc".to_string(),
            path: None,
        };
        cache.insert(file_key.clone(), make_diff(1));
        cache.insert(commit_key.clone(), make_diff(1));
        cache.invalidate_files();
        assert!(cache.get_cloned(&file_key).is_none());
        assert!(cache.get_cloned(&commit_key).is_some());
    }

    #[test]
    fn test_invalidate_branches_removes_branch_entries_only() {
        let mut cache = DiffCache::new();
        let branch_key = DiffCacheKey::Branch {
            name: "main".into(),
            limit: 20,
        };
        let commit_key = DiffCacheKey::Commit {
            oid: "abc".to_string(),
            path: None,
        };
        cache.insert(branch_key.clone(), make_diff(1));
        cache.insert(commit_key.clone(), make_diff(1));

        cache.invalidate_branches();

        assert!(cache.get_cloned(&branch_key).is_none());
        assert!(cache.get_cloned(&commit_key).is_some());
    }

    #[test]
    fn test_evict_lru_on_overflow() {
        let mut cache = DiffCache {
            cache: std::collections::HashMap::new(),
            usage_order: std::collections::VecDeque::new(),
            max_entries: 2,
        };
        let k1 = DiffCacheKey::File {
            path: "a.txt".into(),
            is_staged: false,
        };
        let k2 = DiffCacheKey::File {
            path: "b.txt".into(),
            is_staged: false,
        };
        let k3 = DiffCacheKey::File {
            path: "c.txt".into(),
            is_staged: false,
        };
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
        };
        let branch_key = DiffCacheKey::Branch {
            name: "main".to_string(),
            limit: 100,
        };
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
        };
        cache.insert(key.clone(), make_diff(4));
        assert_eq!(cache.get_cloned(&key).unwrap().len(), 4);
    }
}
