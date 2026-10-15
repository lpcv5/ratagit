//! AppContext-owned caches with recency and owned-capacity budgets.
use crate::{
    AppContext, CachedBranchLog, CachedCommitDiff, CachedFilesDiff, DETAILS_DIFF_CACHE_LIMIT,
};
const CACHE_BYTES_PER_KIND: usize = 8 * 1024 * 1024;
trait CacheCost {
    fn bytes(&self) -> usize;
}
impl CacheCost for CachedFilesDiff {
    fn bytes(&self) -> usize {
        self.diff.capacity()
            + self.paths.capacity() * std::mem::size_of::<String>()
            + self.paths.iter().map(String::capacity).sum::<usize>()
    }
}
impl CacheCost for CachedBranchLog {
    fn bytes(&self) -> usize {
        self.log.capacity() + self.branch.capacity()
    }
}
impl CacheCost for CachedCommitDiff {
    fn bytes(&self) -> usize {
        self.diff.capacity() + self.commit_id.capacity()
    }
}

pub(crate) fn cached_files_diff(state: &mut AppContext, paths: &[String]) -> Option<String> {
    cached_entry(
        &mut state.repo.details.cached_files_diffs,
        paths,
        |entry, paths| entry.paths.as_slice() == paths,
        |entry| &entry.diff,
    )
}

pub(crate) fn cache_files_diff(state: &mut AppContext, paths: &[String], diff: &str) {
    cache_entry(
        &mut state.repo.details.cached_files_diffs,
        paths,
        diff,
        |entry, paths| entry.paths.as_slice() == paths,
        |paths, diff| CachedFilesDiff {
            paths: paths.to_vec(),
            diff: diff.to_string(),
        },
    );
}

pub(crate) fn cached_branch_log(state: &mut AppContext, branch: &str) -> Option<String> {
    cached_entry(
        &mut state.repo.details.cached_branch_logs,
        branch,
        |entry, branch| entry.branch.as_str() == branch,
        |entry| &entry.log,
    )
}

pub(crate) fn cache_branch_log(state: &mut AppContext, branch: &str, log: &str) {
    cache_entry(
        &mut state.repo.details.cached_branch_logs,
        branch,
        log,
        |entry, branch| entry.branch.as_str() == branch,
        |branch, log| CachedBranchLog {
            branch: branch.to_string(),
            log: log.to_string(),
        },
    );
}

pub(crate) fn cached_commit_diff(state: &mut AppContext, commit_id: &str) -> Option<String> {
    cached_entry(
        &mut state.repo.details.cached_commit_diffs,
        commit_id,
        |entry, commit_id| entry.commit_id.as_str() == commit_id,
        |entry| &entry.diff,
    )
}

pub(crate) fn cache_commit_diff(state: &mut AppContext, commit_id: &str, diff: &str) {
    cache_entry(
        &mut state.repo.details.cached_commit_diffs,
        commit_id,
        diff,
        |entry, commit_id| entry.commit_id.as_str() == commit_id,
        |commit_id, diff| CachedCommitDiff {
            commit_id: commit_id.to_string(),
            diff: diff.to_string(),
        },
    );
}

fn cached_entry<K: ?Sized, E>(
    entries: &mut Vec<E>,
    key: &K,
    mut matches_key: impl FnMut(&E, &K) -> bool,
    value: impl Fn(&E) -> &str,
) -> Option<String> {
    let index = entries.iter().position(|entry| matches_key(entry, key))?;
    let entry = entries.remove(index);
    let result = value(&entry).to_string();
    entries.insert(0, entry);
    Some(result)
}

fn cache_entry<K: ?Sized, E: CacheCost>(
    entries: &mut Vec<E>,
    key: &K,
    value: &str,
    mut matches_key: impl FnMut(&E, &K) -> bool,
    build: impl FnOnce(&K, &str) -> E,
) {
    entries.retain(|entry| !matches_key(entry, key));
    let entry = build(key, value);
    if entry.bytes() > CACHE_BYTES_PER_KIND {
        return;
    }
    entries.insert(0, entry);
    entries.truncate(DETAILS_DIFF_CACHE_LIMIT);
    while entries.iter().map(CacheCost::bytes).sum::<usize>() > CACHE_BYTES_PER_KIND {
        entries.pop();
    }
}
#[cfg(test)]
mod tests;
