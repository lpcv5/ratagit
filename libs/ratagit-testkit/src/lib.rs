use ratagit_core::{
    BranchEntry, CommitEntry, CommitHashStatus, FileEntry, RepoSnapshot, StashEntry,
};

pub fn fixture_commit(id: &str, summary: &str) -> CommitEntry {
    CommitEntry {
        id: id.to_string(),
        full_id: id.to_string(),
        summary: summary.to_string(),
        message: summary.to_string(),
        author_name: "ratagit-tests".to_string(),
        graph: "●".to_string(),
        hash_status: CommitHashStatus::Unpushed,
        is_merge: false,
    }
}

pub fn fixture_empty_repo() -> RepoSnapshot {
    RepoSnapshot {
        status_summary: "staged: 0, unstaged: 0".to_string(),
        current_branch: "main".to_string(),
        detached_head: false,
        files: Vec::new(),
        commits: Vec::new(),
        branches: vec![BranchEntry {
            name: "main".to_string(),
            is_current: true,
        }],
        stashes: Vec::new(),
    }
}

pub fn fixture_dirty_repo() -> RepoSnapshot {
    RepoSnapshot {
        status_summary: "staged: 1, unstaged: 2".to_string(),
        current_branch: "main".to_string(),
        detached_head: false,
        files: vec![
            FileEntry {
                path: "src/main.rs".to_string(),
                staged: true,
                untracked: false,
            },
            FileEntry {
                path: "src/lib.rs".to_string(),
                staged: false,
                untracked: false,
            },
            FileEntry {
                path: "README.md".to_string(),
                staged: false,
                untracked: true,
            },
        ],
        commits: vec![
            fixture_commit("abc1234", "init project"),
            fixture_commit("def5678", "wire commands"),
        ],
        branches: vec![
            BranchEntry {
                name: "main".to_string(),
                is_current: true,
            },
            BranchEntry {
                name: "feature/mvp".to_string(),
                is_current: false,
            },
        ],
        stashes: vec![StashEntry {
            id: "stash@{0}".to_string(),
            summary: "WIP on main: local test".to_string(),
        }],
    }
}

pub fn fixture_many_files() -> RepoSnapshot {
    let mut snapshot = fixture_dirty_repo();
    snapshot.files = (0..30)
        .map(|index| FileEntry {
            path: format!("file-{index:02}.txt"),
            staged: index % 2 == 0,
            untracked: false,
        })
        .collect();
    snapshot.status_summary = "staged: 15, unstaged: 15".to_string();
    snapshot
}

pub fn fixture_conflict() -> RepoSnapshot {
    let mut snapshot = fixture_dirty_repo();
    snapshot.status_summary = "staged: 0, unstaged: 2 (conflict)".to_string();
    snapshot.files = vec![
        FileEntry {
            path: "src/conflict.rs (both modified)".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "Cargo.toml (both modified)".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    snapshot
}

pub fn fixture_unicode_paths() -> RepoSnapshot {
    let mut snapshot = fixture_dirty_repo();
    snapshot.files = vec![
        FileEntry {
            path: "docs/你好.md".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "assets/emoji-🙂.txt".to_string(),
            staged: true,
            untracked: false,
        },
    ];
    snapshot.status_summary = "staged: 1, unstaged: 1".to_string();
    snapshot
}
