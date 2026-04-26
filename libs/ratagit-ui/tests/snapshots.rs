use ratagit_core::{Action, AppState, Command, FileEntry, GitResult, PanelFocus, UiAction, update};
use ratagit_testkit::{
    fixture_conflict, fixture_dirty_repo, fixture_empty_repo, fixture_many_files,
    fixture_unicode_paths,
};
use ratagit_ui::{
    TerminalSize, batch_selected_row_style, buffer_contains_selected_text,
    buffer_contains_text_with_style, buffer_to_text_with_selected_marker, focused_panel_style,
    render, render_terminal_buffer, render_terminal_buffer_with_cursor, render_terminal_text,
};
use ratatui::style::{Color, Style};

fn render_snapshot(snapshot: ratagit_core::RepoSnapshot, size: TerminalSize) -> String {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, snapshot);
    render(&state, size).as_text()
}

fn mock_files_details_diff(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let mut blocks = Vec::new();
    for path in paths {
        blocks.push(format!(
            "diff --git a/{0} b/{0}\n@@ -1 +1 @@\n-old {0}\n+new {0}",
            path
        ));
    }
    format!("### unstaged\n{}", blocks.join("\n"))
}

fn apply_refreshed_with_mock_details(state: &mut AppState, snapshot: ratagit_core::RepoSnapshot) {
    let commands = update(state, Action::GitResult(GitResult::Refreshed(snapshot)));
    if let [Command::RefreshFilesDetailsDiff { paths }] = commands.as_slice() {
        let follow_up = update(
            state,
            Action::GitResult(GitResult::FilesDetailsDiff {
                paths: paths.clone(),
                result: Ok(mock_files_details_diff(paths)),
            }),
        );
        assert!(follow_up.is_empty());
    } else {
        panic!("unexpected commands after refreshed snapshot: {commands:?}");
    }
}

fn assert_no_cursor_marker(text: &str) {
    assert!(
        !text
            .lines()
            .any(|line| { line.trim_start().starts_with("> ") || line.contains("│>") })
    );
}

fn render_terminal_snapshot_with_cursor_marker(state: &AppState, size: TerminalSize) -> String {
    buffer_to_text_with_selected_marker(&render_terminal_buffer(state, size))
}

#[test]
fn snapshots_empty_repo_80x24() {
    let text = render_snapshot(
        fixture_empty_repo(),
        TerminalSize {
            width: 80,
            height: 24,
        },
    );
    let first_lines = text.lines().take(2).collect::<Vec<_>>().join("\n");
    assert!(!first_lines.contains("branch=main"));
    assert!(!first_lines.contains("focus=Files"));
    assert!(!first_lines.contains("summary=staged: 0, unstaged: 0"));
    assert!(text.contains("[1] 󰈙 Files"));
    assert!(text.contains("[5]  Details"));
    assert!(!text.contains("<empty>"));
    assert!(!text.contains("<none>"));
    assert!(text.contains("keys(files):"));
    assert_no_cursor_marker(&text);
    assert!(!text.contains("tab/shift+tab"));
    assert!(!text.contains("1-6 focus panel"));
}

#[test]
fn snapshots_dirty_repo_100x30() {
    let text = render_snapshot(
        fixture_dirty_repo(),
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(text.contains("[1] 󰈙 Files"));
    assert!(text.contains("[3]  Commits"));
    assert!(text.contains("[2]  Branches"));
    assert!(text.contains("[4]  Stash"));
    assert!(text.contains("[5]  Details"));
    assert!(text.contains("[6] 󰌱 Log"));
    assert!(text.contains(" src/"));
    assert!(text.contains(" main.rs"));
    assert!(text.contains(" lib.rs"));
    assert!(text.contains(" README.md"));
    assert_no_cursor_marker(&text);
}

#[test]
fn snapshots_conflict_120x40() {
    let text = render_snapshot(
        fixture_conflict(),
        TerminalSize {
            width: 120,
            height: 40,
        },
    );
    assert!(text.contains("both modified"));
    assert!(text.contains("conflict"));
}

#[test]
fn snapshots_unicode_paths_are_stable() {
    let text = render_snapshot(
        fixture_unicode_paths(),
        TerminalSize {
            width: 80,
            height: 24,
        },
    );
    assert!(text.contains("你好"));
    assert!(text.contains("emoji-"));
}

#[test]
fn snapshots_shortcuts_follow_current_focus() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Log,
        }),
    );
    assert!(commands.is_empty());

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(!text.contains("focus=Log"));
    assert!(!text.contains("keys(log):"));
    assert!(!text.contains("tab/shift+tab"));
}

