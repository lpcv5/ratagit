use ratagit_core::{
    Action, AppContext, Command, CommitFileEntry, CommitFileStatus, CommitHashStatus,
    FileDiffTarget, FilesSnapshot, GitErrorKind, GitFailure, GitResult, PanelFocus, ResetChoice,
    UiAction, update,
};
use ratagit_testkit::{
    fixture_commit, fixture_conflict, fixture_dirty_repo, fixture_empty_repo, fixture_file,
    fixture_many_files, fixture_unicode_paths,
};
use ratagit_ui::{
    RenderContext, TerminalSize, batch_selected_row_style, buffer_contains_batch_selected_text,
    buffer_contains_selected_text, buffer_contains_text_with_style,
    buffer_to_text_with_selected_marker, details_content_lines_for_terminal_size,
    details_scroll_lines_for_terminal_size, focused_left_panel_content_lines_for_terminal_size,
    focused_panel_style, render, render_terminal_buffer, render_terminal_buffer_with_cursor,
    render_terminal_buffer_with_render_context, render_terminal_text,
    render_terminal_text_with_context,
};
use ratatui::style::{Color, Modifier, Style};

const MODAL_ACTIVE: Color = Color::Rgb(0x7a, 0xa2, 0xf7);
const MODAL_TEXT: Color = Color::Rgb(0xc0, 0xca, 0xf5);
const MODAL_DIM: Color = Color::Rgb(0x56, 0x5f, 0x89);
const MODAL_BORDER: Color = Color::Rgb(0x3b, 0x42, 0x61);
const MODAL_SURFACE: Color = Color::Rgb(0x24, 0x28, 0x3b);
const MODAL_DANGER: Color = Color::Rgb(0xf7, 0x76, 0x8e);
const MODAL_WARNING: Color = Color::Rgb(0xe0, 0xaf, 0x68);
const MODAL_SCRIM: Color = Color::Rgb(0x16, 0x1b, 0x2d);

