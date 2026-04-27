use super::*;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::io;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use git2::Signature;
use ratagit_core::Command;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

#[test]
fn file_entry_from_status_maps_index_worktree_and_untracked_states() {
    let staged = file_entry_from_status("staged.txt".to_string(), Status::INDEX_MODIFIED);
    assert!(staged.staged);
    assert!(!staged.untracked);

    let unstaged = file_entry_from_status("unstaged.txt".to_string(), Status::WT_MODIFIED);
    assert!(!unstaged.staged);
    assert!(!unstaged.untracked);

    let untracked = file_entry_from_status("new.txt".to_string(), Status::WT_NEW);
    assert!(!untracked.staged);
    assert!(untracked.untracked);

    let both = file_entry_from_status(
        "both.txt".to_string(),
        Status::INDEX_MODIFIED | Status::WT_MODIFIED,
    );
    assert!(both.staged);
    assert!(!both.untracked);
}

#[test]
fn sort_files_is_deterministic_by_path() {
    let mut files = vec![
        FileEntry {
            path: "z.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "a.txt".to_string(),
            staged: true,
            untracked: false,
        },
    ];
    sort_files(&mut files);
    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[1].path, "z.txt");
}

#[test]
fn branch_helpers_handle_normal_empty_and_detached_states() {
    assert_eq!(
        branch_name_from_reference_name("refs/heads/main"),
        Some("main")
    );
    assert_eq!(branch_name_from_reference_name("HEAD"), None);

    let current = branch_entry("main", "main", false);
    assert!(current.is_current);

    let detached = branch_entry("main", "main", true);
    assert!(!detached.is_current);
}

#[test]
fn summarize_files_matches_app_status_summary_shape() {
    let files = vec![
        FileEntry {
            path: "staged.txt".to_string(),
            staged: true,
            untracked: false,
        },
        FileEntry {
            path: "new.txt".to_string(),
            staged: false,
            untracked: true,
        },
    ];
    assert_eq!(summarize_files(&files), "staged: 1, unstaged: 1");
}

#[test]
fn git2_status_fallback_collects_modified_and_nested_untracked_files() {
    let root = std::env::temp_dir().join(format!(
        "ratagit-git2-status-fallback-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    ));
    create_dir_all(&root).expect("temp repo should be creatable");
    let repo = Repository::init(&root).expect("repo should initialize");
    write(root.join("tracked.txt"), "v1\n").expect("tracked file should be writable");

    let mut index = repo.index().expect("index should open");
    index
        .add_path(Path::new("tracked.txt"))
        .expect("tracked file should add");
    index.write().expect("index should write");
    let tree_oid = index.write_tree().expect("tree should write");
    let tree = repo.find_tree(tree_oid).expect("tree should exist");
    let signature = Signature::now("ratagit-tests", "ratagit-tests@example.com")
        .expect("signature should build");
    repo.commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .expect("commit should succeed");

    write(root.join("tracked.txt"), "v2\n").expect("tracked file should modify");
    create_dir_all(root.join("nested")).expect("nested dir should be creatable");
    write(root.join("nested").join("new.txt"), "new\n").expect("untracked file should write");

    let files = collect_files_with_git2(&repo, StatusMode::Full)
        .expect("git2 fallback should collect files");
    let entries = files
        .iter()
        .map(|entry| (entry.path.as_str(), entry.staged, entry.untracked))
        .collect::<Vec<_>>();

    assert_eq!(
        entries,
        vec![
            ("nested/new.txt", false, true),
            ("tracked.txt", false, false)
        ]
    );

    let _ = remove_dir_all(root);
}

#[test]
fn status_mode_switches_to_large_repo_fast_at_threshold() {
    assert_eq!(
        status_mode_for_index_entry_count(LARGE_REPO_INDEX_ENTRY_THRESHOLD - 1),
        StatusMode::Full
    );
    assert_eq!(
        status_mode_for_index_entry_count(LARGE_REPO_INDEX_ENTRY_THRESHOLD),
        StatusMode::LargeRepoFast
    );
}

#[test]
fn refresh_files_emits_debug_performance_events_without_stdout_payload() {
    let root = std::env::temp_dir().join(format!(
        "ratagit-git-tracing-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    ));
    create_dir_all(&root).expect("temp repo should be creatable");
    let repo = Repository::init(&root).expect("repo should initialize");
    write(root.join("tracked.txt"), "v1\n").expect("tracked file should be writable");

    let mut index = repo.index().expect("index should open");
    index
        .add_path(Path::new("tracked.txt"))
        .expect("tracked file should add");
    index.write().expect("index should write");
    let tree_oid = index.write_tree().expect("tree should write");
    let tree = repo.find_tree(tree_oid).expect("tree should exist");
    let signature = Signature::now("ratagit-tests", "ratagit-tests@example.com")
        .expect("signature should build");
    repo.commit(Some("HEAD"), &signature, &signature, "init", &tree, &[])
        .expect("commit should succeed");
    drop(tree);
    drop(repo);

    write(root.join("tracked.txt"), "secret stdout payload\n").expect("tracked file should modify");

    let writer = CapturedWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_writer(writer.clone())
        .with_ansi(false)
        .without_time()
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        let mut backend = HybridGitBackend::open(&root).expect("backend should open");
        let result = crate::execute_command(&mut backend, Command::RefreshFiles);
        assert!(result.is_success());
    });

    let logs = writer.content();
    assert!(logs.contains("git command started"));
    assert!(logs.contains("command=\"refresh_files\""));
    assert!(logs.contains("git backend index count completed"));
    assert!(logs.contains("git status porcelain parsed"));
    assert!(logs.contains("result_count"));
    assert!(logs.contains("elapsed_ms"));
    assert!(!logs.contains("secret stdout payload"));

    let _ = remove_dir_all(root);
}

#[derive(Clone, Default)]
struct CapturedWriter {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl CapturedWriter {
    fn content(&self) -> String {
        let bytes = self.bytes.lock().expect("log buffer lock").clone();
        String::from_utf8(bytes).expect("logs should be utf-8")
    }
}

impl io::Write for CapturedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.bytes
            .lock()
            .expect("log buffer lock")
            .extend_from_slice(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'writer> MakeWriter<'writer> for CapturedWriter {
    type Writer = CapturedWriter;

    fn make_writer(&'writer self) -> Self::Writer {
        self.clone()
    }
}
