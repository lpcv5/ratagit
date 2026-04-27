use ratagit_core::{
    BranchEntry, CommitEntry, CommitFileStatus, CommitHashStatus, FileEntry, RepoSnapshot,
    StashEntry,
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

pub fn fixture_file(path: &str, staged: bool, untracked: bool) -> FileEntry {
    FileEntry {
        path: path.to_string(),
        staged,
        untracked,
        status: if untracked {
            CommitFileStatus::Unknown
        } else {
            CommitFileStatus::Modified
        },
        conflicted: false,
    }
}

pub fn fixture_branch(name: &str, is_current: bool) -> BranchEntry {
    BranchEntry {
        name: name.to_string(),
        is_current,
    }
}

pub fn fixture_empty_repo() -> RepoSnapshot {
    RepoSnapshot {
        status_summary: "staged: 0, unstaged: 0".to_string(),
        current_branch: "main".to_string(),
        detached_head: false,
        files: Vec::new(),
        commits: Vec::new(),
        branches: vec![fixture_branch("main", true)],
        stashes: Vec::new(),
    }
}

pub fn fixture_dirty_repo() -> RepoSnapshot {
    RepoSnapshot {
        status_summary: "staged: 1, unstaged: 2".to_string(),
        current_branch: "main".to_string(),
        detached_head: false,
        files: vec![
            fixture_file("src/main.rs", true, false),
            fixture_file("src/lib.rs", false, false),
            fixture_file("README.md", false, true),
        ],
        commits: vec![
            fixture_commit("abc1234", "init project"),
            fixture_commit("def5678", "wire commands"),
        ],
        branches: vec![
            fixture_branch("main", true),
            fixture_branch("feature/mvp", false),
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
        .map(|index| fixture_file(&format!("file-{index:02}.txt"), index % 2 == 0, false))
        .collect();
    snapshot.status_summary = "staged: 15, unstaged: 15".to_string();
    snapshot
}

pub fn fixture_conflict() -> RepoSnapshot {
    let mut snapshot = fixture_dirty_repo();
    snapshot.status_summary = "staged: 0, unstaged: 2 (conflict)".to_string();
    snapshot.files = vec![
        fixture_file("src/conflict.rs (both modified)", false, false),
        fixture_file("Cargo.toml (both modified)", false, false),
    ];
    for file in &mut snapshot.files {
        file.conflicted = true;
    }
    snapshot
}

pub fn fixture_unicode_paths() -> RepoSnapshot {
    let mut snapshot = fixture_dirty_repo();
    snapshot.files = vec![
        fixture_file("docs/你好.md", false, false),
        fixture_file("assets/emoji-🙂.txt", true, false),
    ];
    snapshot.status_summary = "staged: 1, unstaged: 1".to_string();
    snapshot
}