fn render_snapshot(snapshot: ratagit_core::RepoSnapshot, size: TerminalSize) -> String {
    let mut state = AppContext::default();
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

fn target_paths(targets: &[FileDiffTarget]) -> Vec<String> {
    targets.iter().map(|target| target.path.clone()).collect()
}

fn mock_branch_details_log(branch: &str) -> String {
    format!(
        "\u{1b}[33m*\u{1b}[m \u{1b}[33mcommit abc1234\u{1b}[m\nAuthor: ratagit-tests <ratagit-tests@example.com>\n\n    init project on {branch}"
    )
}

fn mock_commit_details_diff(commit_id: &str) -> String {
    format!(
        "commit {commit_id}\nAuthor: ratagit-tests <ratagit-tests@example.com>\n\n    selected commit\n\ndiff --git a/commit.txt b/commit.txt\n@@ -1 +1 @@\n-old {commit_id}\n+new {commit_id}"
    )
}

fn mock_commit_files() -> Vec<CommitFileEntry> {
    vec![
        CommitFileEntry {
            path: "README.md".to_string(),
            old_path: None,
            status: CommitFileStatus::Modified,
        },
        CommitFileEntry {
            path: "src/lib.rs".to_string(),
            old_path: None,
            status: CommitFileStatus::Added,
        },
        CommitFileEntry {
            path: "src/new_name.rs".to_string(),
            old_path: Some("src/old_name.rs".to_string()),
            status: CommitFileStatus::Renamed,
        },
    ]
}

fn mock_commit_file_diff(target: &ratagit_core::CommitFileDiffTarget) -> String {
    target
        .paths
        .iter()
        .map(|path| {
            let old_path = path.old_path.as_deref().unwrap_or(&path.path);
            format!(
                "diff --git a/{old_path} b/{path}\n@@ -1 +1 @@\n-old {old_path}\n+new {path}",
                path = path.path
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn apply_refreshed_with_mock_details(state: &mut AppContext, snapshot: ratagit_core::RepoSnapshot) {
    let commands = update(state, Action::GitResult(GitResult::Refreshed(snapshot)));
    apply_mock_details_commands(state, commands);
}

fn apply_files_refreshed_with_mock_details(state: &mut AppContext, snapshot: FilesSnapshot) {
    let commands = update(
        state,
        Action::GitResult(GitResult::FilesRefreshed(snapshot)),
    );
    apply_mock_details_commands(state, commands);
}

fn buffer_contains_text_with_exact_fg(
    buffer: &ratagit_ui::TerminalBuffer,
    needle: &str,
    color: Color,
) -> bool {
    let width = buffer.area.width as usize;
    if width == 0 {
        return false;
    }
    buffer.content().chunks(width).any(|cells| {
        let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
        let Some(byte_start) = line.find(needle) else {
            return false;
        };
        let start = line[..byte_start].chars().count();
        cells
            .iter()
            .skip(start)
            .take(needle.len())
            .all(|cell| cell.fg == color)
    })
}

fn buffer_contains_text_with_exact_style(
    buffer: &ratagit_ui::TerminalBuffer,
    needle: &str,
    style: Style,
) -> bool {
    let width = buffer.area.width as usize;
    if width == 0 {
        return false;
    }
    buffer.content().chunks(width).any(|cells| {
        let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
        let Some(byte_start) = line.find(needle) else {
            return false;
        };
        let start = line[..byte_start].chars().count();
        cells
            .iter()
            .skip(start)
            .take(needle.chars().count())
            .all(|cell| {
                style.fg.is_none_or(|fg| cell.fg == fg)
                    && style.bg.is_none_or(|bg| cell.bg == bg)
                    && cell.modifier.contains(style.add_modifier)
            })
    })
}

fn loading_text_cells<'a>(
    buffer: &'a ratagit_ui::TerminalBuffer,
    needle: &str,
) -> Vec<&'a ratatui::buffer::Cell> {
    let width = buffer.area.width as usize;
    assert!(width > 0);
    let Some(cells) = buffer.content().chunks(width).find(|cells| {
        cells
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
            .contains(needle)
    }) else {
        panic!("loading text not found: {needle}");
    };
    let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
    let byte_start = line.find(needle).expect("needle should be present");
    let start = line[..byte_start].chars().count();
    cells
        .iter()
        .skip(start)
        .take(needle.chars().count())
        .collect()
}

fn apply_mock_details_commands(state: &mut AppContext, commands: Vec<Command>) {
    match commands.as_slice() {
        [] => {}
        [
            Command::RefreshFilesDetailsDiff {
                request_id,
                targets,
                truncated_from,
            },
        ] => {
            let paths = target_paths(targets);
            let follow_up = update(
                state,
                Action::GitResult(GitResult::FilesDetailsDiff {
                    request_id: *request_id,
                    targets: targets.clone(),
                    truncated_from: *truncated_from,
                    result: Ok(mock_files_details_diff(&paths)),
                }),
            );
            assert!(follow_up.is_empty());
        }
        [
            Command::RefreshBranchDetailsLog {
                request_id, branch, ..
            },
        ] => {
            let follow_up = update(
                state,
                Action::GitResult(GitResult::BranchDetailsLog {
                    request_id: *request_id,
                    branch: branch.clone(),
                    result: Ok(mock_branch_details_log(branch)),
                }),
            );
            assert!(follow_up.is_empty());
        }
        [
            Command::RefreshCommitDetailsDiff {
                request_id,
                commit_id,
            },
        ] => {
            let follow_up = update(
                state,
                Action::GitResult(GitResult::CommitDetailsDiff {
                    request_id: *request_id,
                    commit_id: commit_id.clone(),
                    result: Ok(mock_commit_details_diff(commit_id)),
                }),
            );
            assert!(follow_up.is_empty());
        }
        [Command::RefreshBranchCommits { branch }] => {
            let follow_up = update(
                state,
                Action::GitResult(GitResult::BranchCommits {
                    branch: branch.clone(),
                    result: Ok(fixture_dirty_repo().commits),
                }),
            );
            apply_mock_details_commands(state, follow_up);
        }
        [Command::RefreshBranchCommitFiles { branch, commit_id }] => {
            let follow_up = update(
                state,
                Action::GitResult(GitResult::BranchCommitFiles {
                    branch: branch.clone(),
                    commit_id: commit_id.clone(),
                    result: Ok(mock_commit_files()),
                }),
            );
            apply_mock_details_commands(state, follow_up);
        }
        [Command::RefreshCommitFiles { commit_id }] => {
            let follow_up = update(
                state,
                Action::GitResult(GitResult::CommitFiles {
                    commit_id: commit_id.clone(),
                    result: Ok(mock_commit_files()),
                }),
            );
            apply_mock_details_commands(state, follow_up);
        }
        [Command::RefreshCommitFileDiff { request_id, target }] => {
            let follow_up = update(
                state,
                Action::GitResult(GitResult::CommitFileDiff {
                    request_id: *request_id,
                    target: target.clone(),
                    result: Ok(mock_commit_file_diff(target)),
                }),
            );
            assert!(follow_up.is_empty());
        }
        _ => panic!("unexpected commands after refreshed snapshot: {commands:?}"),
    }
}

fn assert_no_cursor_marker(text: &str) {
    assert!(
        !text
            .lines()
            .any(|line| { line.trim_start().starts_with("> ") || line.contains("│>") })
    );
}

fn render_terminal_snapshot_with_cursor_marker(state: &AppContext, size: TerminalSize) -> String {
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
    assert!(text.contains("space stage/unstage"));
    assert!(!text.contains("keys(files):"));
    assert_no_cursor_marker(&text);
    assert!(!text.contains("tab/shift+tab"));
    assert!(!text.contains("1-6 focus panel"));
}

#[test]
fn bottom_keys_show_loading_indicator_before_shortcuts() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    state.work.refresh.refresh_pending = true;

    let screen = render_terminal_text_with_context(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
        RenderContext { spinner_frame: 3 },
    );

    assert!(screen.contains("- loading: refresh"));
    assert!(screen.contains("- loading: refresh   p  pull   P  push"));
}

#[test]
fn bottom_loading_indicator_sweeps_spotlight_across_text_without_background() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    state.work.refresh.refresh_pending = true;
    let size = TerminalSize {
        width: 100,
        height: 30,
    };

    let first_frame = render_terminal_buffer_with_render_context(
        &state,
        size,
        RenderContext { spinner_frame: 0 },
    );
    let next_frame = render_terminal_buffer_with_render_context(
        &state,
        size,
        RenderContext { spinner_frame: 1 },
    );

    let first_cells = loading_text_cells(&first_frame, "loading: refresh");
    assert_eq!(first_cells[0].fg, MODAL_WARNING);
    assert_eq!(first_cells[1].fg, MODAL_ACTIVE);
    assert_eq!(first_cells[2].fg, MODAL_DIM);
    assert!(first_cells.iter().all(|cell| cell.bg == Color::Reset));

    let next_cells = loading_text_cells(&next_frame, "loading: refresh");
    assert_eq!(next_cells[0].fg, MODAL_ACTIVE);
    assert_eq!(next_cells[1].fg, MODAL_WARNING);
    assert_eq!(next_cells[2].fg, MODAL_ACTIVE);
    assert_eq!(next_cells[3].fg, MODAL_DIM);
    assert!(next_cells.iter().all(|cell| cell.bg == Color::Reset));
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
    assert!(text.contains("M main.rs"));
    assert!(text.contains("M lib.rs"));
    assert!(text.contains("? README.md"));
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
    let mut state = AppContext::default();
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
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::StartSearch));
    update(&mut state, Action::Ui(UiAction::InputSearchChar('l')));
    update(&mut state, Action::Ui(UiAction::InputSearchChar('i')));

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
    assert!(!text.contains("space stage/unstage"));
}

