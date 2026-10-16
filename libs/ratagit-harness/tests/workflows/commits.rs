use super::*;
use ratagit_core::workflow::SequenceAction;

#[test]
fn commits_copy_paste_revert_branch_tag_and_reset() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-commits");
    let base = repo
        .run_git_capture(&["rev-parse", "HEAD"])
        .trim()
        .to_string();
    repo.run_git(&["switch", "-c", "topic"]);
    commit_file(&repo, "a.txt", "topic a\n", "first");
    commit_file(&repo, "b.txt", "topic b\n", "second");
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        });
        runtime.dispatch_ui(UiAction::EnterCommitsMultiSelect);
        runtime.dispatch_ui(UiAction::MoveDown);
        open(runtime, Feature::CopyCommits);
        assert_eq!(runtime.state().ui.workflow.copied_commits.len(), 2);
        runtime.dispatch_ui(UiAction::ExitCommitsMultiSelect);
        select_branch(runtime, "main");
        runtime.dispatch_ui(UiAction::CheckoutSelectedBranch);
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        });
        open(runtime, Feature::PasteCommits);
        assert!(runtime.state().ui.workflow.copied_commits.is_empty());
        assert_eq!(
            repo.run_git_capture(&["log", "-2", "--format=%s"]).trim(),
            "second\nfirst"
        );
        let first = repo
            .run_git_capture(&["rev-parse", "HEAD~1"])
            .trim()
            .to_string();
        let top = repo
            .run_git_capture(&["rev-parse", "HEAD"])
            .trim()
            .to_string();
        select_commit(runtime, &top);
        open(runtime, Feature::Revert);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
            "b1\n"
        );
        assert!(
            repo.run_git_capture(&["log", "-1", "--format=%s"])
                .contains("Revert")
        );
        select_commit(runtime, &first);
        open(runtime, Feature::BranchFromCommit);
        input(runtime, "from-commit");
        assert_operation(
            runtime,
            Operation::CreateBranch {
                name: "from-commit".into(),
                target: first.clone(),
            },
        );
        assert_eq!(
            repo.run_git_capture(&["rev-parse", "from-commit"]).trim(),
            first
        );
        select_commit(runtime, &first);
        open(runtime, Feature::Tag);
        input(runtime, "v-test");
        assert_operation(
            runtime,
            Operation::Tag {
                name: "v-test".into(),
                target: first.clone(),
            },
        );
        assert_eq!(repo.run_git_capture(&["rev-parse", "v-test"]).trim(), first);
        select_commit(runtime, &base);
        open(runtime, Feature::Reset);
        assert!(
            runtime
                .render_terminal_text()
                .contains("Soft (keep staged changes)")
        );
        assert_operation(
            runtime,
            Operation::Reset {
                target: base.clone(),
                mode: ratagit_core::ResetMode::Soft,
            },
        );
        assert_eq!(repo.run_git_capture(&["rev-parse", "HEAD"]).trim(), base);
        assert!(
            !repo
                .run_git_capture(&["diff", "--cached", "--name-only"])
                .is_empty()
        );
    });
}

#[test]
fn commits_fixup_autosquash_and_move_preserve_content_and_order() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-reorder");
    let first = commit_file(&repo, "a.txt", "first\n", "first");
    commit_file(&repo, "b.txt", "second\n", "second");
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_commit(runtime, &first);
        open(runtime, Feature::MoveUp);
        assert!(matches!(
            runtime.state().ui.workflow.dialog,
            Some(Dialog::Confirm(_))
        ));
        assert_operation(
            runtime,
            Operation::MoveCommit {
                id: first,
                up: true,
            },
        );
        assert_eq!(
            repo.run_git_capture(&["log", "-2", "--format=%s"]).trim(),
            "first\nsecond"
        );
        let id = repo
            .run_git_capture(&["rev-parse", "HEAD"])
            .trim()
            .to_string();
        write(repo.path().join("a.txt"), "fixed\n").unwrap();
        repo.run_git(&["add", "a.txt"]);
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_commit(runtime, &id);
        open(runtime, Feature::Fixup);
        assert!(
            repo.run_git_capture(&["log", "-1", "--format=%s"])
                .starts_with("fixup! first")
        );
        select_commit(runtime, &id);
        open(runtime, Feature::Autosquash);
        assert_operation(runtime, Operation::Autosquash(id));
        assert_eq!(
            repo.run_git_capture(&["rev-list", "--count", "HEAD"])
                .trim(),
            "3"
        );
        assert_eq!(
            repo.run_git_capture(&["log", "-2", "--format=%s"]).trim(),
            "first\nsecond"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "fixed\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
            "second\n"
        );
    });
}

#[test]
fn commits_cherry_pick_conflict_can_skip_without_losing_original_commit() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("workflow-pick-skip");
    repo.run_git(&["switch", "-c", "topic"]);
    let topic = commit_file(&repo, "a.txt", "topic\n", "topic");
    repo.run_git(&["switch", "main"]);
    let original = commit_file(&repo, "a.txt", "main\n", "main");
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_branch(runtime, "topic");
        runtime.dispatch_ui(UiAction::OpenBranchCommitsPanel);
        assert_eq!(
            ratagit_core::commit_key(&runtime.state().repo.branches.commits[0]),
            topic
        );
        open(runtime, Feature::CopyCommits);
        assert!(runtime.state().ui.workflow.copied_commits.contains(&topic));
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        });
        open(runtime, Feature::PasteCommits);
        assert!(
            runtime
                .state()
                .repo
                .files
                .items
                .iter()
                .any(|file| file.conflicted)
        );
        assert!(!runtime.state().ui.workflow.copied_commits.is_empty());
        open(runtime, Feature::Sequence);
        down(runtime, 2);
        assert_operation(runtime, Operation::Sequence(SequenceAction::Skip));
        assert_eq!(
            repo.run_git_capture(&["rev-parse", "HEAD"]).trim(),
            original
        );
        assert!(repo.run_git_capture(&["status", "--short"]).is_empty());
    });
}

#[test]
fn autosquash_squash_messages_do_not_start_the_configured_editor() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("review-autosquash-editor");
    let first = commit_file(&repo, "a.txt", "first\n", "first");
    commit_file(&repo, "a.txt", "combined\n", "squash! first");
    repo.run_git(&["config", "core.editor", "ratagit-editor-must-not-run"]);
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_ui(UiAction::RefreshAll);
        select_commit(runtime, &first);
        open(runtime, Feature::Autosquash);
        assert_operation(runtime, Operation::Autosquash(first));
        assert_eq!(
            repo.run_git_capture(&["rev-list", "--count", "HEAD"])
                .trim(),
            "2"
        );
        assert!(repo.run_git_capture(&["status", "--short"]).is_empty());
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "combined\n"
        );
    });
}
