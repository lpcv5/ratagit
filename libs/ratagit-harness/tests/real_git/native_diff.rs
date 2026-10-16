use super::*;

#[test]
fn test_real_git_partial_stage_unstage_discard_ui_flow() {
    let repo = seeded_repo("harness-partial");
    repo.run_git(&["config", "core.autocrlf", "false"]);
    write(repo.path().join("a.txt"), "a1\nkeep\nend\n").unwrap();
    repo.run_git(&["add", "a.txt"]);
    repo.run_git(&["commit", "-m", "extend"]);
    write(repo.path().join("a.txt"), "FIRST\nkeep\nLAST\n").unwrap();
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_all(&[UiAction::RefreshAll, UiAction::OpenFilePatch]);
        assert!(runtime.render_terminal_text().contains("FIRST"));
        assert!(runtime.render_terminal_text().contains("-a1"));
        assert!(!runtime.render_terminal_text().contains("j/k:move"));
        runtime.dispatch_ui(UiAction::ApplyPatchSelection);
        assert_eq!(
            repo.run_git_capture(&["show", ":a.txt"]),
            "FIRST\nkeep\nend\n"
        );
        assert!(
            runtime
                .render_terminal_text()
                .contains("Partial Stage completed")
        );
        assert!(runtime.state().ui.details.patch.staged);
        runtime.dispatch_ui(UiAction::ApplyPatchSelection);
        assert_eq!(repo.run_git_capture(&["show", ":a.txt"]), "a1\nkeep\nend\n");
        assert!(
            runtime
                .render_terminal_text()
                .contains("Partial Unstage completed")
        );
        assert!(!runtime.state().ui.details.patch.staged);
        runtime.dispatch_ui(UiAction::RequestPatchDiscard);
        assert!(
            runtime
                .render_terminal_text()
                .contains("Discard selected lines?")
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "FIRST\nkeep\nLAST\n"
        );
        runtime.dispatch_ui(UiAction::ConfirmPatchDiscard);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "a1\nkeep\nLAST\n"
        );
        assert!(
            runtime
                .render_terminal_text()
                .contains("Partial Discard completed")
        );
    });
}

#[test]
fn test_native_diff_content_survives_enter_and_leave_without_git_changes() {
    let repo = seeded_repo("harness-native-diff");
    let before = "first\ncontext\nlast\n";
    let after = "FIRST\ncontext\nLAST\n";
    write(repo.path().join("a.txt"), before).unwrap();
    repo.run_git(&["add", "a.txt"]);
    repo.run_git(&["commit", "-m", "context"]);
    write(repo.path().join("a.txt"), after).unwrap();
    let expected = repo.run_git_capture(&[
        "diff",
        "--no-color",
        "--no-ext-diff",
        "--abbrev=7",
        "--",
        "a.txt",
    ]);
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        for action in [
            UiAction::RefreshAll,
            UiAction::OpenFilePatch,
            UiAction::CloseFilePatch,
        ] {
            runtime.dispatch_ui(action);
            let screen = runtime.render_terminal_text();
            let content_column = screen
                .lines()
                .find_map(|line| {
                    line.find("diff --git")
                        .map(|byte| line[..byte].chars().count())
                })
                .expect("native file header must be visible");
            let content = screen
                .lines()
                .skip(1)
                .take(expected.lines().count())
                .map(|line| {
                    line.chars()
                        .skip(content_column)
                        .collect::<String>()
                        .trim_end_matches('│')
                        .trim_end()
                        .to_string()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(content, expected.trim_end(), "{screen}");
            assert!(!screen.contains("### unstaged"));
            assert!(!screen.contains("[Worktree]"));
            assert_eq!(repo.run_git_capture(&["show", ":a.txt"]), before);
            assert_eq!(
                std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
                after
            );
            assert!(repo.run_git_capture(&["diff", "--cached"]).is_empty());
        }
    });
}