#[test]
fn snapshots_files_shortcuts_include_reset_menu_key() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("D reset"));
}

#[test]
fn terminal_snapshot_refresh_pending_loading() {
    let mut state = AppContext::default();
    let commands = update(&mut state, Action::Ui(UiAction::RefreshAll));
    assert_eq!(commands, Command::refresh_all_commands());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_details_pending_loading() {
    let mut state = AppContext::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    assert!(matches!(
        commands.as_slice(),
        [Command::RefreshFilesDetailsDiff { .. }]
    ));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_details_scrolled_down() {
    let mut state = AppContext::default();
    let size = TerminalSize {
        width: 80,
        height: 14,
    };
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollDown {
            lines: details_scroll_lines_for_terminal_size(size),
            visible_lines: details_content_lines_for_terminal_size(size),
        }),
    );

    insta::assert_snapshot!(render_terminal_text(&state, size));
}

#[test]
fn terminal_snapshot_commits_details_diff() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
    );
    apply_mock_details_commands(&mut state, commands);

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_commit_files_subpanel() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
    apply_mock_details_commands(&mut state, commands);

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_commit_files_directory_diff() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_commit_files_loading() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
    assert!(matches!(
        commands.as_slice(),
        [Command::RefreshCommitFiles { .. }]
    ));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_commit_files_empty() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
    let [Command::RefreshCommitFiles { commit_id }] = commands.as_slice() else {
        panic!("expected commit files refresh");
    };
    let follow_up = update(
        &mut state,
        Action::GitResult(GitResult::CommitFiles {
            commit_id: commit_id.clone(),
            result: Ok(Vec::new()),
        }),
    );
    assert!(follow_up.is_empty());

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branch_commits_subview() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenBranchCommitsPanel));
    apply_mock_details_commands(&mut state, commands);

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branch_commit_files_subview() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenBranchCommitsPanel));
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::OpenBranchCommitFilesPanel));
    apply_mock_details_commands(&mut state, commands);

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_reset_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenResetMenu));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_reset_modal_nuke_description() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenResetMenu));
    state.ui.reset_menu.selected = ResetChoice::Nuke;

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_reset_hard_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenResetMenu));
    state.ui.reset_menu.selected = ResetChoice::Hard;
    update(&mut state, Action::Ui(UiAction::ConfirmResetMenu));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_reset_nuke_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenResetMenu));
    state.ui.reset_menu.selected = ResetChoice::Nuke;
    update(&mut state, Action::Ui(UiAction::ConfirmResetMenu));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_discard_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_files_discard_confirm_modal_fullscreen() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 160,
            height: 50,
        },
    ));
}

