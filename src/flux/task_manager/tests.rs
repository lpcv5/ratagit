use super::*;
use crate::app::diff::{DiffCacheKey, DiffTarget};

fn status() -> BackgroundLoad {
    BackgroundLoad::Status {
        fast: true,
        operation_epoch: 42,
    }
}
fn failure(request: TaskRequest) -> BackgroundCompleted {
    BackgroundCompleted {
        request,
        result: Err("controlled failure".into()),
    }
}

#[test]
fn test_request_identity_is_derived_from_its_only_load_description() {
    let cases = [
        (status(), TaskKey::Status),
        (BackgroundLoad::Branches, TaskKey::Branches),
        (BackgroundLoad::Stashes, TaskKey::Stashes),
        (
            BackgroundLoad::Commits {
                limit: 30,
                fast: true,
            },
            TaskKey::Commits,
        ),
        (
            BackgroundLoad::Commits {
                limit: 70,
                fast: false,
            },
            TaskKey::Commits,
        ),
        (
            BackgroundLoad::BranchCommits {
                branch: "feature".into(),
                limit: 100,
            },
            TaskKey::BranchCommits {
                branch: "feature".into(),
            },
        ),
        (
            BackgroundLoad::CommitFiles {
                oid: "a".repeat(40),
            },
            TaskKey::CommitFiles,
        ),
        (BackgroundLoad::StashFiles { index: 3 }, TaskKey::StashFiles),
        (
            BackgroundLoad::Diff {
                cache_key: DiffCacheKey::None,
                target: DiffTarget::None,
            },
            TaskKey::active_diff(),
        ),
    ];
    let mut manager = TaskManager::new();
    for (load, key) in cases {
        let expected = format!("{load:?}");
        let request = manager.start(load, TaskPriority::High);
        assert_eq!(request.key, key);
        assert_eq!(request.priority, TaskPriority::High);
        assert_eq!(format!("{:?}", request.load), expected);
    }
}

#[test]
fn test_superseded_completion_retires_only_its_generation() {
    let mut manager = TaskManager::new();
    let old = failure(manager.start(status(), TaskPriority::High));
    let current = failure(manager.start(status(), TaskPriority::High));
    assert!(current.request.generation > old.request.generation);
    assert!(!manager.accept(&old));
    assert!(manager.has_task(&TaskKey::Status));
    assert!(manager.accept(&current));
    assert!(manager.is_empty());
    assert!(!manager.accept(&old));
    assert!(!manager.accept(&current));
}

#[test]
fn test_superseded_generations_keep_existing_admission_count_until_completion() {
    let mut manager = TaskManager::new();
    let requests: Vec<_> = (0..6)
        .map(|_| {
            assert!(manager.can_start(TaskPriority::Normal));
            failure(manager.start(BackgroundLoad::Branches, TaskPriority::Normal))
        })
        .collect();
    assert!(!manager.can_start(TaskPriority::Normal));
    assert!(!manager.can_start(TaskPriority::Low));
    assert!(manager.can_start(TaskPriority::High));
    assert!(!manager.accept(&requests[0]));
    assert!(manager.can_start(TaskPriority::Normal));
    assert!(manager.accept(&requests[5]));
    // Older submitted jobs still count even after the current result is accepted.
    assert!(manager.has_task(&TaskKey::Branches));
    manager.cancel(&TaskKey::Branches);
    assert!(manager.is_empty());
}

#[test]
fn test_cancelled_latest_generation_is_rejected_without_replacement() {
    let mut manager = TaskManager::new();
    let completed = failure(manager.start(status(), TaskPriority::High));
    manager.cancel(&TaskKey::Status);
    assert!(!manager.accept(&completed));
    assert!(manager.is_empty());
}

#[test]
fn test_cancel_clears_all_generations_only_for_requested_key() {
    let mut manager = TaskManager::new();
    let old = failure(manager.start(status(), TaskPriority::High));
    let latest = failure(manager.start(status(), TaskPriority::High));
    let other = failure(manager.start(BackgroundLoad::Branches, TaskPriority::Normal));
    manager.cancel(&TaskKey::Status);
    assert!(!manager.has_task(&TaskKey::Status));
    assert!(manager.has_task(&TaskKey::Branches));
    assert!(!manager.accept(&old));
    assert!(!manager.accept(&latest));
    assert!(manager.accept(&other));
    assert!(manager.is_empty());
}

#[test]
fn test_cancel_then_restart_rejects_old_result_and_preserves_new_registration() {
    let mut manager = TaskManager::new();
    let old = failure(manager.start(status(), TaskPriority::High));
    manager.cancel(&TaskKey::Status);
    let new = failure(manager.start(status(), TaskPriority::High));
    assert!(new.request.generation > old.request.generation);
    assert!(!manager.accept(&old));
    assert!(manager.has_task(&TaskKey::Status));
    assert!(manager.accept(&new));
}

#[test]
fn test_current_failure_is_accepted_once_and_allows_retry() {
    let mut manager = TaskManager::new();
    let completed = failure(manager.start(BackgroundLoad::Branches, TaskPriority::Normal));
    assert!(manager.accept(&completed));
    assert!(!manager.accept(&completed));
    assert!(!manager.has_task(&TaskKey::Branches));
    assert_eq!(manager.metrics().failed_total, 1);
    let retry = failure(manager.start(BackgroundLoad::Branches, TaskPriority::Normal));
    assert!(manager.accept(&retry));
}

#[test]
fn test_distinct_keys_use_global_generations_and_complete_independently() {
    let mut manager = TaskManager::new();
    let branches = failure(manager.start(BackgroundLoad::Branches, TaskPriority::Normal));
    let status = failure(manager.start(status(), TaskPriority::High));
    assert!(status.request.generation > branches.request.generation);
    assert!(manager.accept(&status));
    assert!(manager.has_task(&TaskKey::Branches));
    assert!(manager.accept(&branches));
}
