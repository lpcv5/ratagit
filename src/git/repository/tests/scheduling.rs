//! Exercise capability routes against real repositories with controlled worker queues.

use super::{init_repo_with_commit, write_file};
use crate::git::repository::Git2Repository;
use crate::git::{capabilities::*, operation::RepositoryPhase, GitError};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(10);

fn receive<T>(rx: &Receiver<Result<T, GitError>>) -> T {
    rx.recv_timeout(TIMEOUT)
        .expect("worker must complete")
        .expect("Git operation must succeed")
}

fn assert_pending<T>(rx: &Receiver<Result<T, GitError>>) {
    assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
}

/// Always release a blocked native worker, including when an assertion panics.
struct WorkerGate {
    release: Option<Sender<()>>,
    finished: Receiver<Result<(), GitError>>,
}

impl WorkerGate {
    fn block(repo: &Git2Repository, external: Option<usize>) -> Self {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let job = move |_: &mut crate::git::repository::Git2RepoInner| {
            started_tx.send(()).expect("signal worker start");
            release_rx
                .recv_timeout(TIMEOUT)
                .map_err(|err| GitError::Git2(format!("test worker gate: {err}")))
        };
        let finished = match external {
            Some(index) => repo.external_workers[index].run("test_gate", job),
            None => repo.spawn_repo_job("test_gate", job),
        }
        .expect("enqueue worker gate");
        started_rx.recv_timeout(TIMEOUT).expect("worker started");
        Self {
            release: Some(release_tx),
            finished,
        }
    }
}

impl Drop for WorkerGate {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        let _ = self.finished.recv_timeout(TIMEOUT);
    }
}

#[test]
fn test_primary_fifo_preserves_reads_between_cross_capability_writes() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "v2\n");
    let gate = WorkerGate::block(&repo, None);
    let path = PathBuf::from("tracked.txt");

    let stage = repo.stage_async(path.clone()).unwrap();
    let staged_status = repo.status_async().unwrap();
    let staged_diff = repo.diff_staged_async(path.clone()).unwrap();
    let unstage = repo.unstage_async(path.clone()).unwrap();
    let unstaged_status = repo.status_async().unwrap();
    let restage = repo.stage_async(path).unwrap();
    let commit = repo.commit_async("queued commit".into()).unwrap();
    let create = repo.create_branch_async("queued-branch".into()).unwrap();
    let checkout = repo.checkout_branch_async("queued-branch".into()).unwrap();
    let branches = repo.branches_async().unwrap();
    let stashes = repo.stashes_async().unwrap();
    let final_status = repo.status_async().unwrap();
    assert_pending(&stage);
    assert_pending(&staged_status);
    assert_pending(&staged_diff);
    assert_pending(&branches);
    assert_pending(&stashes);
    drop(gate);

    receive(&stage);
    let status = receive(&staged_status);
    assert_eq!(status.staged.len(), 1);
    assert!(status.unstaged.is_empty());
    assert!(receive(&staged_diff)
        .iter()
        .any(|line| line.content == "v2"));
    receive(&unstage);
    let status = receive(&unstaged_status);
    assert!(status.staged.is_empty());
    assert_eq!(status.unstaged.len(), 1);
    receive(&restage);
    let oid = receive(&commit);
    receive(&create);
    receive(&checkout);
    assert!(receive(&branches)
        .iter()
        .any(|branch| branch.name == "queued-branch" && branch.is_current));
    assert!(receive(&stashes).is_empty());
    let status = receive(&final_status);
    assert!(status.staged.is_empty() && status.unstaged.is_empty());

    // The same aggregate backend must observe the completed writes from other capabilities.
    let backend: &dyn GitRepository = &repo;
    assert_eq!(backend.commits(1).unwrap()[0].oid, oid);
    assert_eq!(
        backend.commit_files(&oid).unwrap()[0].path,
        Path::new("tracked.txt")
    );
    assert!(backend
        .commit_diff_scoped(&oid, Some(Path::new("tracked.txt")))
        .unwrap()
        .iter()
        .any(|line| line.content == "v2"));
    assert_eq!(
        backend.operation_snapshot().unwrap().phase(),
        RepositoryPhase::Idle
    );
}