#[test]
fn terminal_snapshot_files_discard_confirm_multiselect_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::EnterFilesMultiSelect));
    update(&mut state, Action::Ui(UiAction::MoveDown));
    update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branches_create_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(&mut state, Action::Ui(UiAction::OpenBranchCreateInput));
    for ch in "feature/new".chars() {
        update(&mut state, Action::Ui(UiAction::BranchCreateInputChar(ch)));
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
fn terminal_snapshot_branches_delete_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(&mut state, Action::Ui(UiAction::OpenBranchDeleteMenu));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branches_remote_delete_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(&mut state, Action::Ui(UiAction::OpenBranchDeleteMenu));
    update(&mut state, Action::Ui(UiAction::MoveBranchDeleteMenuDown));
    update(&mut state, Action::Ui(UiAction::ConfirmBranchDeleteMenu));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branches_both_delete_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(&mut state, Action::Ui(UiAction::OpenBranchDeleteMenu));
    update(&mut state, Action::Ui(UiAction::MoveBranchDeleteMenuDown));
    update(&mut state, Action::Ui(UiAction::MoveBranchDeleteMenuDown));
    update(&mut state, Action::Ui(UiAction::ConfirmBranchDeleteMenu));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branches_force_delete_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(
        &mut state,
        Action::GitResult(GitResult::DeleteBranch {
            name: "feature/mvp".to_string(),
            mode: ratagit_core::BranchDeleteMode::Local,
            force: false,
            result: Err(GitFailure::new(
                GitErrorKind::UnmergedBranchDelete,
                "error: The branch 'feature/mvp' is not fully merged.",
            )),
        }),
    );

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_force_push_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(
        &mut state,
        Action::GitResult(GitResult::Push {
            force: false,
            result: Err(GitFailure::new(
                GitErrorKind::DivergentPush,
                "! [rejected] main -> main (fetch first)",
            )),
        }),
    );

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_stage_all_confirm_modal() {
    let mut state = AppContext::default();
    let mut snapshot = fixture_dirty_repo();
    for file in &mut snapshot.files {
        file.staged = false;
    }
    snapshot.status_summary = "staged: 0, unstaged: 2".to_string();
    apply_refreshed_with_mock_details(&mut state, snapshot);
    update(
        &mut state,
        Action::Ui(UiAction::CreateCommit {
            message: "feat: ship".to_string(),
        }),
    );

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branches_rebase_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(&mut state, Action::Ui(UiAction::OpenBranchRebaseMenu));
    update(&mut state, Action::Ui(UiAction::MoveBranchRebaseMenuDown));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_branches_auto_stash_confirm_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    apply_mock_details_commands(&mut state, commands);
    update(&mut state, Action::Ui(UiAction::CheckoutSelectedBranch));

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn snapshots_files_multi_select_marks_selected_rows() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::EnterFilesMultiSelect));

    let text = render(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    )
    .as_text();
    assert!(text.contains("✓  ? README.md"));
    assert_no_cursor_marker(&text);
}

