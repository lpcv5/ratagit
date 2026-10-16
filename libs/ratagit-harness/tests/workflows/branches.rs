use super::*;

#[test]
fn branches_squash_merge_rename_checkout_and_previous() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-branches");
    let original = repo.run_git_capture(&["rev-parse", "HEAD"]);
    repo.run_git(&["switch", "-c", "topic"]);
    commit_file(&repo, "a.txt", "topic\n", "topic change");
    repo.run_git(&["switch", "main"]);
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_branch(runtime, "topic");
        open(runtime, Feature::Merge);
        down(runtime, 1);
        assert_operation(
            runtime,
            Operation::Merge {
                target: "topic".into(),
                squash: true,
            },
        );
        assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]), original);
        assert!(
            repo.run_git_capture(&["diff", "--cached", "--name-only"])
                .contains("a.txt")
        );
        repo.run_git(&["reset", "--hard", "HEAD"]);
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_branch(runtime, "topic");
        open(runtime, Feature::Rename);
        action(runtime, WorkflowAction::CursorHome);
        for _ in 0..5 {
            action(runtime, WorkflowAction::CursorEnd);
            action(runtime, WorkflowAction::Backspace);
        }
        input(runtime, "renamed");
        assert!(runtime.render_terminal_text().contains("Rename branch"));
        assert_operation(
            runtime,
            Operation::Rename {
                old: "topic".into(),
                new: "renamed".into(),
            },
        );
        assert!(
            runtime
                .state()
                .repo
                .branches
                .items
                .iter()
                .any(|branch| branch.name == "renamed")
        );
        open(runtime, Feature::CheckoutByName);
        input(runtime, "renamed");
        assert_operation(
            runtime,
            Operation::Checkout {
                name: "renamed".into(),
            },
        );
        assert_eq!(runtime.state().repo.status.current_branch, "renamed");
        open(runtime, Feature::CheckoutPrevious);
        assert_eq!(
            repo.run_git_capture(&["branch", "--show-current"]).trim(),
            "main"
        );
        assert_eq!(runtime.state().repo.status.current_branch, "main");
    });
}

#[test]
fn branches_remote_tracking_upstream_fast_forward_and_upstream_reset() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-tracking");
    repo.run_git(&["remote", "add", "origin", "."]);
    repo.run_git(&["switch", "-c", "topic"]);
    let topic = commit_file(&repo, "a.txt", "remote\n", "remote change");
    repo.run_git(&["update-ref", "refs/remotes/origin/topic", &topic]);
    repo.run_git(&["switch", "main"]);
    repo.run_git(&["branch", "--set-upstream-to=origin/topic", "main"]);
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_branch(runtime, "main");
        open(runtime, Feature::FastForward);
        assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]).trim(), topic);
        open(runtime, Feature::RemoteBranches);
        assert!(runtime.render_terminal_text().contains("origin/topic"));
        confirm(runtime);
        input(runtime, "tracking");
        assert_operation(
            runtime,
            Operation::CreateBranch {
                name: "tracking".into(),
                target: "origin/topic".into(),
            },
        );
        assert_eq!(runtime.state().repo.status.current_branch, "tracking");
        assert_eq!(
            repo.run_git_capture(&["rev-parse", "--abbrev-ref", "@{upstream}"])
                .trim(),
            "origin/topic"
        );
        select_branch(runtime, "tracking");
        open(runtime, Feature::Upstream);
        down(runtime, 1);
        assert_operation(
            runtime,
            Operation::Upstream {
                branch: "tracking".into(),
                target: None,
            },
        );
        assert!(
            repo.run_git_capture(&[
                "for-each-ref",
                "--format=%(upstream)",
                "refs/heads/tracking"
            ])
            .trim()
            .is_empty()
        );
        open(runtime, Feature::Upstream);
        confirm(runtime);
        input(runtime, "origin/topic");
        assert_operation(
            runtime,
            Operation::Upstream {
                branch: "tracking".into(),
                target: Some("origin/topic".into()),
            },
        );
        commit_file(&repo, "b.txt", "extra\n", "local extra");
        runtime.dispatch_ui(UiAction::RefreshAll);
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Files,
        });
        open(runtime, Feature::UpstreamReset);
        down(runtime, 2);
        confirm(runtime);
        assert!(matches!(
            runtime.state().ui.workflow.dialog,
            Some(Dialog::Confirm(_))
        ));
        action(runtime, WorkflowAction::Cancel);
        assert_eq!(
            repo.run_git_capture(&["log", "-1", "--format=%s"]).trim(),
            "local extra"
        );
        open(runtime, Feature::UpstreamReset);
        down(runtime, 2);
        confirm(runtime);
        assert_operation(
            runtime,
            Operation::Reset {
                target: "@{upstream}".into(),
                mode: ratagit_core::ResetMode::Hard,
            },
        );
        assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]).trim(), topic);
    });
}

#[test]
fn branches_invalid_names_and_dirty_merge_leave_git_unchanged() {
    if !git_available() {
        return;
    }
    use ratagit_git::GitBackendWrite;
    let repo = seeded_repo("workflow-invalid-refs");
    repo.run_git(&["branch", "topic"]);
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    for operation in [
        Operation::Rename {
            old: "topic".into(),
            new: "--bad".into(),
        },
        Operation::CreateBranch {
            name: "bad name".into(),
            target: "HEAD".into(),
        },
        Operation::Reset {
            target: "--all".into(),
            mode: ratagit_core::ResetMode::Hard,
        },
    ] {
        assert!(
            backend
                .repository_operation(&RepositoryOperation::Workflow(operation))
                .is_err()
        );
    }
    write(repo.path().join("a.txt"), "keep me\n").unwrap();
    assert!(
        backend
            .repository_operation(&RepositoryOperation::Workflow(Operation::Merge {
                target: "topic".into(),
                squash: false
            }))
            .is_err()
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
        "keep me\n"
    );
}