#[test]
fn test_stash_mutations_and_reads_share_primary_fifo() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "stash sequence\n");
    // CLI stash needs a repository-local identity, independent of user configuration.
    let raw = git2::Repository::open(dir.path()).unwrap();
    raw.config()
        .unwrap()
        .set_str("user.name", "tester")
        .unwrap();
    raw.config()
        .unwrap()
        .set_str("user.email", "tester@example.com")
        .unwrap();
    let gate = WorkerGate::block(&repo, None);
    let push = repo
        .stash_push_paths_async(vec!["tracked.txt".into()], "queued stash".into())
        .unwrap();
    let clean_status = repo.status_async().unwrap();
    let stashes = repo.stashes_async().unwrap();
    let stash_diff = repo
        .stash_diff_async(0, Some("tracked.txt".into()))
        .unwrap();
    let apply = repo.stash_apply_async(0).unwrap();
    let restored_status = repo.status_async().unwrap();
    let remove = repo.stash_drop_async(0).unwrap();
    let empty_stashes = repo.stashes_async().unwrap();
    let worktree_diff = repo.diff_unstaged_async("tracked.txt".into()).unwrap();
    assert_pending(&push);
    assert_pending(&stashes);
    assert_pending(&stash_diff);
    drop(gate);

    assert_eq!(receive(&push), 0);
    assert!(receive(&clean_status).unstaged.is_empty());
    assert_eq!(receive(&stashes).len(), 1);
    assert!(receive(&stash_diff)
        .iter()
        .any(|line| line.content == "stash sequence"));
    receive(&apply);
    assert_eq!(receive(&restored_status).unstaged.len(), 1);
    receive(&remove);
    assert!(receive(&empty_stashes).is_empty());
    assert!(receive(&worktree_diff)
        .iter()
        .any(|line| line.content == "stash sequence"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("tracked.txt")).unwrap(),
        "stash sequence\n"
    );
}

#[test]
fn test_primary_failure_does_not_drop_following_writes_or_reads() {
    let (dir, repo) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "after failure\n");
    let gate = WorkerGate::block(&repo, None);
    let failed = repo.checkout_branch_async("missing-branch".into()).unwrap();
    let stage = repo.stage_async("tracked.txt".into()).unwrap();
    let commit = repo.commit_async("after failure".into()).unwrap();
    let status = repo.status_async().unwrap();
    drop(gate);

    assert!(failed.recv_timeout(TIMEOUT).unwrap().is_err());
    receive(&stage);
    let oid = receive(&commit);
    assert!(receive(&status).staged.is_empty());
    assert_eq!(repo.commits(1).unwrap()[0].oid, oid);
    assert_eq!(repo.commits(1).unwrap()[0].message, "after failure");
}

#[test]
fn test_external_reads_progress_while_primary_lane_is_blocked() {
    let (dir, repo) = init_repo_with_commit();
    let raw = git2::Repository::open(dir.path()).unwrap();
    let head = raw.head().unwrap();
    let oid = head.target().unwrap().to_string();
    let branch = head.shorthand().unwrap().to_string();
    write_file(&dir.path().join("tracked.txt"), "modified\n");
    write_file(&dir.path().join("untracked.txt"), "untracked\n");
    let gate = WorkerGate::block(&repo, None);
    let full_status = repo.status_async().unwrap();
    let branches = repo.branches_async().unwrap();
    let stashes = repo.stashes_async().unwrap();
    let worktree_diff = repo.diff_unstaged_async("tracked.txt".into()).unwrap();

    let fast_status = repo.status_fast_async().unwrap();
    let commits = repo.commits_async(10).unwrap();
    let fast_commits = repo.commits_fast_async(10).unwrap();
    let branch_commits = repo.commits_for_branch_async(&branch, 10).unwrap();
    let branch_log = repo.branch_log_async(branch, 10).unwrap();
    let historical_diff = repo.commit_diff_scoped_async(oid.clone(), None).unwrap();
    let status = receive(&fast_status);
    assert_eq!(status.unstaged.len(), 1);
    assert!(status.untracked.is_empty());
    assert_eq!(receive(&commits)[0].oid, oid);
    assert_eq!(receive(&fast_commits)[0].oid, oid);
    assert_eq!(receive(&branch_commits)[0].oid, oid);
    assert!(receive(&branch_log)
        .iter()
        .any(|line| line.content.contains("initial")));
    assert!(receive(&historical_diff)
        .iter()
        .any(|line| line.content == "v1"));

    assert_pending(&full_status);
    assert_pending(&branches);
    assert_pending(&stashes);
    assert_pending(&worktree_diff);
    drop(gate);
    let status = receive(&full_status);
    assert_eq!(status.unstaged.len(), 1);
    assert_eq!(status.untracked.len(), 1);
    assert!(!receive(&branches).is_empty());
    assert!(receive(&stashes).is_empty());
    assert!(receive(&worktree_diff)
        .iter()
        .any(|line| line.content == "modified"));
}

