use super::*;
use ratagit_core::workflow::{ConflictSide, FileFilter, SequenceAction, StashMode};

#[test]
fn files_filter_flat_tree_and_stash_modes_preserve_index_boundaries() {
    if !git_available() {
        return;
    }
    for (choice, mode) in [
        (1, StashMode::Staged),
        (2, StashMode::Unstaged),
        (0, StashMode::All),
    ] {
        let repo = seeded_repo("workflow-stash");
        write(repo.path().join("a.txt"), "staged\n").unwrap();
        repo.run_git(&["add", "a.txt"]);
        write(repo.path().join("b.txt"), "unstaged\n").unwrap();
        write(repo.path().join("untracked.txt"), "untracked\n").unwrap();
        let mut runtime = runtime_for(&repo);
        with_case_artifacts(&repo, &mut runtime, |runtime| {
            runtime.dispatch_ui(UiAction::RefreshAll);
            open(runtime, Feature::Filter);
            assert!(
                runtime
                    .render_terminal_text()
                    .contains("Filter files by status")
            );
            down(runtime, 1);
            confirm(runtime);
            assert_eq!(runtime.state().ui.files.filter, FileFilter::Staged);
            assert_eq!(
                ratagit_core::file_tree_rows(&runtime.state().ui.files).len(),
                1
            );
            assert_eq!(runtime.state().repo.files.items.len(), 3);
            open(runtime, Feature::ToggleTree);
            assert!(runtime.state().ui.files.flat);
            open(runtime, Feature::Stash);
            assert!(
                runtime
                    .render_terminal_text()
                    .contains("Staged changes only")
            );
            action(runtime, WorkflowAction::Cancel);
            assert!(repo.run_git_capture(&["stash", "list"]).is_empty());
            open(runtime, Feature::Stash);
            down(runtime, choice);
            assert_operation(runtime, Operation::Stash(mode));
            assert_eq!(runtime.state().repo.stash.items.len(), 1);
            let status = repo.run_git_capture(&["status", "--short"]);
            match mode {
                StashMode::All => assert!(status.is_empty()),
                StashMode::Staged => {
                    assert!(!status.contains("a.txt"));
                    assert!(status.contains("b.txt"));
                }
                StashMode::Unstaged => {
                    assert!(status.contains("a.txt"));
                    assert!(!status.contains("b.txt"));
                }
            }
        });
    }
}

fn conflicted_repo(name: &str) -> TmpGitRepo {
    let repo = seeded_repo(name);
    repo.run_git(&["switch", "-c", "topic"]);
    commit_file(&repo, "a.txt", "topic\n", "topic change");
    repo.run_git(&["switch", "main"]);
    commit_file(&repo, "a.txt", "main\n", "main change");
    repo
}

fn start_conflict(runtime: &mut TracedCase<'_>) {
    runtime.dispatch_ui(UiAction::RefreshAll);
    select_branch(runtime, "topic");
    open(runtime, Feature::Merge);
    assert!(
        runtime
            .render_terminal_text()
            .contains("Merge selected branch")
    );
    assert_operation(
        runtime,
        Operation::Merge {
            target: "topic".into(),
            squash: false,
        },
    );
    assert!(
        runtime
            .state()
            .repo
            .files
            .items
            .iter()
            .any(|file| file.conflicted)
    );
    runtime.dispatch_ui(UiAction::FocusPanel {
        panel: PanelFocus::Files,
    });
}

#[test]
fn files_conflict_choose_theirs_then_continue_merge() {
    if !git_available() {
        return;
    }
    let repo = conflicted_repo("workflow-conflict");
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        start_conflict(runtime);
        open(runtime, Feature::Conflict);
        down(runtime, 1);
        confirm(runtime);
        assert!(matches!(
            runtime.state().ui.workflow.dialog,
            Some(Dialog::Confirm(_))
        ));
        action(runtime, WorkflowAction::Cancel);
        assert!(
            repo.run_git_capture(&["diff", "--name-only", "--diff-filter=U"])
                .contains("a.txt")
        );
        open(runtime, Feature::Conflict);
        down(runtime, 1);
        confirm(runtime);
        assert_operation(
            runtime,
            Operation::Resolve {
                paths: vec!["a.txt".into()],
                side: ConflictSide::Theirs,
            },
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "topic\n"
        );
        assert!(
            !runtime
                .state()
                .repo
                .files
                .items
                .iter()
                .any(|file| file.conflicted)
        );
        open(runtime, Feature::Sequence);
        assert_operation(runtime, Operation::Sequence(SequenceAction::Continue));
        assert_eq!(
            repo.run_git_capture(&["rev-list", "--parents", "-1", "HEAD"])
                .split_whitespace()
                .count(),
            3
        );
        assert!(repo.run_git_capture(&["status", "--short"]).is_empty());
    });
}