#[test]
fn snapshots_files_list_scrolls_to_keep_selection_visible() {
    let mut state = AppContext::default();
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
    assert!(text.contains("   M file-16.txt"));
    assert!(text.contains("   M file-17.txt"));
    assert!(text.contains("   M file-20.txt"));
    assert!(text.contains("M file-23.txt"));
    assert_no_cursor_marker(&text);
    assert!(!text.contains("file-24.txt"));

    let screen = render_terminal_snapshot_with_cursor_marker(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(screen.contains(">│   M file-20.txt"));
}

#[test]
fn snapshots_files_list_reversing_up_does_not_jump_to_top_reserve() {
    let mut state = AppContext::default();
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    apply_refreshed_with_mock_details(&mut state, fixture_many_files());
    let visible_lines = focused_left_panel_content_lines_for_terminal_size(&state, size);
    for _ in 0..25 {
        update(
            &mut state,
            Action::Ui(UiAction::MoveDownInViewport { visible_lines }),
        );
    }
    for _ in 0..5 {
        update(
            &mut state,
            Action::Ui(UiAction::MoveUpInViewport { visible_lines }),
        );
    }

    let text = render(&state, size).as_text();
    assert!(text.contains("   M file-17.txt"));
    assert!(text.contains("   M file-20.txt"));
    assert!(text.contains("M file-24.txt"));
    assert_no_cursor_marker(&text);

    let screen = render_terminal_snapshot_with_cursor_marker(&state, size);
    assert!(screen.contains(">│   M file-20.txt"));
}

#[test]
fn snapshots_files_list_reversing_down_does_not_jump_to_bottom_reserve() {
    let mut state = AppContext::default();
    let size = TerminalSize {
        width: 100,
        height: 30,
    };
    apply_refreshed_with_mock_details(&mut state, fixture_many_files());
    let visible_lines = focused_left_panel_content_lines_for_terminal_size(&state, size);
    for _ in 0..25 {
        update(
            &mut state,
            Action::Ui(UiAction::MoveDownInViewport { visible_lines }),
        );
    }
    for _ in 0..5 {
        update(
            &mut state,
            Action::Ui(UiAction::MoveUpInViewport { visible_lines }),
        );
    }
    update(
        &mut state,
        Action::Ui(UiAction::MoveDownInViewport { visible_lines }),
    );

    let text = render(&state, size).as_text();
    assert!(text.contains("   M file-17.txt"));
    assert!(text.contains("   M file-21.txt"));
    assert!(text.contains("M file-24.txt"));

    update(
        &mut state,
        Action::Ui(UiAction::MoveDownInViewport { visible_lines }),
    );
    let text = render(&state, size).as_text();
    assert!(text.contains("   M file-18.txt"));
    assert!(text.contains("   M file-22.txt"));
    assert!(text.contains("M file-25.txt"));
}

#[test]
fn terminal_snapshot_empty_repo_80x24() {
    let mut state = AppContext::default();
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
    let mut state = AppContext::default();
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
    let mut state = AppContext::default();
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
fn terminal_snapshot_large_repo_fast_status_notice() {
    let mut snapshot = fixture_dirty_repo();
    snapshot.files = vec![
        fixture_file("src/lib.rs", false, false),
        fixture_file("src/main.rs", true, false),
    ];
    let mut state = AppContext::default();
    apply_files_refreshed_with_mock_details(
        &mut state,
        FilesSnapshot {
            status_summary: snapshot.status_summary,
            current_branch: snapshot.current_branch,
            detached_head: snapshot.detached_head,
            files: snapshot.files,
            index_entry_count: 100_000,
            large_repo_mode: true,
            status_truncated: true,
            status_scan_skipped: false,
            untracked_scan_skipped: true,
        },
    );

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_huge_repo_metadata_only_status_notice() {
    let snapshot = fixture_dirty_repo();
    let mut state = AppContext::default();
    apply_files_refreshed_with_mock_details(
        &mut state,
        FilesSnapshot {
            status_summary: "status scan skipped: 1000000 indexed files".to_string(),
            current_branch: snapshot.current_branch,
            detached_head: snapshot.detached_head,
            files: Vec::new(),
            index_entry_count: 1_000_000,
            large_repo_mode: true,
            status_truncated: false,
            status_scan_skipped: true,
            untracked_scan_skipped: true,
        },
    );

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    ));
}

#[test]
fn terminal_snapshot_large_directory_details_limit() {
    let mut snapshot = fixture_dirty_repo();
    snapshot.files = (0..101)
        .map(|index| fixture_file(&format!("src/file-{index:03}.txt"), false, false))
        .collect();
    let mut state = AppContext::default();
    apply_files_refreshed_with_mock_details(
        &mut state,
        FilesSnapshot {
            status_summary: snapshot.status_summary,
            current_branch: snapshot.current_branch,
            detached_head: snapshot.detached_head,
            files: snapshot.files,
            index_entry_count: 100_000,
            large_repo_mode: true,
            status_truncated: false,
            status_scan_skipped: false,
            untracked_scan_skipped: true,
        },
    );

    insta::assert_snapshot!(render_terminal_text(
        &state,
        TerminalSize {
            width: 120,
            height: 34,
        },
    ));
}

#[test]
fn terminal_snapshot_conflict_repo_120x40() {
    let mut state = AppContext::default();
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
    let mut state = AppContext::default();
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
    assert!(screen.contains("enter  files"));
    assert!(!screen.contains(" Keys "));
    assert!(!screen.contains("keys(commits):"));
    assert!(
        screen
            .lines()
            .last()
            .is_some_and(|line| line.starts_with("/ loading: details   p  pull   P  push"))
    );
}

#[test]
fn terminal_commits_panel_colors_hashes_and_authors() {
    let mut snapshot = fixture_dirty_repo();
    let mut main_commit = fixture_commit("aaa1111", "merged");
    main_commit.hash_status = CommitHashStatus::MergedToMain;
    main_commit.author_name = "Alice Baker".to_string();
    let mut pushed_commit = fixture_commit("bbb2222", "pushed");
    pushed_commit.hash_status = CommitHashStatus::Pushed;
    pushed_commit.author_name = "Bea Clark".to_string();
    let mut unpushed_commit = fixture_commit("ccc3333", "unpushed");
    unpushed_commit.hash_status = CommitHashStatus::Unpushed;
    unpushed_commit.author_name = "Alice Baker".to_string();
    snapshot.commits = vec![main_commit, pushed_commit, unpushed_commit];

    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, snapshot);
    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 40,
        },
    );

    assert!(buffer_contains_text_with_exact_fg(
        &buffer,
        "aaa1111",
        Color::Green
    ));
    assert!(buffer_contains_text_with_exact_fg(
        &buffer,
        "bbb2222",
        Color::Yellow
    ));
    assert!(buffer_contains_text_with_exact_fg(
        &buffer,
        "ccc3333",
        Color::Red
    ));
    assert!(buffer_contains_text_with_exact_fg(
        &buffer,
        "AB",
        Color::Magenta
    ));
    assert!(buffer_contains_text_with_exact_fg(
        &buffer,
        "BC",
        Color::White
    ));
}