#[test]
fn test_external_round_robin_wraps_without_blocking_primary_writes() {
    let (dir, repo) = init_repo_with_commit();
    write_file(
        &dir.path().join("tracked.txt"),
        "staged while pool blocked\n",
    );
    let count = repo.external_workers.len();
    let mut gates: Vec<_> = (0..count)
        .map(|index| Some(WorkerGate::block(&repo, Some(index))))
        .collect();
    // Two complete rounds must return to the same workers across capability calls.
    let statuses: Vec<_> = (0..count)
        .map(|_| repo.status_fast_async().unwrap())
        .collect();
    let histories: Vec<_> = (0..count)
        .map(|_| repo.commits_fast_async(1).unwrap())
        .collect();
    let stage = repo.stage_async("tracked.txt".into()).unwrap();
    let primary_status = repo.status_async().unwrap();
    let primary_diff = repo.diff_staged_async("tracked.txt".into()).unwrap();
    receive(&stage);
    assert_eq!(receive(&primary_status).staged.len(), 1);
    assert!(receive(&primary_diff)
        .iter()
        .any(|line| line.content == "staged while pool blocked"));
    for index in 0..count {
        assert_pending(&statuses[index]);
        assert_pending(&histories[index]);
    }
    // Releasing workers in reverse order also checks that one lane cannot hold up others.
    for index in (0..count).rev() {
        drop(gates[index].take());
        assert_eq!(receive(&statuses[index]).staged.len(), 1);
        assert_eq!(receive(&histories[index])[0].message, "initial");
        for blocked in 0..index {
            assert_pending(&statuses[blocked]);
            assert_pending(&histories[blocked]);
        }
    }
}

#[test]
fn test_fetch_probes_primary_after_checkout_then_runs_on_external_pool() {
    let (dir, repo) = init_repo_with_commit();
    let remote_dir = TempDir::new().unwrap();
    let remote_repo = git2::Repository::init_bare(remote_dir.path()).unwrap();
    let raw = git2::Repository::open(dir.path()).unwrap();
    let initial = raw.head().unwrap().peel_to_commit().unwrap();
    raw.remote("upstream", remote_dir.path().to_str().unwrap())
        .unwrap()
        .push(&["HEAD:refs/heads/main"], None)
        .unwrap();
    raw.reference(
        "refs/remotes/upstream/main",
        initial.id(),
        true,
        "test upstream",
    )
    .unwrap();
    raw.branch("with-upstream", &initial, false)
        .unwrap()
        .set_upstream(Some("upstream/main"))
        .unwrap();
    let parent = remote_repo.find_commit(initial.id()).unwrap();
    let tree = parent.tree().unwrap();
    let signature = git2::Signature::now("tester", "tester@example.com").unwrap();
    let new_oid = remote_repo
        .commit(
            Some("refs/heads/main"),
            &signature,
            &signature,
            "remote advance",
            &tree,
            &[&parent],
        )
        .unwrap();
    let gates: Vec<_> = (0..repo.external_workers.len())
        .map(|index| WorkerGate::block(&repo, Some(index)))
        .collect();

    let checkout = repo.checkout_branch_async("with-upstream".into()).unwrap();
    // This call waits for its primary probe, but must not wait for the external fetch.
    let fetch = repo.fetch_default_async().unwrap();
    receive(&checkout);
    assert_pending(&fetch);
    assert_eq!(raw.head().unwrap().shorthand(), Some("with-upstream"));
    assert_eq!(
        raw.find_reference("refs/remotes/upstream/main")
            .unwrap()
            .target(),
        Some(initial.id())
    );
    drop(gates);
    assert_eq!(receive(&fetch), "upstream");
    assert_eq!(
        raw.find_reference("refs/remotes/upstream/main")
            .unwrap()
            .target(),
        Some(new_oid)
    );
}