#[test]
fn files_conflict_mark_resolved_refuses_markers_and_abort_restores_head() {
    if !git_available() {
        return;
    }
    let repo = conflicted_repo("workflow-abort");
    let original = repo.run_git_capture(&["rev-parse", "HEAD"]);
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        start_conflict(runtime);
        open(runtime, Feature::Conflict);
        down(runtime, 2);
        assert_operation(
            runtime,
            Operation::Resolve {
                paths: vec!["a.txt".into()],
                side: ConflictSide::Resolved,
            },
        );
        assert!(
            runtime
                .state()
                .notices
                .iter()
                .any(|line| line.contains("conflict markers remain"))
        );
        open(runtime, Feature::Sequence);
        down(runtime, 1);
        confirm(runtime);
        assert!(
            runtime
                .render_terminal_text()
                .contains("Confirm repository change")
        );
        assert_operation(runtime, Operation::Sequence(SequenceAction::Abort));
        assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]), original);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "main\n"
        );
        assert!(runtime.state().repo.files.items.is_empty());
    });
}

#[test]
fn files_no_verify_commit_bypasses_hook_and_external_message_uses_commit_flow() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-commit-hook");
    let hook = repo.path().join(".git/hooks/pre-commit");
    write(&hook, "#!/bin/sh\nexit 1\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    write(repo.path().join("a.txt"), "staged\n").unwrap();
    repo.run_git(&["add", "a.txt"]);
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        open(runtime, Feature::CommitNoVerify);
        input(runtime, "skip hook");
        assert_operation(
            runtime,
            Operation::Commit {
                message: "skip hook".into(),
                skip_hooks: true,
            },
        );
        assert_eq!(
            repo.run_git_capture(&["log", "-1", "--format=%s"]).trim(),
            "skip hook"
        );
        std::fs::remove_file(&hook).unwrap();
        write(repo.path().join("b.txt"), "editor\n").unwrap();
        runtime.dispatch_ui(UiAction::RefreshAll);
        action(
            runtime,
            WorkflowAction::ExternalCommit("from editor\n\nbody".into()),
        );
        assert!(runtime.state().ui.stage_all_confirm.active);
        runtime.dispatch_ui(UiAction::ConfirmStageAll);
        assert_eq!(
            repo.run_git_capture(&["log", "-1", "--format=%B"]).trim(),
            "from editor\n\nbody"
        );
    });
}

#[test]
fn files_rebase_conflict_edit_mark_resolved_and_continue() {
    if !git_available() {
        return;
    }
    let repo = conflicted_repo("workflow-rebase-continue");
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_branch(runtime, "topic");
        runtime.dispatch_ui(UiAction::OpenBranchRebaseMenu);
        runtime.dispatch_ui(UiAction::ConfirmBranchRebaseMenu);
        assert!(
            runtime
                .state()
                .repo
                .files
                .items
                .iter()
                .any(|file| file.conflicted)
        );
        write(repo.path().join("a.txt"), "manually resolved\n").unwrap();
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Files,
        });
        open(runtime, Feature::Conflict);
        down(runtime, 2);
        assert_operation(
            runtime,
            Operation::Resolve {
                paths: vec!["a.txt".into()],
                side: ConflictSide::Resolved,
            },
        );
        open(runtime, Feature::Sequence);
        assert_operation(runtime, Operation::Sequence(SequenceAction::Continue));
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "manually resolved\n"
        );
        assert_eq!(
            repo.run_git_capture(&["branch", "--show-current"]).trim(),
            "main"
        );
        assert!(repo.run_git_capture(&["status", "--short"]).is_empty());
        assert_eq!(
            repo.run_git_capture(&["rev-list", "--count", "HEAD"])
                .trim(),
            "3"
        );
    });
}

