use super::*;
mod checkout;
mod custom;
mod listing;
use ratagit_core::extras::RepositoryOperation;
use ratagit_core::patch::{Patch, PatchRequest};
use ratagit_core::workflow::{Operation, commit_files::PatchAction};

fn run(repo: &TmpGitRepo, op: Operation) -> Result<(), ratagit_git::GitError> {
    HybridGitBackend::open(repo.path())
        .unwrap()
        .repository_operation(&RepositoryOperation::Workflow(op))
}
fn id(repo: &TmpGitRepo) -> String {
    repo.run_git_capture(&["rev-parse", "HEAD"]).trim().into()
}
fn changed_repo(name: &str) -> (TmpGitRepo, String) {
    let repo = seeded_repo_with_two_files(name);
    write(repo.path().join("a.txt"), "a2\n").unwrap();
    write(repo.path().join("b.txt"), "b2\n").unwrap();
    repo.run_git(&["commit", "-am", "both files"]);
    let oid = id(&repo);
    (repo, oid)
}
fn patch(repo: &TmpGitRepo, oid: &str) -> Patch {
    HybridGitBackend::open(repo.path())
        .unwrap()
        .load_patch(PatchRequest::CommitFiles {
            oid: oid.into(),
            paths: vec!["a.txt".into()],
        })
        .unwrap()
}
fn state(repo: &TmpGitRepo) -> (String, String, String, Vec<u8>) {
    (
        id(repo),
        repo.run_git_capture(&["status", "--porcelain=v1", "--untracked-files=all"]),
        repo.run_git_capture(&["ls-files", "--stage"]),
        std::fs::read(repo.path().join("a.txt")).unwrap_or_default(),
    )
}

#[test]
fn commit_files_checkout_preserves_index_and_unrelated_changes_and_rejects_selected_dirt() {
    if !git_available() {
        return;
    }
    let (repo, _) = changed_repo("commit-file-checkout");
    let old = repo
        .run_git_capture(&["rev-parse", "HEAD^"])
        .trim()
        .to_string();
    write(repo.path().join("b.txt"), "unrelated\n").unwrap();
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    run(
        &repo,
        Operation::CommitFileCheckout {
            oid: old.clone(),
            paths: vec!["a.txt".into()],
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        "a1\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "unrelated\n"
    );
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CommitFileCheckout {
                oid: old,
                paths: vec!["a.txt".into(), "b.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(before, state(&repo));
}

#[test]
fn commit_files_checkout_handles_literal_paths_and_deleted_versions() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-literal");
    let root = id(&repo);
    write(repo.path().join("[one].txt"), "first\n").unwrap();
    write(repo.path().join("o.txt"), "other\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "literal"]);
    let source = id(&repo);
    write(repo.path().join("[one].txt"), "second\n").unwrap();
    write(repo.path().join("o.txt"), "unrelated second\n").unwrap();
    repo.run_git(&["commit", "-am", "second"]);
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    run(
        &repo,
        Operation::CommitFileCheckout {
            oid: source,
            paths: vec!["[one].txt".into()],
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("[one].txt")).unwrap(),
        "first\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("o.txt")).unwrap(),
        "unrelated second\n"
    );
    repo.run_git(&["restore", "--worktree", "--", ":(literal)[one].txt"]);
    run(
        &repo,
        Operation::CommitFileCheckout {
            oid: root,
            paths: vec!["[one].txt".into()],
        },
    )
    .unwrap();
    assert!(!repo.path().join("[one].txt").exists());
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
}

#[test]
fn commit_files_discard_rewrites_only_selected_files_and_preserves_descendants() {
    if !git_available() {
        return;
    }
    let (repo, oid) = changed_repo("commit-file-discard");
    let author = repo.run_git_capture(&["show", "-s", "--format=%an <%ae> %aI%n%B", &oid]);
    write(repo.path().join("later.txt"), "later\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "later"]);
    let old_head = id(&repo);
    run(
        &repo,
        Operation::CommitFileDiscard {
            oid,
            paths: vec!["a.txt".into()],
        },
    )
    .unwrap();
    assert_ne!(id(&repo), old_head);
    assert_eq!(repo.run_git_capture(&["show", "HEAD:a.txt"]), "a1\n");
    assert_eq!(repo.run_git_capture(&["show", "HEAD:b.txt"]), "b2\n");
    assert_eq!(repo.run_git_capture(&["show", "HEAD:later.txt"]), "later\n");
    assert_eq!(
        repo.run_git_capture(&["show", "-s", "--format=%an <%ae> %aI%n%B", "HEAD^"]),
        author
    );
    assert_eq!(
        repo.run_git_capture(&["log", "-2", "--format=%s"]),
        "later\nboth files\n"
    );
    assert!(
        repo.run_git_capture(&["status", "--porcelain=v1"])
            .is_empty()
    );
    assert_eq!(
        repo.run_git_capture(&["rev-parse", "HEAD@{1}"]).trim(),
        old_head
    );
}

#[test]
fn commit_files_discard_root_binary_and_rename_changes() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-root");
    let root = id(&repo);
    run(
        &repo,
        Operation::CommitFileDiscard {
            oid: root,
            paths: vec!["a.txt".into()],
        },
    )
    .unwrap();
    assert!(!repo.path().join("a.txt").exists());
    assert_eq!(
        repo.run_git_capture(&["rev-list", "--count", "HEAD"])
            .trim(),
        "1"
    );
    write(repo.path().join("blob.bin"), [0, 1, 2, 255]).unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "binary"]);
    repo.run_git(&["mv", "b.txt", "renamed.txt"]);
    write(repo.path().join("blob.bin"), [0, 2, 3, 254]).unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "rename and binary"]);
    run(
        &repo,
        Operation::CommitFileDiscard {
            oid: id(&repo),
            paths: vec!["b.txt".into(), "renamed.txt".into(), "blob.bin".into()],
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read(repo.path().join("blob.bin")).unwrap(),
        [0, 1, 2, 255]
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "b1\n"
    );
    assert!(!repo.path().join("renamed.txt").exists());
}