#[test]
fn terminal_snapshot_files_search_updates_screen() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::StartSearch));
    update(&mut state, Action::Ui(UiAction::InputSearchChar('l')));
    update(&mut state, Action::Ui(UiAction::InputSearchChar('i')));

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(screen.contains("search: li"));
    assert!(screen.contains("   M lib.rs"));
    assert_no_cursor_marker(&screen);
}

#[test]
fn terminal_snapshot_error_is_visible_in_log_panel() {
    let mut state = AppContext::default();
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
    let mut state = AppContext::default();
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
    let mut state = AppContext::default();
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

    assert_eq!(cursor.expect("editor cursor should render").y, 13);
}

#[test]
fn terminal_commit_editor_cursor_wraps_long_body_line() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    update(&mut state, Action::Ui(UiAction::EditorNextField));
    for ch in "x".repeat(75).chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    let cursor = cursor.expect("editor cursor should render");

    assert_eq!((cursor.x, cursor.y), (44, 13));
}

#[test]
fn terminal_commit_editor_cursor_follows_subject_field() {
    let mut state = AppContext::default();
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

    assert_eq!(cursor.expect("editor cursor should render").y, 9);
}

#[test]
fn terminal_commit_editor_cursor_wraps_long_subject() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    for ch in "x".repeat(75).chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    let cursor = cursor.expect("editor cursor should render");

    assert_eq!((cursor.x, cursor.y), (44, 10));
}