#[test]
fn snapshots_files_search_input_replaces_shortcut_bar() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::StartFileSearch));
    update(&mut state, Action::Ui(UiAction::InputFileSearchChar('l')));
    update(&mut state, Action::Ui(UiAction::InputFileSearchChar('i')));

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("search: li"));
    assert!(!text.contains("keys(files):"));
}

#[test]
fn snapshots_files_multi_select_marks_selected_rows() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("✓   README.md"));
    assert_no_cursor_marker(&text);
}

#[test]
fn snapshots_files_list_scrolls_to_keep_selection_visible() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_many_files());
    for _ in 0..20 {
        update(&mut state, Action::Ui(UiAction::MoveDown));
    }

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("    file-16.txt"));
    assert!(text.contains("    file-17.txt"));
    assert!(text.contains("    file-20.txt"));
    assert!(text.contains(" file-23.txt"));
    assert_no_cursor_marker(&text);
    assert!(!text.contains("file-24.txt"));

    let screen = render_terminal_snapshot_with_cursor_marker(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(screen.contains(">│    file-20.txt"));
}

#[test]
fn snapshots_files_list_reversing_up_does_not_jump_to_top_reserve() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_many_files());
    for _ in 0..25 {
        update(&mut state, Action::Ui(UiAction::MoveDown));
    }
    for _ in 0..5 {
        update(&mut state, Action::Ui(UiAction::MoveUp));
    }

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("    file-17.txt"));
    assert!(text.contains("    file-20.txt"));
    assert!(text.contains(" file-24.txt"));
    assert_no_cursor_marker(&text);

    let screen = render_terminal_snapshot_with_cursor_marker(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(screen.contains(">│    file-20.txt"));
}

#[test]
fn snapshots_files_list_reversing_down_does_not_jump_to_bottom_reserve() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_many_files());
    for _ in 0..25 {
        update(&mut state, Action::Ui(UiAction::MoveDown));
    }
    for _ in 0..5 {
        update(&mut state, Action::Ui(UiAction::MoveUp));
    }
    update(&mut state, Action::Ui(UiAction::MoveDown));

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("    file-17.txt"));
    assert!(text.contains("    file-21.txt"));
    assert!(text.contains(" file-24.txt"));

    update(&mut state, Action::Ui(UiAction::MoveDown));
    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("    file-18.txt"));
    assert!(text.contains("    file-22.txt"));
    assert!(text.contains(" file-25.txt"));
}

#[test]
fn terminal_snapshot_empty_repo_80x24() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_empty_repo());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 80,
            height: 24,
        },
    ));
}

#[test]
fn terminal_snapshot_dirty_repo_100x30() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_many_files_focus_expands_left_panel() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_many_files());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 24,
        },
    ));
}

#[test]
fn terminal_snapshot_conflict_repo_120x40() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_conflict());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 120,
            height: 40,
        },
    ));
}

#[test]
fn terminal_snapshot_focus_and_keys_follow_actions() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::FocusNext));
    update(&mut state, Action::Ui(UiAction::FocusNext));

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(screen.contains(" Commits"));
    assert!(!screen.contains("Commits *"));
    assert!(screen.contains("keys(commits): c commit"));
    assert!(!screen.contains(" Keys "));
    assert!(
        screen
            .lines()
            .last()
            .is_some_and(|line| line.starts_with("keys(commits): c commit"))
    );
}