#[test]
fn commit_files_discard_replay_conflict_leaves_head_index_and_worktree_unchanged() {
    if !git_available() {
        return;
    }
    let (repo, oid) = changed_repo("commit-file-conflict");
    write(repo.path().join("a.txt"), "a3\n").unwrap();
    repo.run_git(&["commit", "-am", "overlap"]);
    let before = state(&repo);
    let error = run(
        &repo,
        Operation::CommitFileDiscard {
            oid,
            paths: vec!["a.txt".into()],
        },
    )
    .unwrap_err();
    assert!(error.message.contains("conflicts"), "{error:?}");
    assert_eq!(before, state(&repo));
    assert_eq!(
        git2::Repository::open(repo.path()).unwrap().state(),
        git2::RepositoryState::Clean
    );
}

#[test]
fn commit_files_partial_reverse_apply_and_original_commit_hunk_removal() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-hunks");
    let before = (1..=30).map(|n| format!("line {n}\n")).collect::<String>();
    let after = before
        .replace("line 2\n", "FIRST\n")
        .replace("line 28\n", "LAST\n");
    write(repo.path().join("a.txt"), &before).unwrap();
    repo.run_git(&["commit", "-am", "base"]);
    write(repo.path().join("a.txt"), &after).unwrap();
    repo.run_git(&["commit", "-am", "two hunks"]);
    let oid = id(&repo);
    let reviewed = patch(&repo, &oid);
    assert_eq!(reviewed.files[0].hunks.len(), 2);
    let ids = reviewed
        .line_ids()
        .into_iter()
        .filter(|id| id.hunk == 0)
        .collect::<Vec<_>>();
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    let reverse = || Operation::CommitPatch {
        patch: Box::new(reviewed.clone()),
        ids: ids.clone(),
        action: PatchAction::Reverse,
    };
    run(&repo, reverse()).unwrap();
    let partial = before.replace("line 28\n", "LAST\n");
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        partial
    );
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
    let unchanged = state(&repo);
    assert!(run(&repo, reverse()).is_err());
    assert_eq!(state(&repo), unchanged);
    run(
        &repo,
        Operation::CommitPatch {
            patch: Box::new(reviewed.clone()),
            ids: ids.clone(),
            action: PatchAction::Apply,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        after
    );
    run(
        &repo,
        Operation::CommitPatch {
            patch: Box::new(reviewed),
            ids,
            action: PatchAction::Discard,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        partial
    );
    assert_eq!(repo.run_git_capture(&["show", "HEAD:a.txt"]), partial);
    assert!(
        repo.run_git_capture(&["status", "--porcelain=v1"])
            .is_empty()
    );
}

#[test]
fn commit_files_rejects_unsafe_patch_and_dirty_history_without_changes() {
    if !git_available() {
        return;
    }
    let (repo, oid) = changed_repo("commit-file-guards");
    let reviewed = patch(&repo, &oid);
    for unsafe_patch in 0..4 {
        let mut p = reviewed.clone();
        match unsafe_patch {
            0 => p.truncated = true,
            1 => p.files[0].binary = true,
            2 => p.files[0].new.mode = 0o120000,
            _ => p.files[0].encoded = Some(b"different patch".to_vec()),
        }
        let before = state(&repo);
        let ids = p.line_ids();
        assert!(
            run(
                &repo,
                Operation::CommitPatch {
                    patch: Box::new(p),
                    ids,
                    action: PatchAction::Reverse
                }
            )
            .is_err()
        );
        assert_eq!(before, state(&repo));
    }
    write(repo.path().join("b.txt"), "dirty\n").unwrap();
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CommitFileDiscard {
                oid,
                paths: vec!["a.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(before, state(&repo));
}

#[test]
fn commit_files_rejects_ignored_obstructions_before_checkout_or_history_change() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-ignored");
    let source = id(&repo);
    repo.run_git(&["rm", "a.txt"]);
    write(repo.path().join(".gitignore"), "a.txt\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "remove and ignore"]);
    let target = id(&repo);
    write(repo.path().join("a.txt"), "ignored local content\n").unwrap();
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CommitFileCheckout {
                oid: source,
                paths: vec!["a.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
    assert!(
        run(
            &repo,
            Operation::CommitFileDiscard {
                oid: target,
                paths: vec!["a.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
}

#[test]
fn commit_files_rejects_foreign_detached_and_merge_history_without_mutation() {
    if !git_available() {
        return;
    }
    let (repo, target) = changed_repo("commit-file-history-guards");
    repo.run_git(&["switch", "--orphan", "other"]);
    write(repo.path().join("foreign.txt"), "foreign\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "foreign"]);
    let foreign = id(&repo);
    repo.run_git(&["switch", "main"]);
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CommitFileDiscard {
                oid: foreign,
                paths: vec!["a.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
    repo.run_git(&["switch", "--detach"]);
    assert!(
        run(
            &repo,
            Operation::CommitFileDiscard {
                oid: target.clone(),
                paths: vec!["a.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
    repo.run_git(&["switch", "main"]);
    repo.run_git(&["switch", "-c", "side"]);
    write(repo.path().join("side.txt"), "side\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "side"]);
    repo.run_git(&["switch", "main"]);
    repo.run_git(&["merge", "--no-ff", "-m", "merge", "side"]);
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CommitFileDiscard {
                oid: target,
                paths: vec!["a.txt".into()]
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
}

#[test]
fn commit_files_partial_original_addition_and_deletion_keep_unselected_lines() {
    if !git_available() {
        return;
    }
    for deletion in [false, true] {
        let repo = seeded_repo_with_two_files("commit-file-partial-existence");
        write(repo.path().join("a.txt"), "one\ntwo\nthree\n").unwrap();
        if deletion {
            repo.run_git(&["commit", "-am", "long base"]);
            repo.run_git(&["rm", "a.txt"]);
        } else {
            repo.run_git(&["rm", "--cached", "a.txt"]);
            repo.run_git(&["commit", "-m", "remove base"]);
            repo.run_git(&["add", "a.txt"]);
        }
        repo.run_git(&["commit", "-m", "existence change"]);
        let reviewed = patch(&repo, &id(&repo));
        let ids = vec![reviewed.line_ids()[0]];
        run(
            &repo,
            Operation::CommitPatch {
                patch: Box::new(reviewed),
                ids,
                action: PatchAction::Discard,
            },
        )
        .unwrap();
        assert_eq!(
            repo.run_git_capture(&["show", "HEAD:a.txt"]),
            if deletion { "one\n" } else { "two\nthree\n" }
        );
        assert!(
            repo.run_git_capture(&["status", "--porcelain=v1"])
                .is_empty()
        );
    }
}
