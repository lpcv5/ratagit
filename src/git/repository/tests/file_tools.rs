use super::*;
use pretty_assertions::assert_eq;

#[test]
fn test_toggle_stage_all_uses_fresh_status_and_preserves_worktree() {
    let (dir, backend) = init_repo_with_commit();
    let raw = git2::Repository::open(dir.path()).unwrap();
    raw.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    write_file(&dir.path().join("tracked.txt"), "staged\n");
    backend.stage(Path::new("tracked.txt")).unwrap();
    write_file(&dir.path().join("tracked.txt"), "newer\n");
    fs::create_dir(dir.path().join("new dir")).unwrap();
    write_file(&dir.path().join("new dir/new 文件.txt"), "new\n");
    write_file(&dir.path().join(".gitignore"), "/ignored\n");
    write_file(&dir.path().join("ignored"), "excluded\n");

    assert!(backend.toggle_stage_all().unwrap());
    let status = backend.status().unwrap();
    assert!(status.unstaged.is_empty() && status.untracked.is_empty());
    assert_eq!(status.staged.len(), 3);
    assert!(!backend.toggle_stage_all().unwrap());
    assert!(backend.status().unwrap().staged.is_empty());
    assert_eq!(
        fs::read_to_string(dir.path().join("tracked.txt")).unwrap(),
        "newer\n"
    );
    assert_eq!(
        fs::read_to_string(dir.path().join("new dir/new 文件.txt")).unwrap(),
        "new\n"
    );
    fs::remove_file(dir.path().join("tracked.txt")).unwrap();
    assert!(backend.toggle_stage_all().unwrap());
    assert!(backend
        .status()
        .unwrap()
        .staged
        .iter()
        .any(|entry| entry.status == FileStatus::Deleted));
    assert!(!backend.toggle_stage_all().unwrap());
    assert!(!dir.path().join("tracked.txt").exists());
}

#[test]
fn test_toggle_stage_all_in_unborn_and_clean_repositories() {
    let dir = TempDir::new().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    let backend = Git2Repository::open(dir.path()).unwrap();
    assert!(backend.toggle_stage_all().is_err());
    assert!(backend.amend_head().is_err());
    write_file(&dir.path().join("new.txt"), "new\n");
    assert!(backend.toggle_stage_all().unwrap());
    assert!(!backend.toggle_stage_all().unwrap());
    assert_eq!(backend.status().unwrap().untracked.len(), 1);
    assert_eq!(
        fs::read_to_string(dir.path().join("new.txt")).unwrap(),
        "new\n"
    );
}

#[test]
fn test_amend_preserves_message_author_and_parents_and_only_uses_index() {
    let (dir, backend) = init_repo_with_commit();
    let raw = git2::Repository::open(dir.path()).unwrap();
    raw.config()
        .unwrap()
        .set_bool("core.autocrlf", false)
        .unwrap();
    write_file(&dir.path().join("tracked.txt"), "second\n");
    backend.stage(Path::new("tracked.txt")).unwrap();
    backend.commit("second title\n\nbody\n").unwrap();
    let old = raw.head().unwrap().peel_to_commit().unwrap();
    write_file(&dir.path().join("tracked.txt"), "amended\n");
    backend.stage(Path::new("tracked.txt")).unwrap();
    write_file(&dir.path().join("tracked.txt"), "unstaged\n");
    let oid = backend.amend_head().unwrap();
    let amended = raw.find_commit(git2::Oid::from_str(&oid).unwrap()).unwrap();
    assert_ne!(amended.id(), old.id());
    assert_eq!(amended.message_bytes(), old.message_bytes());
    assert!(amended.author() == old.author());
    assert_eq!(
        amended.parent_ids().collect::<Vec<_>>(),
        old.parent_ids().collect::<Vec<_>>()
    );
    let blob = amended
        .tree()
        .unwrap()
        .get_path(Path::new("tracked.txt"))
        .unwrap()
        .to_object(&raw)
        .unwrap()
        .peel_to_blob()
        .unwrap();
    assert_eq!(blob.content(), b"amended\n");
    assert!(backend.status().unwrap().staged.is_empty());
    assert_eq!(backend.status().unwrap().unstaged.len(), 1);
    assert_eq!(
        fs::read_to_string(dir.path().join("tracked.txt")).unwrap(),
        "unstaged\n"
    );
}

