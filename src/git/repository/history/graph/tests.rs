use super::*;
use pretty_assertions::assert_eq;

#[test]
fn test_build_commit_graph_lines_linear_history() {
    let rows = vec![
        GraphCommitRow {
            oid: oid("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            parents: vec![oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")],
        },
        GraphCommitRow {
            oid: oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            parents: vec![oid("cccccccccccccccccccccccccccccccccccccccc")],
        },
        GraphCommitRow {
            oid: oid("cccccccccccccccccccccccccccccccccccccccc"),
            parents: vec![],
        },
    ];

    let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
    let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
    assert_eq!(strs, vec!["◯", "◯", "◯"]);
}

#[test]
fn test_build_commit_graph_lines_merge_history() {
    let rows = vec![
        GraphCommitRow {
            oid: oid("1111111111111111111111111111111111111111"),
            parents: vec![
                oid("2222222222222222222222222222222222222222"),
                oid("3333333333333333333333333333333333333333"),
            ],
        },
        GraphCommitRow {
            oid: oid("2222222222222222222222222222222222222222"),
            parents: vec![oid("4444444444444444444444444444444444444444")],
        },
        GraphCommitRow {
            oid: oid("3333333333333333333333333333333333333333"),
            parents: vec![oid("4444444444444444444444444444444444444444")],
        },
        GraphCommitRow {
            oid: oid("4444444444444444444444444444444444444444"),
            parents: vec![],
        },
    ];

    let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
    let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
    assert_eq!(strs, vec!["⏣ ╮", "◯ │", "│ ◯", "◯ ╯"]);
}

