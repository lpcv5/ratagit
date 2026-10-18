use super::*;
use ratagit_core::custom_patch::{CustomPatch, FileSelection, Inclusion, file_key, line_ids};

fn custom(repo: &TmpGitRepo, oid: &str, paths: &[&str]) -> CustomPatch {
    let patch = HybridGitBackend::open(repo.path())
        .unwrap()
        .load_custom_patch(PatchRequest::CommitFiles {
            oid: oid.into(),
            paths: paths.iter().map(|p| (*p).into()).collect(),
        })
        .unwrap();
    CustomPatch {
        oid: oid.into(),
        files: patch
            .files
            .into_iter()
            .map(|file| FileSelection {
                file,
                inclusion: Inclusion::Whole,
            })
            .collect(),
    }
}
fn mixed_repo(name: &str) -> (TmpGitRepo, CustomPatch, String, String) {
    let repo = seeded_repo_with_two_files(name);
    let before = (1..=50).map(|n| format!("line {n}\n")).collect::<String>();
    write(repo.path().join("a.txt"), &before).unwrap();
    repo.run_git(&["commit", "-am", "long base"]);
    let after = before
        .replace("line 2\n", "FIRST\n")
        .replace("line 40\n", "LAST\n");
    write(repo.path().join("a.txt"), &after).unwrap();
    write(repo.path().join("b.txt"), "b2\n").unwrap();
    repo.run_git(&["commit", "-am", "source"]);
    let mut patch = custom(&repo, &id(&repo), &["a.txt", "b.txt"]);
    let a = patch
        .files
        .iter_mut()
        .find(|f| file_key(&f.file).unwrap() == "a.txt")
        .unwrap();
    a.inclusion = Inclusion::Lines(
        line_ids(&a.file)
            .into_iter()
            .filter(|(h, _)| *h == 0)
            .collect(),
    );
    (repo, patch, before, after)
}

#[test]
fn custom_patch_combines_partial_and_whole_files_and_round_trips_without_index_writes() {
    if !git_available() {
        return;
    }
    let (repo, patch, before, after) = mixed_repo("custom-mixed");
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let text = backend.render_custom_patch(&patch).unwrap();
    let text = String::from_utf8(text).unwrap();
    assert!(text.contains("+FIRST"));
    assert!(!text.contains("+LAST"));
    assert!(text.contains("+b2"));
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    let head = id(&repo);
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch.clone()),
            action: PatchAction::Reverse,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        before.replace("line 40\n", "LAST\n")
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "b1\n"
    );
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
    assert_eq!(id(&repo), head);
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch.clone()),
            action: PatchAction::Apply,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        after
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "b2\n"
    );
    assert!(
        repo.run_git_capture(&["status", "--porcelain=v1"])
            .is_empty()
    );
}

#[test]
fn custom_patch_conflict_in_later_file_rejects_the_entire_batch() {
    if !git_available() {
        return;
    }
    let (repo, patch, _, after) = mixed_repo("custom-conflict");
    write(repo.path().join("b.txt"), "local conflict\n").unwrap();
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CustomPatch {
                patch: Box::new(patch),
                action: PatchAction::Reverse
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        after
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "local conflict\n"
    );
}

#[test]
fn custom_patch_removes_exact_mixed_selection_from_original_commit_and_replays_descendants() {
    if !git_available() {
        return;
    }
    let (repo, patch, before, _) = mixed_repo("custom-history");
    write(repo.path().join("later.txt"), "descendant\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "later"]);
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch),
            action: PatchAction::Discard,
        },
    )
    .unwrap();
    assert_eq!(
        repo.run_git_capture(&["show", "HEAD:a.txt"]),
        before.replace("line 40\n", "LAST\n")
    );
    assert_eq!(repo.run_git_capture(&["show", "HEAD:b.txt"]), "b1\n");
    assert_eq!(
        repo.run_git_capture(&["show", "HEAD:later.txt"]),
        "descendant\n"
    );
    assert_eq!(
        repo.run_git_capture(&["log", "-2", "--format=%s"]),
        "later\nsource\n"
    );
    assert!(
        repo.run_git_capture(&["status", "--porcelain=v1"])
            .is_empty()
    );
}