#[test]
fn test_amend_root_commit_and_reject_repository_operation() {
    let (dir, backend) = init_repo_with_commit();
    write_file(&dir.path().join("tracked.txt"), "root amended\n");
    backend.stage(Path::new("tracked.txt")).unwrap();
    let oid = backend.amend_head().unwrap();
    let raw = git2::Repository::open(dir.path()).unwrap();
    assert_eq!(
        raw.find_commit(git2::Oid::from_str(&oid).unwrap())
            .unwrap()
            .parent_count(),
        0
    );
    fs::write(raw.path().join("MERGE_HEAD"), format!("{oid}\n")).unwrap();
    assert!(backend.amend_head().is_err());
    assert_eq!(raw.head().unwrap().target().unwrap().to_string(), oid);
}

#[test]
fn test_ignore_literal_unicode_paths_preserves_existing_rules_and_is_idempotent() {
    let (dir, backend) = init_repo_with_commit();
    let path = Path::new("[draft] 文件 #!.txt");
    write_file(&dir.path().join(path), "keep me\n");
    write_file(&dir.path().join(".gitignore"), "# existing\r\n*.bak");
    backend.ignore_file(path).unwrap();
    let text = fs::read_to_string(dir.path().join(".gitignore")).unwrap();
    assert_eq!(
        text,
        "# existing\r\n*.bak\r\n/\\[draft\\]\\ 文件\\ \\#\\!.txt\r\n"
    );
    assert!(git2::Repository::open(dir.path())
        .unwrap()
        .is_path_ignored(path)
        .unwrap());
    backend.ignore_file(path).unwrap();
    assert_eq!(
        fs::read_to_string(dir.path().join(".gitignore")).unwrap(),
        text
    );
    assert!(backend.status().unwrap().staged.is_empty());
    assert_eq!(
        fs::read_to_string(dir.path().join(path)).unwrap(),
        "keep me\n"
    );
}

#[test]
fn test_ignore_directory_is_anchored_and_rejects_tracked_or_invalid_paths() {
    let (dir, backend) = init_repo_with_commit();
    fs::create_dir(dir.path().join("build")).unwrap();
    write_file(&dir.path().join("build/output"), "artifact\n");
    backend.ignore_file(Path::new("build")).unwrap();
    let before = fs::read(dir.path().join(".gitignore")).unwrap();
    assert_eq!(before, b"/build/\n");
    for path in ["tracked.txt", "..", "../escape", ".git/config", "missing"] {
        assert!(backend.ignore_file(Path::new(path)).is_err(), "{path}");
    }
    assert_eq!(fs::read(dir.path().join(".gitignore")).unwrap(), before);
    assert!(backend.prepare_editor(Path::new("build")).is_err());
    assert!(backend.prepare_editor(Path::new("../escape")).is_err());
}

#[cfg(unix)]
#[test]
fn test_ignore_rejects_symlinked_gitignore_and_escapes_glob_characters() {
    let (dir, backend) = init_repo_with_commit();
    let outside = TempDir::new().unwrap();
    write_file(&outside.path().join("rules"), "untouched\n");
    std::os::unix::fs::symlink(outside.path().join("rules"), dir.path().join(".gitignore"))
        .unwrap();
    write_file(&dir.path().join("a*?.txt"), "keep\n");
    assert!(backend.ignore_file(Path::new("a*?.txt")).is_err());
    assert_eq!(
        fs::read(outside.path().join("rules")).unwrap(),
        b"untouched\n"
    );
    fs::remove_file(dir.path().join(".gitignore")).unwrap();
    backend.ignore_file(Path::new("a*?.txt")).unwrap();
    assert_eq!(
        fs::read(dir.path().join(".gitignore")).unwrap(),
        b"/a\\*\\?.txt\n"
    );
}