#[test]
fn files_conflict_literal_paths_do_not_resolve_neighboring_files() {
    if !git_available() {
        return;
    }
    use ratagit_git::GitBackendWrite;
    let repo = seeded_repo("workflow-conflict-literal");
    commit_file(&repo, "[a].txt", "base\n", "base literal");
    commit_file(&repo, "a.txt", "base\n", "base neighbor");
    repo.run_git(&["switch", "-c", "topic"]);
    commit_file(&repo, "[a].txt", "topic\n", "topic literal");
    commit_file(&repo, "a.txt", "topic\n", "topic neighbor");
    repo.run_git(&["switch", "main"]);
    commit_file(&repo, "[a].txt", "main\n", "main literal");
    commit_file(&repo, "a.txt", "main\n", "main neighbor");
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    assert!(
        backend
            .repository_operation(&RepositoryOperation::Workflow(Operation::Merge {
                target: "topic".into(),
                squash: false
            }))
            .is_err()
    );
    backend
        .repository_operation(&RepositoryOperation::Workflow(Operation::Resolve {
            paths: vec!["[a].txt".into()],
            side: ConflictSide::Ours,
        }))
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(repo.path().join("[a].txt")).unwrap(),
        "main\n"
    );
    let unresolved = repo.run_git_capture(&["diff", "--name-only", "--diff-filter=U"]);
    assert_eq!(unresolved.trim(), "a.txt");
    assert!(
        std::fs::read_to_string(repo.path().join("a.txt"))
            .unwrap()
            .contains("<<<<<<<")
    );
}

#[test]
fn files_partially_staged_file_appears_in_both_filters_and_stage_all_finishes_staging() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-partially-staged-filter");
    write(repo.path().join("a.txt"), "staged version\n").unwrap();
    repo.run_git(&["add", "a.txt"]);
    write(repo.path().join("a.txt"), "unstaged version\n").unwrap();
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        assert!(
            runtime.state().repo.files.items[0].staged
                && runtime.state().repo.files.items[0].unstaged
        );
        assert_eq!(
            runtime.state().repo.status.summary,
            "staged: 1, unstaged: 1"
        );
        for choice in [1, 2] {
            open(runtime, Feature::Filter);
            down(runtime, choice);
            confirm(runtime);
            assert_eq!(
                ratagit_core::file_tree_rows(&runtime.state().ui.files).len(),
                1
            );
            assert!(runtime.render_terminal_text().contains("a.txt"));
        }
        runtime.dispatch_ui(UiAction::ToggleStageAll);
        assert_eq!(
            repo.run_git_capture(&["show", ":a.txt"]),
            "unstaged version\n"
        );
        assert!(repo.run_git_capture(&["diff", "--name-only"]).is_empty());
        assert!(
            runtime
                .state()
                .repo
                .files
                .items
                .iter()
                .all(|file| file.staged && !file.unstaged)
        );
    });
}

#[test]
fn mark_resolved_rejects_custom_sized_git_conflict_markers() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("review-conflict-marker-size");
    commit_file(
        &repo,
        ".gitattributes",
        "a.txt conflict-marker-size=12\n",
        "marker size",
    );
    repo.run_git(&["switch", "-c", "topic"]);
    commit_file(&repo, "a.txt", "topic\n", "topic change");
    repo.run_git(&["switch", "main"]);
    commit_file(&repo, "a.txt", "main\n", "main change");
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        start_conflict(runtime);
        let original = std::fs::read(repo.path().join("a.txt")).unwrap();
        assert!(String::from_utf8_lossy(&original).starts_with("<<<<<<<<<<<< "));
        open(runtime, Feature::Conflict);
        down(runtime, 2);
        assert_operation(
            runtime,
            Operation::Resolve {
                paths: vec!["a.txt".into()],
                side: ConflictSide::Resolved,
            },
        );
        assert!(
            runtime
                .state()
                .notices
                .iter()
                .any(|line| line.contains("conflict markers remain"))
        );
        assert!(
            repo.run_git_capture(&["diff", "--name-only", "--diff-filter=U"])
                .contains("a.txt")
        );
        assert_eq!(std::fs::read(repo.path().join("a.txt")).unwrap(), original);
    });
}