#[test]
fn terminal_snapshot_files_stash_editor_modal() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::EnterFilesMultiSelect));
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
    let mut state = AppContext::default();
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

    assert_eq!(cursor.expect("editor cursor should render").y, 9);
}

#[test]
fn terminal_stash_editor_cursor_wraps_long_title() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::OpenStashEditor));
    for ch in "x".repeat(75).chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let (_, cursor) = render_terminal_buffer_with_cursor(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    let cursor = cursor.expect("editor cursor should render");

    assert_eq!((cursor.x, cursor.y), (44, 10));
}

#[test]
fn terminal_snapshot_untracked_directory_marker_renders_as_directory_node() {
    let mut state = AppContext::default();
    let mut snapshot = fixture_empty_repo();
    snapshot.files = vec![fixture_file("libs/ratagit-git/tests/", false, true)];
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
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_selected_text(&buffer, "? README.md"));
    assert!(!buffer_contains_selected_text(&buffer, " main"));
}

#[test]
fn terminal_buffer_highlights_marked_files_with_batch_style() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::EnterFilesMultiSelect));
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
        "✓  ? README.md",
        batch_selected_row_style()
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "✓   src/",
        batch_selected_row_style()
    ));
}

#[test]
fn terminal_buffer_highlights_marked_branches_with_batch_style() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    update(&mut state, Action::Ui(UiAction::EnterBranchesMultiSelect));
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
        " main",
        batch_selected_row_style()
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "  feature/mvp",
        batch_selected_row_style()
    ));
}

#[test]
fn terminal_buffer_highlights_marked_commit_files_with_batch_style() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(&mut state, Action::Ui(UiAction::FocusNext));
    update(&mut state, Action::Ui(UiAction::FocusNext));
    let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
    apply_mock_details_commands(&mut state, commands);
    update(
        &mut state,
        Action::Ui(UiAction::EnterCommitFilesMultiSelect),
    );
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
        "✓  M README.md",
        batch_selected_row_style()
    ));
    assert!(buffer_contains_batch_selected_text(&buffer, " src/"));
}

#[test]
fn terminal_buffer_moves_selection_highlight_with_focus() {
    let mut state = AppContext::default();
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

    assert!(!buffer_contains_selected_text(&buffer, "? README.md"));
    assert!(buffer_contains_selected_text(&buffer, " main"));
}

#[test]
fn terminal_buffer_styles_focused_panel_title() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_exact_style(
        &buffer,
        "󰈙 Files",
        focused_panel_style()
    ));
    assert!(!buffer_contains_text_with_exact_style(
        &buffer,
        " Branches",
        focused_panel_style()
    ));
}

#[test]
fn terminal_buffer_uses_rounded_shared_panel_borders() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    let first_line = screen.lines().next().expect("screen should have lines");

    assert!(first_line.starts_with('╭'));
    assert!(!screen.contains('┌'));
    assert_eq!(first_line.matches('╭').count(), 1);
}

#[test]
fn terminal_buffer_focused_shared_panel_uses_complete_border() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );

    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(screen.contains("╭ 2   Branches"));
}