#[test]
fn custom_patch_whole_files_support_binary_rename_modes_empty_additions_and_deletions() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("custom-metadata");
    write(repo.path().join("blob.bin"), [0, 1, 2, 255]).unwrap();
    write(repo.path().join("old.txt"), "rename content\n").unwrap();
    write(repo.path().join("dead.txt"), "deleted content\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "metadata base"]);
    write(repo.path().join("blob.bin"), [0, 2, 3, 254]).unwrap();
    repo.run_git(&["mv", "old.txt", "new.txt"]);
    repo.run_git(&["rm", "dead.txt"]);
    write(repo.path().join("empty.txt"), "").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["update-index", "--chmod=+x", "new.txt"]);
    repo.run_git(&["commit", "-m", "metadata changes"]);
    let patch = custom(
        &repo,
        &id(&repo),
        &["blob.bin", "old.txt", "new.txt", "empty.txt", "dead.txt"],
    );
    assert_eq!(patch.files.len(), 4);
    let bytes = HybridGitBackend::open(repo.path())
        .unwrap()
        .render_custom_patch(&patch)
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("GIT binary patch"));
    assert!(String::from_utf8_lossy(&bytes).contains("rename from old.txt"));
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch.clone()),
            action: PatchAction::Reverse,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read(repo.path().join("blob.bin")).unwrap(),
        [0, 1, 2, 255]
    );
    assert!(repo.path().join("old.txt").exists());
    assert!(!repo.path().join("new.txt").exists());
    assert!(!repo.path().join("empty.txt").exists());
    assert!(repo.path().join("dead.txt").exists());
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch.clone()),
            action: PatchAction::Apply,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read(repo.path().join("blob.bin")).unwrap(),
        [0, 2, 3, 254]
    );
    assert!(repo.path().join("new.txt").exists());
    assert!(repo.path().join("empty.txt").exists());
    assert!(
        repo.run_git_capture(&["status", "--porcelain=v1"])
            .is_empty()
    );
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch),
            action: PatchAction::Discard,
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read(repo.path().join("blob.bin")).unwrap(),
        [0, 1, 2, 255]
    );
    assert!(repo.path().join("old.txt").exists());
    assert!(!repo.path().join("empty.txt").exists());
    assert!(repo.path().join("dead.txt").exists());
    assert!(
        repo.run_git_capture(&["status", "--porcelain=v1"])
            .is_empty()
    );
}

#[test]
fn custom_patch_tampered_source_and_replay_conflict_preserve_repository_state() {
    if !git_available() {
        return;
    }
    let (repo, mut patch, _, _) = mixed_repo("custom-stale");
    let before = state(&repo);
    let valid = patch.clone();
    patch.files[0].file.encoded = Some(b"tampered native bytes".to_vec());
    assert!(
        run(
            &repo,
            Operation::CustomPatch {
                patch: Box::new(patch),
                action: PatchAction::Reverse
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
    write(repo.path().join("b.txt"), "b3\n").unwrap();
    repo.run_git(&["commit", "-am", "overlap"]);
    let before = state(&repo);
    assert!(
        run(
            &repo,
            Operation::CustomPatch {
                patch: Box::new(valid),
                action: PatchAction::Discard
            }
        )
        .is_err()
    );
    assert_eq!(state(&repo), before);
}

#[test]
fn custom_patch_preserves_crlf_missing_newline_and_non_utf8_bytes() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("custom-bytes");
    let old = b"old\xff\r\nkeep\r\nlast";
    let new = b"new\xfe\r\nkeep\r\nLAST";
    write(repo.path().join("a.txt"), old).unwrap();
    repo.run_git(&["commit", "-am", "bytes base"]);
    write(repo.path().join("a.txt"), new).unwrap();
    repo.run_git(&["commit", "-am", "bytes source"]);
    let patch = custom(&repo, &id(&repo), &["a.txt"]);
    let bytes = HybridGitBackend::open(repo.path())
        .unwrap()
        .render_custom_patch(&patch)
        .unwrap();
    assert!(
        bytes
            .windows(b"No newline at end of file".len())
            .any(|s| s == b"No newline at end of file")
    );
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch.clone()),
            action: PatchAction::Reverse,
        },
    )
    .unwrap();
    assert_eq!(std::fs::read(repo.path().join("a.txt")).unwrap(), old);
    run(
        &repo,
        Operation::CustomPatch {
            patch: Box::new(patch),
            action: PatchAction::Apply,
        },
    )
    .unwrap();
    assert_eq!(std::fs::read(repo.path().join("a.txt")).unwrap(), new);
}