#[test]
fn terminal_snapshot_files_search_updates_screen() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::StartFileSearch));
    update(&mut state, Action::Ui(UiAction::InputFileSearchChar('l')));
    update(&mut state, Action::Ui(UiAction::InputFileSearchChar('i')));

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(screen.contains("search: li"));
    assert!(screen.contains("    lib.rs"));
    assert_no_cursor_marker(&screen);
}

#[test]
fn terminal_snapshot_error_is_visible_in_log_panel() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(
        &mut state,
        Action::GitResult(GitResult::CreateCommit {
            message: String::new(),
            result: Err("nothing staged".to_string()),
        }),
    );

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(screen.contains("error=Failed to create commit"));
}

#[test]
fn terminal_snapshot_files_commit_editor_modal() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    for ch in "feat: add modal".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorNextField));
    for ch in "body line 1".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorInsertNewline));
    for ch in "body line 2".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_commit_editor_cursor_follows_active_body_field() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    for ch in "feat".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorNextField));
    for ch in "line 1".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorInsertNewline));
    for ch in "line 2".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert_eq!(cursor.expect("editor cursor should render").y, 14);
}

#[test]
fn terminal_commit_editor_cursor_follows_subject_field() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    for ch in "feat".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert_eq!(cursor.expect("editor cursor should render").y, 10);
}

#[test]
fn terminal_snapshot_files_stash_editor_modal() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));
    update(&mut state, Action::Ui(UiAction::MoveDown));
    update(&mut state, Action::Ui(UiAction::OpenStashEditor));
    for ch in "pick files".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_stash_editor_cursor_follows_title() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenStashEditor));
    for ch in "pick".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert_eq!(cursor.expect("editor cursor should render").y, 12);
}

#[test]
fn terminal_snapshot_untracked_directory_marker_renders_as_directory_node() {
    let mut state = AppState::default();
    let mut snapshot = fixture_empty_repo();
    snapshot.files = vec![FileEntry {
        path: "libs/ratagit-git/tests/".to_string(),
        staged: false,
        untracked: true,
    }];
    snapshot.status_summary = "staged: 0, unstaged: 1".to_string();
    apply_refreshed_with_mock_details(&mut state, snapshot);

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_buffer_highlights_selected_row_only_in_focused_panel() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_selected_text(&buffer, " README.md"));
    assert!(!buffer_contains_selected_text(&buffer, " main"));
}

#[test]
fn terminal_buffer_highlights_marked_files_with_batch_style() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));
    update(&mut state, Action::Ui(UiAction::MoveDown));

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_style(
        &buffer,
        "✓   README.md",
        batch_selected_row_style()
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "✓   src/",
        batch_selected_row_style()
    ));
}

#[test]
fn terminal_buffer_moves_selection_highlight_with_focus() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(!buffer_contains_selected_text(&buffer, " README.md"));
    assert!(buffer_contains_selected_text(&buffer, " main"));
}

#[test]
fn terminal_buffer_styles_focused_panel_title() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_style(
        &buffer,
        "󰈙 Files",
        focused_panel_style()
    ));
    assert!(!buffer_contains_text_with_style(
        &buffer,
        " Branches",
        focused_panel_style()
    ));
}

#[test]
fn terminal_buffer_styles_files_details_diff_rows_by_semantics() {
    let mut state = AppState::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_style(
        &buffer,
        "diff --git a/README.md b/README.md",
        Style::default().fg(Color::Cyan),
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "@@ -1 +1 @@",
        Style::default().fg(Color::Magenta),
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "-old README.md",
        Style::default().fg(Color::Red),
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "+new README.md",
        Style::default().fg(Color::Green),
    ));
}
