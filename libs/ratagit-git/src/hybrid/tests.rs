use super::*;
use std::fs::{create_dir_all, remove_dir_all, write};
use std::io;
use std::path::{Path, PathBuf};
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
            status: CommitFileStatus::Modified,
            conflicted: false,
        },
        FileEntry {
            path: "a.txt".to_string(),
            staged: true,
            untracked: false,
            status: CommitFileStatus::Modified,
            conflicted: false,
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
            status: CommitFileStatus::Modified,
            conflicted: false,
        },
        FileEntry {
            path: "new.txt".to_string(),
            staged: false,
            untracked: true,
            status: CommitFileStatus::Unknown,
            conflicted: false,
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
    assert_eq!(
        status_mode_for_index_entry_count(HUGE_REPO_INDEX_ENTRY_THRESHOLD - 1),
        StatusMode::LargeRepoFast
    );
    assert_eq!(
        status_mode_for_index_entry_count(HUGE_REPO_INDEX_ENTRY_THRESHOLD),
        StatusMode::HugeRepoMetadataOnly
    );
}

#[test]
fn commit_log_parser_reads_multiline_messages_merges_and_skips_empty_summaries() {
    let first = "1111111111111111111111111111111111111111";
    let second = "2222222222222222222222222222222222222222";
    let parent_a = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let parent_b = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let output = format!(
        "\x1e{first}\x001111111\x00{parent_a} {parent_b}\x00Alice Baker\x00subject line\n\nbody line\n\x00\n\
         \x1e{second}\x002222222\x00\x00Bob Creator\x00\n\n\x00\n",
    );

    let entries = parse_commit_log_page(output.as_bytes()).expect("commit log should parse");

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].full_id, first);
    assert_eq!(entries[0].id, "1111111");
    assert_eq!(entries[0].summary, "subject line");
    assert_eq!(entries[0].message, "subject line\n\nbody line");
    assert_eq!(entries[0].author_name, "Alice Baker");
    assert!(entries[0].is_merge);
}

#[test]
fn batch_hash_status_classifies_main_upstream_unpushed_and_detached() {
    let root = unique_temp_path("ratagit-batch-hash-status");
    create_dir_all(&root).expect("temp repo should be creatable");
    let repo = Repository::init(&root).expect("repo should initialize");

    let main_oid = create_empty_commit(&repo, "main", &[]);
    let pushed_oid = create_empty_commit(&repo, "pushed", &[main_oid]);
    let unpushed_oid = create_empty_commit(&repo, "unpushed", &[pushed_oid]);
    repo.reference("refs/heads/main", main_oid, true, "main ref")
        .expect("main ref should write");
    repo.reference(
        "refs/heads/feature/upstream",
        pushed_oid,
        true,
        "upstream ref",
    )
    .expect("upstream ref should write");
    repo.reference(
        "refs/heads/feature/status",
        unpushed_oid,
        true,
        "feature ref",
    )
    .expect("feature ref should write");
    repo.set_head("refs/heads/feature/status")
        .expect("feature branch should be current");
    let mut config = repo.config().expect("repo config should open");
    config
        .set_str("branch.feature/status.remote", ".")
        .expect("upstream remote should write");
    config
        .set_str("branch.feature/status.merge", "refs/heads/feature/upstream")
        .expect("upstream merge should write");

    let oids = vec![unpushed_oid, pushed_oid, main_oid];
    let statuses = classify_commit_hash_statuses(&repo, "feature/status", false, &oids)
        .expect("statuses should classify");
    assert_eq!(
        statuses.get(&unpushed_oid),
        Some(&CommitHashStatus::Unpushed)
    );
    assert_eq!(statuses.get(&pushed_oid), Some(&CommitHashStatus::Pushed));
    assert_eq!(
        statuses.get(&main_oid),
        Some(&CommitHashStatus::MergedToMain)
    );

    let detached = classify_commit_hash_statuses(&repo, "feature/status", true, &oids)
        .expect("detached statuses should classify");
    assert_eq!(detached.get(&pushed_oid), Some(&CommitHashStatus::Unpushed));
    assert_eq!(
        detached.get(&main_oid),
        Some(&CommitHashStatus::MergedToMain)
    );

    let _ = remove_dir_all(root);
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

fn unique_temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "{}-{}-{}",
        label,
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos()
    ))
}

fn create_empty_commit(repo: &Repository, message: &str, parents: &[Oid]) -> Oid {
    let mut index = repo.index().expect("index should open");
    let tree_oid = index.write_tree().expect("tree should write");
    let tree = repo.find_tree(tree_oid).expect("tree should exist");
    let signature = Signature::now("ratagit-tests", "ratagit-tests@example.com")
        .expect("signature should build");
    let parent_commits = parents
        .iter()
        .map(|oid| repo.find_commit(*oid).expect("parent commit should exist"))
        .collect::<Vec<_>>();
    let parent_refs = parent_commits.iter().collect::<Vec<_>>();
    repo.commit(None, &signature, &signature, message, &tree, &parent_refs)
        .expect("commit should write")
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
