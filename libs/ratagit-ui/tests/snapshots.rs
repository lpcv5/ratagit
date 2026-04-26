use ratagit_core::{Action, AppState, GitResult, PanelFocus, UiAction, update};
use ratagit_testkit::{
    fixture_conflict, fixture_dirty_repo, fixture_empty_repo, fixture_many_files,
    fixture_unicode_paths,
};
use ratagit_ui::{TerminalSize, render, render_terminal_text};

fn render_snapshot(snapshot: ratagit_core::RepoSnapshot, size: TerminalSize) -> String {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(snapshot)),
    );
    assert!(commands.is_empty());
    render(&state, size).as_text()
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
    assert!(text.contains("[Files]"));
    assert!(text.contains("[Details]"));
    assert!(text.contains("keys(files):"));
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
    assert!(text.contains("[Files]"));
    assert!(text.contains("[Commits]"));
    assert!(text.contains("[Branches]"));
    assert!(text.contains("[Stash]"));
    assert!(text.contains("[Details]"));
    assert!(text.contains("[Log]"));
    assert!(text.contains("[-] src/"));
    assert!(text.contains("[S] main.rs"));
    assert!(text.contains("[ ] lib.rs"));
    assert!(text.contains("[?] README.md"));
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
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());
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
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());
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
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());
    update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("> * [?] README.md"));
}

#[test]
fn snapshots_files_list_scrolls_to_keep_selection_visible() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_many_files())),
    );
    assert!(commands.is_empty());
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
    assert!(text.contains("    [S] file-16.txt"));
    assert!(text.contains("    [ ] file-17.txt"));
    assert!(text.contains(">   [S] file-20.txt"));
    assert!(text.contains("[ ] file-23.txt"));
    assert!(!text.contains("file-24.txt"));
    assert!(!text.contains("file-00.txt"));
}

#[test]
fn snapshots_files_list_scrolls_up_with_top_reserve() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_many_files())),
    );
    assert!(commands.is_empty());
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
    assert!(!text.contains("file-16.txt"));
    assert!(text.contains("    [ ] file-17.txt"));
    assert!(text.contains(">   [S] file-20.txt"));
    assert!(text.contains("[S] file-24.txt"));
}

#[test]
fn terminal_snapshot_empty_repo_80x24() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_empty_repo())),
    );
    assert!(commands.is_empty());

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
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_conflict_repo_120x40() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_conflict())),
    );
    assert!(commands.is_empty());

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
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());
    update(&mut state, Action::Ui(UiAction::FocusNext));
    update(&mut state, Action::Ui(UiAction::FocusNext));

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(screen.contains("Commits *"));
    assert!(screen.contains("keys(commits): c commit"));
}

#[test]
fn terminal_snapshot_files_search_updates_screen() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());
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
    assert!(screen.contains("!  [ ] lib.rs"));
}

#[test]
fn terminal_snapshot_error_is_visible_in_log_panel() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(commands.is_empty());
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
