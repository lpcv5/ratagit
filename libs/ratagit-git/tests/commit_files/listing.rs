use super::*;
use ratagit_core::{CommitFileDiffPath, CommitFileDiffTarget};

fn preview(repo: &TmpGitRepo, oid: &str, path: &str, old_path: Option<&str>) -> String {
    HybridGitBackend::open(repo.path())
        .unwrap()
        .commit_file_diff(&CommitFileDiffTarget {
            commit_id: oid.into(),
            paths: vec![CommitFileDiffPath {
                path: path.into(),
                old_path: old_path.map(str::to_string),
            }],
        })
        .unwrap()
}

#[test]
fn commit_files_preserves_unicode_rename_paths_through_preview_checkout_and_discard() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-unicode-rename");
    let old = "旧 文件.txt";
    let new = "新 文件.txt";
    write(repo.path().join(old), "unicode rename\n").unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "unicode source"]);
    repo.run_git(&["mv", old, new]);
    repo.run_git(&["commit", "-m", "unicode rename"]);
    let oid = id(&repo);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let files = backend.commit_files(&oid).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, new);
    assert_eq!(files[0].old_path.as_deref(), Some(old));
    assert_eq!(files[0].status, CommitFileStatus::Renamed);
    let diff = preview(&repo, &oid, new, Some(old));
    assert!(diff.contains("rename from"), "{diff}");
    let native = backend
        .load_patch(PatchRequest::CommitFiles {
            oid: oid.clone(),
            paths: vec![old.into(), new.into()],
        })
        .unwrap();
    assert_eq!(native.files.len(), 1);
    assert_eq!(native.files[0].new.path.as_deref(), Some(Path::new(new)));
    write(repo.path().join(new), "later\n").unwrap();
    repo.run_git(&["commit", "-am", "later"]);
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    run(
        &repo,
        Operation::CommitFileCheckout {
            oid: oid.clone(),
            paths: vec![old.into(), new.into()],
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join(new)).unwrap(),
        "unicode rename\n"
    );
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
    repo.run_git(&["restore", "--worktree", "."]);
    repo.run_git(&["reset", "--hard", &oid]);
    run(
        &repo,
        Operation::CommitFileDiscard {
            oid,
            paths: vec![old.into(), new.into()],
        },
    )
    .unwrap();
    assert!(repo.path().join(old).exists());
    assert!(!repo.path().join(new).exists());
}

#[test]
fn commit_files_copy_preview_and_discard_leave_independently_modified_source_alone() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-copy-source");
    let original = (0..150)
        .map(|i| format!("source line {i}\n"))
        .collect::<String>();
    write(repo.path().join("source.txt"), &original).unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "source"]);
    write(repo.path().join("copy.txt"), &original).unwrap();
    write(
        repo.path().join("source.txt"),
        format!("{original}independent source change\n"),
    )
    .unwrap();
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "copy and edit source"]);
    let oid = id(&repo);
    let files = HybridGitBackend::open(repo.path())
        .unwrap()
        .commit_files(&oid)
        .unwrap();
    let copied = files.iter().find(|file| file.path == "copy.txt").unwrap();
    assert_eq!(copied.status, CommitFileStatus::Copied);
    assert_eq!(copied.old_path.as_deref(), Some("source.txt"));
    let diff = preview(&repo, &oid, "copy.txt", Some("source.txt"));
    assert!(!diff.contains("independent source change"), "{diff}");
    let native = HybridGitBackend::open(repo.path())
        .unwrap()
        .load_patch(PatchRequest::CommitFiles {
            oid: oid.clone(),
            paths: vec!["copy.txt".into()],
        })
        .unwrap();
    assert_eq!(native.files.len(), 1);
    assert_eq!(
        native.files[0].new.path.as_deref(),
        Some(Path::new("copy.txt"))
    );
    run(
        &repo,
        Operation::CommitFileDiscard {
            oid,
            paths: vec!["copy.txt".into()],
        },
    )
    .unwrap();
    assert!(!repo.path().join("copy.txt").exists());
    assert_eq!(
        std::fs::read_to_string(repo.path().join("source.txt")).unwrap(),
        format!("{original}independent source change\n")
    );
}

