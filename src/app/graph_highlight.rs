use crate::git::CommitInfo;
use std::collections::{HashMap, HashSet};

/// Walk ancestry chains from the selected commit to compute a set of highlighted OIDs.
/// Ancestors: follow first-parent chain from selected toward older commits.
/// Descendants: pick a single first-parent child chain toward newer commits.
pub fn compute_highlight_set(commits: &[CommitInfo], selected_oid: &str) -> HashSet<String> {
    let mut highlighted = HashSet::new();
    if commits.is_empty() {
        return highlighted;
    }

    // Build OID → index map for O(1) lookup.
    let oid_to_idx: HashMap<&str, usize> = commits
        .iter()
        .enumerate()
        .map(|(i, c)| (c.oid.as_str(), i))
        .collect();

    let selected_idx = match oid_to_idx.get(selected_oid) {
        Some(&i) => i,
        None => return highlighted,
    };

    // Include the selected commit itself.
    highlighted.insert(selected_oid.to_string());

    // Walk ancestors via first-parent chain.
    let mut cur_oid = selected_oid;
    while let Some(&idx) = oid_to_idx.get(cur_oid) {
        let Some(first_parent) = commits[idx].parent_oids.first().map(String::as_str) else {
            break;
        };
        if !oid_to_idx.contains_key(first_parent) {
            break;
        }
        highlighted.insert(first_parent.to_string());
        cur_oid = first_parent;
    }

    // Build reverse first-parent map: OID -> list of OIDs that have it as first parent.
    let mut fp_children: HashMap<&str, Vec<&str>> = HashMap::new();
    for commit in commits {
        if let Some(fp) = commit.parent_oids.first() {
            fp_children
                .entry(fp.as_str())
                .or_default()
                .push(commit.oid.as_str());
        }
    }

    let lane_by_oid = build_lane_map(commits);

    // Follow a single descendant chain. When multiple children overlap in time,
    // use a stable tie-break: same lane first, then nearest lane distance, then nearest row.
    let mut cur_oid = selected_oid.to_string();
    let mut cur_idx = selected_idx;
    loop {
        let Some(children) = fp_children.get(cur_oid.as_str()) else {
            break;
        };

        let cur_lane = lane_by_oid.get(cur_oid.as_str()).copied();
        let mut candidates: Vec<(&str, usize)> = children
            .iter()
            .filter_map(|child| {
                oid_to_idx
                    .get(*child)
                    .copied()
                    .filter(|&idx| idx < cur_idx)
                    .map(|idx| (*child, idx))
            })
            .collect();
        if candidates.is_empty() {
            break;
        }

        candidates.sort_by(|a, b| {
            let a_lane = lane_by_oid.get(a.0).copied();
            let b_lane = lane_by_oid.get(b.0).copied();

            let a_same_lane = matches!((cur_lane, a_lane), (Some(c), Some(l)) if c == l);
            let b_same_lane = matches!((cur_lane, b_lane), (Some(c), Some(l)) if c == l);
            // true should sort before false.
            if a_same_lane != b_same_lane {
                return b_same_lane.cmp(&a_same_lane);
            }

            let a_lane_dist = lane_distance(cur_lane, a_lane);
            let b_lane_dist = lane_distance(cur_lane, b_lane);
            if a_lane_dist != b_lane_dist {
                return a_lane_dist.cmp(&b_lane_dist);
            }

            let a_row_dist = cur_idx.saturating_sub(a.1);
            let b_row_dist = cur_idx.saturating_sub(b.1);
            if a_row_dist != b_row_dist {
                return a_row_dist.cmp(&b_row_dist);
            }

            a.0.cmp(b.0)
        });

        let (next_oid, next_idx) = candidates[0];
        if !highlighted.insert(next_oid.to_string()) {
            break;
        }
        cur_oid = next_oid.to_string();
        cur_idx = next_idx;
    }

    highlighted
}

fn lane_distance(lhs: Option<usize>, rhs: Option<usize>) -> usize {
    match (lhs, rhs) {
        (Some(a), Some(b)) => a.abs_diff(b),
        _ => usize::MAX / 2,
    }
}

fn build_lane_map(commits: &[CommitInfo]) -> HashMap<&str, usize> {
    let mut lanes = HashMap::new();
    for commit in commits {
        if let Some(lane) = commit_lane(commit) {
            lanes.insert(commit.oid.as_str(), lane);
        }
    }
    lanes
}

fn commit_lane(commit: &CommitInfo) -> Option<usize> {
    // Prefer a node cell that belongs to this commit.
    if let Some(cell) = commit
        .graph
        .iter()
        .find(|cell| is_node_cell(cell.text.as_str()) && cell_has_owner(cell, commit.oid.as_str()))
    {
        return Some(cell.lane);
    }

    // Fallback to any owned graph cell if node markers are not available.
    commit
        .graph
        .iter()
        .find(|cell| cell_has_owner(cell, commit.oid.as_str()))
        .map(|cell| cell.lane)
}

