use super::diff_cache::{DiffCache, DiffCacheKey};

pub(super) struct DiffCacheManager {
    pub(super) cache: DiffCache,
    pub(super) last_diff_key: Option<DiffCacheKey>,
    pub(super) in_flight_diff_key: Option<DiffCacheKey>,
}

impl DiffCacheManager {
    pub(super) fn new() -> Self {
        Self {
            cache: DiffCache::new(),
            last_diff_key: None,
            in_flight_diff_key: None,
        }
    }
}