#[test]
fn terminal_buffer_styles_panel_number_as_badge() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_exact_style(
        &buffer,
        " 1 ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
    assert!(buffer_contains_text_with_exact_style(
        &buffer,
        " 2 ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    ));
}

#[test]
fn terminal_buffer_styles_shortcut_keys_as_badges() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    let screen = render_terminal_text(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(!screen.contains("keys(files):"));
    assert!(
        screen
            .lines()
            .last()
            .is_some_and(|line| !line.contains('|'))
    );
    assert!(buffer_contains_text_with_exact_style(
        &buffer,
        " space ",
        Style::default()
            .fg(Color::Black)
            .bg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ));
}

#[test]
fn terminal_buffer_styles_files_details_diff_from_ansi() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    state.repo.details.files_diff = [
        "### unstaged",
        "\u{1b}[1mdiff --git a/old.bin b/new.bin\u{1b}[m",
        "similarity index 88%",
        "rename from old.bin",
        "rename to new.bin",
        "old mode 100644",
        "new mode 100755",
        "Binary files a/old.bin and b/new.bin differ",
        "\\ No newline at end of file",
        "\u{1b}[36m@@ -1 +1 @@\u{1b}[m",
        "\u{1b}[31m-old README.md\u{1b}[m",
        "\u{1b}[32m+new README.md\u{1b}[m",
    ]
    .join("\n");

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_style(
        &buffer,
        "diff --git a/old.bin b/new.bin",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "@@ -1 +1 @@",
        Style::default().fg(Color::Cyan),
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

#[test]
fn terminal_buffer_styles_branch_details_log_from_ansi() {
    let mut state = AppContext::default();
    apply_refreshed_with_mock_details(&mut state, fixture_dirty_repo());
    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );
    apply_mock_details_commands(&mut state, commands);

    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );

    assert!(buffer_contains_text_with_style(
        &buffer,
        "*",
        Style::default().fg(Color::Yellow),
    ));
    assert!(buffer_contains_text_with_style(
        &buffer,
        "commit abc1234",
        Style::default().fg(Color::Yellow),
    ));
}

#[test]
fn terminal_buffer_styles_modal_titles_by_tone() {
    let info_style = Style::default()
        .fg(MODAL_ACTIVE)
        .add_modifier(Modifier::BOLD);
    let warning_style = Style::default()
        .fg(MODAL_WARNING)
        .add_modifier(Modifier::BOLD);
    let danger_style = Style::default()
        .fg(MODAL_DANGER)
        .add_modifier(Modifier::BOLD);
    let border_style = Style::default().fg(MODAL_BORDER);
    let selected_choice_style = Style::default()
        .fg(MODAL_TEXT)
        .bg(MODAL_SURFACE)
        .add_modifier(Modifier::BOLD);
    let scrim_style = Style::default().fg(MODAL_DIM).bg(MODAL_SCRIM);

    let mut commit_state = AppContext::default();
    apply_refreshed_with_mock_details(&mut commit_state, fixture_dirty_repo());
    update(&mut commit_state, Action::Ui(UiAction::OpenCommitEditor));
    let commit_buffer = render_terminal_buffer(
        &commit_state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(buffer_contains_text_with_style(
        &commit_buffer,
        "Commit Message",
        info_style
    ));
    assert!(buffer_contains_text_with_exact_style(
        &commit_buffer,
        "README.md",
        scrim_style
    ));
    assert!(buffer_contains_text_with_exact_style(
        &commit_buffer,
        "Subject",
        info_style
    ));
    assert!(buffer_contains_text_with_exact_style(
        &commit_buffer,
        "Body",
        border_style
    ));
    let commit_screen = render_terminal_text(
        &commit_state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(commit_screen.contains("╭ ✎ Commit Message"));
    assert!(commit_screen.contains("────────────────"));

    let mut reset_state = AppContext::default();
    apply_refreshed_with_mock_details(&mut reset_state, fixture_dirty_repo());
    update(&mut reset_state, Action::Ui(UiAction::OpenResetMenu));
    let reset_buffer = render_terminal_buffer(
        &reset_state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(buffer_contains_text_with_style(
        &reset_buffer,
        "Reset",
        warning_style
    ));
    assert!(buffer_contains_text_with_exact_style(
        &reset_buffer,
        "mixed",
        selected_choice_style
    ));

    let mut discard_state = AppContext::default();
    apply_refreshed_with_mock_details(&mut discard_state, fixture_dirty_repo());
    update(&mut discard_state, Action::Ui(UiAction::OpenDiscardConfirm));
    let discard_buffer = render_terminal_buffer(
        &discard_state,
        TerminalSize {
            width: 100,
            height: 30,
        },
    );
    assert!(buffer_contains_text_with_style(
        &discard_buffer,
        "Confirm",
        danger_style
    ));
    assert!(buffer_contains_text_with_style(
        &discard_buffer,
        "Discard selected file changes?",
        danger_style
    ));
}