fn is_node_cell(text: &str) -> bool {
    matches!(text, "◯" | "⏣" | "*" | "#" | "●")
}

fn cell_has_owner(cell: &crate::git::GraphCell, oid: &str) -> bool {
    cell.pipe_oids.iter().any(|owner| owner == oid) || cell.pipe_oid.as_deref() == Some(oid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::{CommitInfo, CommitSyncState, GraphCell};

    fn make_commit(oid: &str, parents: &[&str]) -> CommitInfo {
        make_commit_with_lane(oid, parents, 0)
    }

    fn make_commit_with_lane(oid: &str, parents: &[&str], lane: usize) -> CommitInfo {
        CommitInfo {
            short_hash: oid[..7.min(oid.len())].to_string(),
            oid: oid.to_string(),
            message: format!("commit {}", oid),
            author: "test".to_string(),
            graph: vec![GraphCell {
                text: "◯".to_string(),
                lane,
                pipe_oid: Some(oid.to_string()),
                pipe_oids: vec![oid.to_string()],
            }],
            time: "2026-01-01 00:00".to_string(),
            parent_count: parents.len(),
            sync_state: CommitSyncState::DefaultBranch,
            parent_oids: parents.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn test_linear_ancestry() {
        // newest-first: aaa -> bbb -> ccc
        let commits = vec![
            make_commit("aaa", &["bbb"]),
            make_commit("bbb", &["ccc"]),
            make_commit("ccc", &[]),
        ];
        let result = compute_highlight_set(&commits, "bbb");
        assert!(result.contains("bbb"));
        assert!(result.contains("aaa"), "aaa should be a descendant");
        assert!(result.contains("ccc"), "ccc should be an ancestor");
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_single_commit() {
        let commits = vec![make_commit("aaa", &[])];
        let result = compute_highlight_set(&commits, "aaa");
        assert!(result.contains("aaa"));
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_unknown_oid() {
        let commits = vec![make_commit("aaa", &[])];
        let result = compute_highlight_set(&commits, "zzz");
        assert!(result.is_empty());
    }

    #[test]
    fn test_branch_off_main_uses_single_descendant_path() {
        // newest-first: main: m1->m2->m3, feature: f1->m2
        // m1, f1 both descend from m2
        let commits = vec![
            make_commit("m1", &["m2"]),
            make_commit("f1", &["m2"]),
            make_commit("m2", &["m3"]),
            make_commit("m3", &[]),
        ];
        // Select m2: ancestors = m3, descendants choose one chain only.
        let result = compute_highlight_set(&commits, "m2");
        assert!(result.contains("m2"));
        assert!(result.contains("m3"));
        assert!(result.contains("f1"));
        assert!(!result.contains("m1"));
    }

    #[test]
    fn test_descendant_tie_break_prefers_same_lane() {
        // newest-first:
        // c_same -> m
        // c_near -> m
        // m -> base
        // base
        let commits = vec![
            make_commit_with_lane("c_same", &["m"], 4),
            make_commit_with_lane("c_near", &["m"], 1),
            make_commit_with_lane("m", &["base"], 4),
            make_commit_with_lane("base", &[], 4),
        ];
        let result = compute_highlight_set(&commits, "m");
        assert!(result.contains("m"));
        assert!(result.contains("base"));
        assert!(result.contains("c_same"), "same-lane child should win");
        assert!(!result.contains("c_near"));
    }

    #[test]
    fn test_merge_commit_ancestors() {
        // merge: m has parents [a, b]; newest-first: m, a, b, base
        let commits = vec![
            make_commit("m", &["a", "b"]),
            make_commit("a", &["base"]),
            make_commit("b", &["base"]),
            make_commit("base", &[]),
        ];
        // Select m: first-parent ancestor chain = a, base
        let result = compute_highlight_set(&commits, "m");
        assert!(result.contains("m"));
        assert!(result.contains("a"), "first parent a should be highlighted");
        assert!(result.contains("base"), "base via first-parent chain");
        // b is NOT on first-parent chain
        assert!(
            !result.contains("b"),
            "b is second parent, should not be highlighted"
        );
    }

    #[test]
    fn test_select_oldest_no_descendants() {
        let commits = vec![
            make_commit("a", &["b"]),
            make_commit("b", &["c"]),
            make_commit("c", &[]),
        ];
        let result = compute_highlight_set(&commits, "c");
        assert!(result.contains("c"));
        // a and b are descendants
        assert!(result.contains("b"));
        assert!(result.contains("a"));
    }

    #[test]
    fn test_select_newest_no_ancestors_out_of_range() {
        // Only one commit visible in list; parent not in list
        let commits = vec![make_commit("a", &["outside"])];
        let result = compute_highlight_set(&commits, "a");
        assert!(result.contains("a"));
        assert_eq!(result.len(), 1);
    }
}