#[test]
fn commit_files_merge_uses_first_parent_for_listing_preview_and_checkout_rejects_rewrite() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-merge-first-parent");
    repo.run_git(&["switch", "-c", "side"]);
    write(repo.path().join("a.txt"), "from side\n").unwrap();
    repo.run_git(&["commit", "-am", "side change"]);
    repo.run_git(&["switch", "main"]);
    write(repo.path().join("b.txt"), "main only\n").unwrap();
    repo.run_git(&["commit", "-am", "main change"]);
    repo.run_git(&["merge", "--no-ff", "-m", "merge", "side"]);
    let oid = id(&repo);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    let files = backend.commit_files(&oid).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "a.txt");
    assert_eq!(files[0].status, CommitFileStatus::Modified);
    let diff = strip_ansi(&preview(&repo, &oid, "a.txt", None));
    assert!(diff.contains("+from side"), "{diff}");
    assert!(diff.contains("-a1"), "{diff}");
    let native = patch(&repo, &oid);
    assert_eq!(native.files.len(), 1);
    assert_eq!(
        native.files[0].new.path.as_deref(),
        Some(Path::new("a.txt"))
    );
    let before = state(&repo);
    let error = run(
        &repo,
        Operation::CommitFileDiscard {
            oid: oid.clone(),
            paths: vec!["a.txt".into()],
        },
    )
    .unwrap_err();
    assert!(error.message.contains("merge"), "{error:?}");
    assert_eq!(state(&repo), before);
    write(repo.path().join("a.txt"), "later\n").unwrap();
    repo.run_git(&["commit", "-am", "later"]);
    let index = repo.run_git_capture(&["ls-files", "--stage"]);
    run(
        &repo,
        Operation::CommitFileCheckout {
            oid,
            paths: vec!["a.txt".into()],
        },
    )
    .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        "from side\n"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "main only\n"
    );
    assert_eq!(repo.run_git_capture(&["ls-files", "--stage"]), index);
}

#[test]
fn commit_files_shallow_boundary_matches_native_patch_and_preview() {
    assert!(git_available(), "Git is required for this regression");
    let source = seeded_repo_with_two_files("commit-file-shallow-source");
    write(source.path().join("a.txt"), "shallow tip\n").unwrap();
    source.run_git(&["commit", "-am", "tip"]);
    let destination = TmpGitRepo::new("commit-file-shallow-destination");
    let clone_path = destination.path().join("clone");
    source.run_git(&[
        "clone",
        "--depth=1",
        "--no-local",
        "--",
        source.path().to_str().unwrap(),
        clone_path.to_str().unwrap(),
    ]);
    let repo = TmpGitRepo { path: clone_path };
    assert_eq!(
        repo.run_git_capture(&["rev-parse", "--is-shallow-repository"])
            .trim(),
        "true"
    );
    assert_eq!(
        repo.run_git_capture(&["rev-list", "--count", "HEAD"])
            .trim(),
        "1"
    );
    let oid = id(&repo);
    let native_repo = git2::Repository::open(repo.path()).unwrap();
    let boundary = native_repo
        .find_commit(git2::Oid::from_str(&oid).unwrap())
        .unwrap();
    assert_eq!(boundary.parent_count(), 0);
    let missing_parent = source.run_git_capture(&["rev-parse", "HEAD^"]);
    assert!(
        native_repo
            .find_commit(git2::Oid::from_str(missing_parent.trim()).unwrap())
            .is_err()
    );
    repo.run_git(&["config", "log.showRoot", "false"]);
    let native = patch(&repo, &oid);
    assert_eq!(native.files.len(), 1);
    assert!(native.files[0].old.oid.is_none());
    let files = HybridGitBackend::open(repo.path())
        .unwrap()
        .commit_files(&oid)
        .unwrap();
    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .all(|file| file.status == CommitFileStatus::Added)
    );
    assert!(files.iter().any(|file| file.path == "a.txt"));
    assert!(files.iter().any(|file| file.path == "b.txt"));
    let diff = strip_ansi(&preview(&repo, &oid, "a.txt", None));
    assert!(diff.contains("+shallow tip"), "{diff}");
    assert!(diff.contains("new file mode"), "{diff}");
}

#[test]
fn commit_files_root_preview_ignores_log_show_root_false() {
    assert!(git_available(), "Git is required for this regression");
    let repo = seeded_repo_with_two_files("commit-file-root-preview-config");
    repo.run_git(&["config", "log.showRoot", "false"]);
    let oid = id(&repo);
    let native = patch(&repo, &oid);
    assert_eq!(native.files.len(), 1);
    assert!(native.files[0].old.oid.is_none());
    let files = HybridGitBackend::open(repo.path())
        .unwrap()
        .commit_files(&oid)
        .unwrap();
    assert_eq!(files.len(), 2);
    assert!(
        files
            .iter()
            .all(|file| file.status == CommitFileStatus::Added)
    );
    let diff = strip_ansi(&preview(&repo, &oid, "a.txt", None));
    assert!(diff.contains("+a1"), "{diff}");
    assert!(diff.contains("new file mode"), "{diff}");
    assert!(!diff.contains("b.txt"), "{diff}");
}

#[cfg(unix)]
#[test]
fn commit_files_preserves_tabs_and_newlines_in_real_git_paths() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo_with_two_files("commit-file-unusual-paths");
    let paths = ["with\ttab.txt", "with\nnewline.txt", " leading trailing "];
    for path in paths {
        write(repo.path().join(path), path).unwrap();
    }
    repo.run_git(&["add", "."]);
    repo.run_git(&["commit", "-m", "unusual paths"]);
    let files = HybridGitBackend::open(repo.path())
        .unwrap()
        .commit_files("HEAD")
        .unwrap();
    assert_eq!(files.len(), paths.len());
    for path in paths {
        assert!(files.iter().any(|file| file.path == path));
    }
}