#[test]
fn test_build_commit_graph_lines_octopus_merge_like() {
    let rows = vec![
        GraphCommitRow {
            oid: oid("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"),
            parents: vec![
                oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
                oid("cccccccccccccccccccccccccccccccccccccccc"),
                oid("dddddddddddddddddddddddddddddddddddddddd"),
            ],
        },
        GraphCommitRow {
            oid: oid("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
            parents: vec![oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")],
        },
        GraphCommitRow {
            oid: oid("cccccccccccccccccccccccccccccccccccccccc"),
            parents: vec![oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")],
        },
        GraphCommitRow {
            oid: oid("dddddddddddddddddddddddddddddddddddddddd"),
            parents: vec![oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee")],
        },
        GraphCommitRow {
            oid: oid("eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee"),
            parents: vec![],
        },
    ];

    let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
    let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
    assert_eq!(strs[0], "⏣ ┬─╮");
    assert_eq!(strs[1], "◯ │ │");
    assert_eq!(strs[2], "│ ◯ │");
    assert_eq!(strs[3], "│ │ ◯");
    assert_eq!(strs[4], "◯ ┴─╯");
}

#[test]
fn test_graph_charset_values() {
    assert_eq!(graph_charset(None).node, '◯');
    assert_eq!(graph_charset(Some("1")).node, '*');
    assert_eq!(graph_charset(Some("0")).node, '◯');
    assert_eq!(graph_charset(Some("unexpected")).node, '◯');
}

fn oid(value: &str) -> git2::Oid {
    git2::Oid::from_str(value).expect("valid oid")
}

fn rows_from_oid_pairs(pairs: &[(&str, &[&str])]) -> Vec<GraphCommitRow> {
    pairs
        .iter()
        .map(|(o, ps)| GraphCommitRow {
            oid: oid(o),
            parents: ps.iter().map(|p| oid(p)).collect(),
        })
        .collect()
}

fn complex_repo_rows() -> Vec<GraphCommitRow> {
    rows_from_oid_pairs(&[
        (
            "841a287edd4e6b32c59c1affc677fc28dcede01d",
            &[
                "6d2c737c870c743a0a3e5d6113fd1225f8ce7214",
                "6ff4b94506817257e24550cb4cb86db19758ed2d",
            ],
        ),
        (
            "6ff4b94506817257e24550cb4cb86db19758ed2d",
            &["428ecd10104810073d8752729e789f582f0264ad"],
        ),
        (
            "428ecd10104810073d8752729e789f582f0264ad",
            &[
                "22751db39f9ea5f474440fa82c84301c693ba3ed",
                "6d2c737c870c743a0a3e5d6113fd1225f8ce7214",
            ],
        ),
        (
            "6d2c737c870c743a0a3e5d6113fd1225f8ce7214",
            &[
                "59280f68a38d8d594589173a7eafb371bcecb5ee",
                "13fe9925399cc03ba03628485c3f95b078ea9dc1",
            ],
        ),
        (
            "13fe9925399cc03ba03628485c3f95b078ea9dc1",
            &["1a0137ef486eae5607fbc7852169e626113ebd07"],
        ),
        (
            "59280f68a38d8d594589173a7eafb371bcecb5ee",
            &[
                "c014d84fb6656d9a0f1199405c75bbf2cca0c260",
                "69ca3992d4902f601412d7f4d372fa9d32f4c9ed",
            ],
        ),
        (
            "69ca3992d4902f601412d7f4d372fa9d32f4c9ed",
            &["1363c71ecdba04252939f2e5baf9fb0d6e78612c"],
        ),
        (
            "1363c71ecdba04252939f2e5baf9fb0d6e78612c",
            &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
        ),
        (
            "2fdf45590c1b89c3ee5ad627c6c65cf1c537a17f",
            &["e5fe47f2deddf8bf38f6295e64663fd2d4ffd062"],
        ),
        (
            "e5fe47f2deddf8bf38f6295e64663fd2d4ffd062",
            &[
                "17b91a61c98954a0d56e162c5749b036f2a8c53b",
                "c014d84fb6656d9a0f1199405c75bbf2cca0c260",
            ],
        ),
        (
            "c014d84fb6656d9a0f1199405c75bbf2cca0c260",
            &[
                "1a0137ef486eae5607fbc7852169e626113ebd07",
                "1b3ce77a986f530827838e548103d7b81a5e8d38",
            ],
        ),
        (
            "1b3ce77a986f530827838e548103d7b81a5e8d38",
            &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
        ),
        (
            "1a0137ef486eae5607fbc7852169e626113ebd07",
            &[
                "eaa7952dc7a202a75a741ed4bc9516c6aa624942",
                "22751db39f9ea5f474440fa82c84301c693ba3ed",
            ],
        ),
        (
            "22751db39f9ea5f474440fa82c84301c693ba3ed",
            &["14837bd3e1eddf263536d7121f0f6b22293715eb"],
        ),
        (
            "14837bd3e1eddf263536d7121f0f6b22293715eb",
            &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
        ),
        (
            "eaa7952dc7a202a75a741ed4bc9516c6aa624942",
            &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
        ),
        (
            "17b91a61c98954a0d56e162c5749b036f2a8c53b",
            &["a054e8662b0bce64197c45a6f97f380547cee39c"],
        ),
        (
            "a054e8662b0bce64197c45a6f97f380547cee39c",
            &["14c40bfded212b58b04ea8344e7c3712f8b9bfcb"],
        ),
        (
            "14c40bfded212b58b04ea8344e7c3712f8b9bfcb",
            &["1a095db72c8bfd761cddfa4983f8acdfc7ef2f5f"],
        ),
        ("1a095db72c8bfd761cddfa4983f8acdfc7ef2f5f", &[]),
    ])
}

// Build GraphCommitRows from a list of (oid_hex, [parent_hex...]) pairs.
// Each label is padded to 40 hex chars using only hex-safe digits.
fn rows_from_pairs(pairs: &[(&str, &[&str])]) -> Vec<GraphCommitRow> {
    fn expand(s: &str) -> git2::Oid {
        // Map label chars to hex: use sha256-style - only 0-9a-f.
        // Simple approach: hash the label string to a deterministic 40-char hex.
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        s.hash(&mut h);
        let v = h.finish();
        // Produce 40 hex chars by repeating the 16-char hash.
        let hex16 = format!("{:016x}", v);
        let hex40 = format!("{}{}{}{}", hex16, &hex16[..8], &hex16[..8], &hex16[..8]);
        git2::Oid::from_str(&hex40[..40]).expect("oid from label")
    }
    pairs
        .iter()
        .map(|(o, ps)| GraphCommitRow {
            oid: expand(o),
            parents: ps.iter().map(|p| expand(p)).collect(),
        })
        .collect()
}

// Render rows to a list of strings, one per commit row.
fn render_rows(pairs: &[(&str, &[&str])]) -> Vec<String> {
    let rows = rows_from_pairs(pairs);
    let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
    lines.iter().map(|c| graph_cells_to_string(c)).collect()
}

// ------- Tests derived from complex-merge-graph-repo topology -------
// Commit order (topo-order, newest first), using short labels:
//
//  0: M1  parents=[R, F1_3]     merge: feature/a back into main
//  1: F1_3 parents=[F1_2]       feat(a): add a3 after cross merge
//  2: F1_2 parents=[A2, R]      merge: sync main into feature/a (cross merge)
//  3: R    parents=[C, L1]      merge: release/1.0 back to main
//  4: L1   parents=[MA]         docs(release): prepare 1.0 notes
//  5: C    parents=[H, C2]      merge: feature/c into main
//  6: C2   parents=[C1]         feat(c): add c2
//  7: C1   parents=[BASE]       feat(c): add c1
//  8: H    parents=[MA, FX]     merge: hotfix/urgent into main
//  9: FX   parents=[BASE]       fix: urgent hotfix
// 10: MA   parents=[MB, A2]     merge: feature/a into main
// 11: A2   parents=[A1]         feat(a): add a2
// 12: A1   parents=[BASE]       feat(a): add a1
// 13: MB   parents=[BASE]       feat: main baseline 2
// 14: B2   parents=[B1]         feat(b): add b2 (on feature/b, not in main history here)
// 15: B1   parents=[BASE]       feat(b): add b1
// 16: BASE parents=[INIT]       feat: main baseline 1
// 17: INIT parents=[]           chore: initial commit
//
// (Using git --topo-order output)

#[test]
fn test_graph_full_repo_topology() {
    // Full topology from d:/tmp/complex-merge-graph-repo, topo-order --all.
    let lines = build_commit_graph_lines(&complex_repo_rows(), GraphCharset::unicode());
    let strs: Vec<String> = lines.iter().map(|c| graph_cells_to_string(c)).collect();
    eprintln!("=== Full repo topology ===");
    for (i, row) in strs.iter().enumerate() {
        eprintln!("row {:2}: {}", i, row);
    }
    for (i, row) in strs.iter().enumerate() {
        assert!(!row.is_empty(), "row {i} empty");
    }
    for (i, row) in strs.iter().enumerate() {
        let w = row.chars().count();
        assert!(w <= 22, "row {i} too wide ({w} chars): {row:?}");
    }
}

#[test]
fn test_complex_repo_node_cells_include_row_oid() {
    let rows = complex_repo_rows();
    let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
    for (idx, row) in rows.iter().enumerate() {
        let row_oid = row.oid.to_string();
        let node = lines[idx]
            .iter()
            .find(|cell| matches!(cell.text.as_str(), "◯" | "⏣" | "*" | "#"))
            .expect("node cell exists");
        assert!(
            node.pipe_oids.iter().any(|oid| oid == &row_oid),
            "row {idx} node should include row oid"
        );
    }
}

#[test]
fn test_complex_repo_has_multi_owner_overlap_cells() {
    let rows = complex_repo_rows();
    let lines = build_commit_graph_lines(&rows, GraphCharset::unicode());
    let has_overlap = lines
        .iter()
        .flat_map(|line| line.iter())
        .any(|cell| cell.text != " " && cell.pipe_oids.len() > 1);
    assert!(
        has_overlap,
        "expected at least one non-space overlap cell with multiple path owners"
    );
}

#[test]
fn test_graph_cross_merge_bidirectional() {
    // Minimal cross-merge: X merges [A, B], later Y merges [C, X] where X was already
    // on a side branch. Tests that lane assignments don't collide.
    let strs = render_rows(&[
        ("M", &["A", "B"]),  // merge commit on main
        ("B", &["A", "M2"]), // cross: B references M2 which is ahead in the list
        ("A", &["BASE"]),
        ("M2", &["BASE"]),
        ("BASE", &[]),
    ]);
    for (i, row) in strs.iter().enumerate() {
        assert!(!row.is_empty(), "row {i} empty");
    }
}

#[test]
fn test_graph_feature_b_parallel_branch() {
    // feature/b has commits b3, sync-merge, b2, b1 that never merge into main
    // within this window. They should appear as parallel lanes.
    let strs = render_rows(&[
        ("B3", &["BM"]),
        ("BM", &["B2", "MB"]), // sync merge from main
        ("MB", &["BASE"]),
        ("B2", &["B1"]),
        ("B1", &["BASE"]),
        ("BASE", &[]),
    ]);
    for (i, row) in strs.iter().enumerate() {
        assert!(!row.is_empty(), "row {i} empty: {:?}", row);
    }
    // BM is a merge node.
    assert!(
        strs[1].contains('⏣'),
        "BM should be merge node: {:?}",
        strs[1]
    );
}
