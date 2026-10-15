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
/// A lossless patch fixture for partial staging UI and input tests.
pub fn fixture_patch() -> Box<ratagit_core::patch::Patch> {
    use ratagit_core::patch::*;
    let side = PatchSide {
        path: Some("src/lib.rs".into()),
        oid: None,
        mode: 0o100644,
    };
    let lines = [
        (PatchLineKind::Removed, "old first"),
        (PatchLineKind::Added, "new first"),
        (PatchLineKind::Context, "unchanged"),
        (PatchLineKind::Removed, "old last"),
        (PatchLineKind::Added, "new last"),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, (kind, text))| PatchLine {
        kind,
        old_line: match index {
            0 => Some(1),
            2 => Some(2),
            3 => Some(3),
            _ => None,
        },
        new_line: match index {
            1 => Some(1),
            2 => Some(2),
            4 => Some(3),
            _ => None,
        },
        content: format!("{text}\n").into_bytes(),
    })
    .collect();
    let file = PatchFile {
        source: PatchSource::Worktree,
        change: PatchChange::Modified,
        old: side.clone(),
        new: side,
        binary: false,
        encoded: Some(Vec::new()),
        hunks: vec![PatchHunk {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            header: b"@@ -1,3 +1,3 @@\n".to_vec(),
            lines,
        }],
    };
    let mut staged = file.clone();
    staged.source = PatchSource::Index;
    Box::new(Patch {
        preview: None,
        request: PatchRequest::File("src/lib.rs".into()),
        files: vec![staged, file],
        truncated: false,
    })
}