#[test]
fn branches_fast_forward_noncurrent_ref_preserves_dirty_worktree_and_rejects_divergence() {
    if !git_available() {
        return;
    }
    use ratagit_git::GitBackendWrite;
    let repo = seeded_repo("workflow-fast-forward-noncurrent");
    let base = repo
        .run_git_capture(&["rev-parse", "HEAD"])
        .trim()
        .to_string();
    repo.run_git(&["remote", "add", "origin", "."]);
    repo.run_git(&["switch", "-c", "topic"]);
    let topic = commit_file(&repo, "a.txt", "topic\n", "topic");
    repo.run_git(&["update-ref", "refs/remotes/origin/topic", &topic]);
    repo.run_git(&["branch", "--set-upstream-to=origin/topic", "main"]);
    write(repo.path().join("b.txt"), "dirty preserve\n").unwrap();
    let mut backend = HybridGitBackend::open(repo.path()).unwrap();
    backend
        .repository_operation(&RepositoryOperation::Workflow(Operation::FastForward {
            branch: "main".into(),
        }))
        .unwrap();
    assert_eq!(repo.run_git_capture(&["rev-parse", "main"]).trim(), topic);
    assert_eq!(
        repo.run_git_capture(&["branch", "--show-current"]).trim(),
        "topic"
    );
    assert_eq!(
        std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
        "dirty preserve\n"
    );
    repo.run_git(&["reset", "--hard", "HEAD"]);
    repo.run_git(&["switch", "-c", "diverged", &base]);
    let diverged = commit_file(&repo, "b.txt", "diverged\n", "diverged");
    repo.run_git(&["branch", "--set-upstream-to=origin/topic", "diverged"]);
    repo.run_git(&["switch", "topic"]);
    assert!(
        backend
            .repository_operation(&RepositoryOperation::Workflow(Operation::FastForward {
                branch: "diverged".into()
            }))
            .is_err()
    );
    assert_eq!(
        repo.run_git_capture(&["rev-parse", "diverged"]).trim(),
        diverged
    );
}

#[test]
fn dirty_checkout_by_name_rejects_a_file_path_instead_of_reporting_success() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("review-checkout-file-name");
    write(repo.path().join("b.txt"), "keep dirty data\n").unwrap();
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_branch(runtime, "main");
        open(runtime, Feature::CheckoutByName);
        input(runtime, "a.txt");
        confirm(runtime);
        assert!(runtime.state().ui.branches.auto_stash_confirm.active);
        runtime.dispatch_ui(UiAction::ConfirmAutoStash);
        assert!(
            runtime
                .state()
                .notices
                .iter()
                .any(|line| line.contains("Failed to checkout branch: a.txt"))
        );
        assert_eq!(
            repo.run_git_capture(&["branch", "--show-current"]).trim(),
            "main"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
            "keep dirty data\n"
        );
        assert!(repo.run_git_capture(&["stash", "list"]).trim().is_empty());
    });
}

#[test]
fn checkout_auto_stash_preserves_existing_stashes_and_partial_staging() {
    if !git_available() {
        return;
    }
    for target in ["topic", "a.txt"] {
        let repo = seeded_repo("review-checkout-existing-stash");
        repo.run_git(&["branch", "topic"]);
        write(repo.path().join("a.txt"), "older stashed work\n").unwrap();
        repo.run_git(&["stash", "push", "-m", "keep existing stash"]);
        let original_stash = repo.run_git_capture(&["stash", "list", "--format=%H"]);
        write(repo.path().join("b.txt"), "staged data\n").unwrap();
        repo.run_git(&["add", "--", "b.txt"]);
        write(repo.path().join("b.txt"), "staged data\nunstaged data\n").unwrap();
        write(repo.path().join("new.txt"), "untracked data\n").unwrap();
        let index = repo.run_git_capture(&["diff", "--cached"]);
        let worktree = repo.run_git_capture(&["diff"]);
        let status = repo.run_git_capture(&["status", "--short"]);
        let mut runtime = runtime_for(&repo);
        with_case_artifacts(&repo, &mut runtime, |runtime| {
            runtime.dispatch_ui(UiAction::RefreshAll);
            select_branch(runtime, "main");
            open(runtime, Feature::CheckoutByName);
            input(runtime, target);
            confirm(runtime);
            assert!(runtime.state().ui.branches.auto_stash_confirm.active);
            assert_eq!(
                update(
                    &mut runtime.state().clone(),
                    Action::Ui(UiAction::ConfirmAutoStash)
                ),
                vec![CoreCommand::CheckoutBranch {
                    name: target.into(),
                    auto_stash: true,
                }]
            );
            runtime.dispatch_ui(UiAction::ConfirmAutoStash);
            assert_eq!(
                repo.run_git_capture(&["branch", "--show-current"]).trim(),
                if target == "topic" { "topic" } else { "main" }
            );
            assert_eq!(repo.run_git_capture(&["diff", "--cached"]), index);
            assert_eq!(repo.run_git_capture(&["diff"]), worktree);
            assert_eq!(repo.run_git_capture(&["status", "--short"]), status);
            assert_eq!(
                repo.run_git_capture(&["stash", "list", "--format=%H"]),
                original_stash
            );
            assert_eq!(
                std::fs::read_to_string(repo.path().join("new.txt")).unwrap(),
                "untracked data\n"
            );
        });
    }
}
