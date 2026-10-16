// Keep log capture in a dedicated test binary: tracing callsite interest is
// process-wide and can race with parallel backend tests using no subscriber.
use std::fs::{create_dir_all, remove_dir_all, write};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use git2::{Repository, Signature};
use ratagit_core::Command;
use ratagit_git::{HybridGitBackend, execute_command};
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

#[test]
fn refresh_files_emits_debug_performance_events_without_stdout_payload() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tmp/git-tests")
        .join(format!(
            "ratagit-git-tracing-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time should be after epoch")
                .as_nanos()
        ));
    create_dir_all(&root).expect("temp repo should be creatable");
    let _cleanup = RepoCleanup(root.clone());
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
        let result = execute_command(&mut backend, Command::RefreshFiles);
        assert!(result.is_success());
    });

    let logs = writer.content();
    assert!(logs.contains("git command started"), "{logs}");
    assert!(logs.contains("command=\"refresh_files\""), "{logs}");
    assert!(logs.contains("git backend index count completed"), "{logs}");
    assert!(logs.contains("git status porcelain parsed"), "{logs}");
    assert!(logs.contains("result_count"), "{logs}");
    assert!(logs.contains("elapsed_ms"), "{logs}");
    assert!(!logs.contains("secret stdout payload"));
}

struct RepoCleanup(PathBuf);

impl Drop for RepoCleanup {
    fn drop(&mut self) {
        let _ = remove_dir_all(&self.0);
    }
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
