use super::*;

#[test]
fn real_git_selected_stash_payload_excludes_unselected_index_changes() {
    if !git_available() {
        return;
    }
    let repo = seeded_repo("harness-stash-selected-scope");
    write(repo.path().join("a.txt"), "unselected staged\n").unwrap();
    repo.run_git(&["add", "--", "a.txt"]);
    write(repo.path().join("b.txt"), "selected tracked\n").unwrap();
    write(repo.path().join("new.txt"), "selected untracked\n").unwrap();
    let mut runtime = runtime_for(&repo);
    with_case_artifacts(&repo, &mut runtime, |runtime| {
        runtime.dispatch_all(&[
            UiAction::RefreshAll,
            UiAction::MoveDown,
            UiAction::EnterFilesMultiSelect,
            UiAction::MoveDown,
            UiAction::OpenStashEditor,
        ]);
        assert!(
            runtime
                .render_terminal_text()
                .contains("selected files (2)")
        );
        let commands = ratagit_core::update(
            &mut runtime.state().clone(),
            ratagit_core::Action::Ui(UiAction::EditorConfirm),
        );
        assert_eq!(
            commands,
            vec![ratagit_core::Command::StashFiles {
                message: String::new(),
                paths: vec!["b.txt".to_string(), "new.txt".to_string()],
            }]
        );
        runtime.dispatch_ui(UiAction::EditorConfirm);
        let names = repo.run_git_capture(&[
            "stash",
            "show",
            "--include-untracked",
            "--name-only",
            "stash@{0}",
        ]);
        assert_eq!(names.lines().collect::<Vec<_>>(), ["b.txt", "new.txt"]);
        assert_eq!(
            repo.run_git_capture(&["show", ":a.txt"]),
            "unselected staged\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "unselected staged\n"
        );
        assert_eq!(
            runtime.state().last_operation.as_deref(),
            Some("stash_files")
        );
        assert!(runtime.render_terminal_text().contains("Stashed"));

        // Discarding the remaining unselected change must not make pop revive it.
        repo.run_git(&["restore", "--staged", "--worktree", "--", "a.txt"]);
        runtime.dispatch_ui(UiAction::RefreshAll);
        runtime.dispatch_ui(UiAction::FocusPanel {
            panel: PanelFocus::Stash,
        });
        let preview = runtime.render_terminal_text();
        assert!(preview.contains("diff --git a/new.txt b/new.txt"));
        assert!(!preview.contains("diff --git a/a.txt b/a.txt"));
        runtime.dispatch_ui(UiAction::StashPopSelected);
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt")).unwrap(),
            "a1\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("b.txt")).unwrap(),
            "selected tracked\n"
        );
        assert_eq!(
            std::fs::read_to_string(repo.path().join("new.txt")).unwrap(),
            "selected untracked\n"
        );
        assert!(repo.run_git_capture(&["stash", "list"]).is_empty());
    });
}
