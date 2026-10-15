use super::*;

#[test]
fn test_byte_budget_and_oversized_replacement() {
    let mut state = AppContext::default();
    let text = "x".repeat(CACHE_BYTES_PER_KIND / 3);
    for key in ["a", "b", "c", "d"] {
        cache_commit_diff(&mut state, key, &text);
    }
    assert!(
        state
            .repo
            .details
            .cached_commit_diffs
            .iter()
            .map(CacheCost::bytes)
            .sum::<usize>()
            <= CACHE_BYTES_PER_KIND
    );
    assert!(cached_commit_diff(&mut state, "a").is_none());
    cache_commit_diff(&mut state, "d", &"x".repeat(CACHE_BYTES_PER_KIND + 1));
    assert!(cached_commit_diff(&mut state, "d").is_none());
    assert!(cached_commit_diff(&mut state, "c").is_some());
}

#[test]
fn test_cache_hit_promotes_recency_before_eviction() {
    let mut state = AppContext::default();
    for key in 0..DETAILS_DIFF_CACHE_LIMIT {
        cache_commit_diff(&mut state, &key.to_string(), "diff");
    }
    assert!(cached_commit_diff(&mut state, "0").is_some());
    cache_commit_diff(&mut state, "new", "diff");
    assert!(cached_commit_diff(&mut state, "0").is_some());
    assert!(cached_commit_diff(&mut state, "1").is_none());
}
