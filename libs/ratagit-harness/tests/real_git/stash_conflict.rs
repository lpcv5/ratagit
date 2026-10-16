use super::*;

#[test]
fn real_git_stash_pop_conflict_refreshes_files_and_preserves_stash() {
    if !git_available() {
        eprintln!("git is unavailable, skipping stash pop conflict regression");
        return;
    }

    let repo = seeded_repo("harness-stash-pop-conflict");
    write(repo.path().join("a.txt"), "stashed a\n").unwrap();
    write(repo.path().join("b.txt"), "stashed b\n").unwrap();
    write(repo.path().join("new.txt"), "stashed untracked\n").unwrap();
    repo.run_git(&[
        "stash",
        "push",
        "--include-untracked",
        "-m",
        "conflicting stash",
    ]);
    let saved_stash = repo.run_git_capture(&["rev-parse", "stash@{0}"]);
    write(repo.path().join("a.txt"), "committed a\n").unwrap();
    repo.run_git(&["add", "--", "a.txt"]);
    repo.run_git(&["commit", "-m", "conflicting change"]);
    let saved_head = repo.run_git_capture(&["rev-parse", "HEAD"]);
    let mut runtime = runtime_for(&repo);

    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        assert!(runtime.state().repo.files.items.is_empty());
        assert_eq!(runtime.state().repo.stash.items.len(), 1);
        let refresh_count = runtime.state().repo.status.refresh_count;

        // Pop alone must settle into refreshed Files state, without a manual RefreshAll.
        runtime.dispatch_ui(UiAction::StashPopSelected);

        let status = repo.run_git_capture(&["status", "--porcelain=v1", "--untracked-files=all"]);
        assert_eq!(
            status.lines().collect::<Vec<_>>(),
            ["UU a.txt", "M  b.txt", "?? new.txt"]
        );
        assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]), saved_head);
        assert_eq!(
            repo.run_git_capture(&["rev-parse", "stash@{0}"]),
            saved_stash
        );
        let unmerged = repo.run_git_capture(&["ls-files", "--unmerged"]);
        assert_eq!(unmerged.lines().count(), 3);
        assert!(unmerged.lines().all(|line| line.ends_with("\ta.txt")));
        let conflicted = std::fs::read_to_string(repo.path().join("a.txt")).unwrap();
        assert!(conflicted.contains("<<<<<<<"));
        assert!(conflicted.contains("committed a"));
        assert!(conflicted.contains("stashed a"));
        assert_eq!(repo.run_git_capture(&["show", ":b.txt"]), "stashed b\n");
        assert_eq!(
            std::fs::read_to_string(repo.path().join("new.txt")).unwrap(),
            "stashed untracked\n"
        );

        let files = &runtime.state().repo.files.items;
        assert_eq!(files.len(), 3);
        assert!(
            files
                .iter()
                .any(|file| file.path == "a.txt" && file.conflicted)
        );
        assert!(files.iter().any(|file| file.path == "b.txt" && file.staged));
        assert!(
            files
                .iter()
                .any(|file| file.path == "new.txt" && file.untracked)
        );
        assert_eq!(runtime.state().repo.stash.items.len(), 1);
        assert!(
            runtime.state().repo.stash.items[0]
                .summary
                .contains("conflicting stash")
        );

        // Completion state and the retained failure notice trace the failed mutation and refresh.
        assert_eq!(runtime.state().last_operation.as_deref(), Some("stash_pop"));
        assert_eq!(runtime.state().repo.status.refresh_count, refresh_count + 1);
        assert!(!runtime.state().work.refresh.refresh_pending);
        assert!(runtime.state().work.mutation.operation_pending.is_none());
        assert!(
            runtime
                .state()
                .notices
                .iter()
                .any(|notice| { notice.starts_with("Failed to stash pop: stash@{0}:") })
        );
        let screen = runtime.render_terminal_text();
        for expected in [
            "U a.txt",
            "M b.txt",
            "? new.txt",
            "Failed to stash pop: stash@{0}:",
        ] {
            assert!(screen.contains(expected), "missing {expected:?}\n{screen}");
        }
        assert!(!screen.contains("Stash popped:"));
    });
}
