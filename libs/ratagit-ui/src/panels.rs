#[path = "panel_details.rs"]
mod panel_details;
#[path = "panel_format.rs"]
mod panel_format;
#[path = "panel_left.rs"]
mod panel_left;
#[path = "panel_scroll.rs"]
mod panel_scroll;
#[path = "panel_shortcuts.rs"]
mod panel_shortcuts;
#[path = "panel_types.rs"]
mod panel_types;

#[cfg(test)]
use crate::theme::{RowRole, row_style};
pub(crate) use panel_details::{render_details_lines, render_log_lines};
pub use panel_format::{
    format_branch_entry, format_commit_entry, format_file_tree_row, format_stash_entry,
};
pub(crate) use panel_left::{
    left_panel_content_len, panel_title, panel_title_label, render_branches_lines,
    render_commits_lines, render_files_lines, render_stash_lines,
};
#[cfg(test)]
use panel_scroll::scroll_window_start;
pub(crate) use panel_shortcuts::{ShortcutLine, shortcut_line_for_state, shortcuts_for_state};
pub(crate) use panel_types::PanelLine;
#[cfg(test)]
use ratagit_core::AppContext;
#[cfg(test)]
mod tests {
    use ratagit_core::{
        Action, COMMITS_PAGE_SIZE, Command, FileDiffTarget, GitResult, PanelFocus, UiAction, update,
    };
    use ratagit_testkit::{fixture_branch, fixture_commit, fixture_dirty_repo, fixture_empty_repo};

    use super::*;

    fn mock_diff_for_paths(paths: &[String]) -> String {
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

    fn state_with_dirty_repo() -> AppContext {
        let mut state = AppContext::default();
        let commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
        );
        if let [
            Command::RefreshFilesDetailsDiff {
                targets,
                truncated_from,
            },
        ] = commands.as_slice()
        {
            let paths = target_paths(targets);
            let follow_up = update(
                &mut state,
                Action::GitResult(GitResult::FilesDetailsDiff {
                    targets: targets.clone(),
                    truncated_from: *truncated_from,
                    result: Ok(mock_diff_for_paths(&paths)),
                }),
            );
            assert!(follow_up.is_empty());
        } else {
            panic!("unexpected commands after refresh: {commands:?}");
        }
        state
    }

    fn commit_scroll_state(count: usize) -> AppContext {
        let mut state = state_with_dirty_repo();
        state.ui.focus = PanelFocus::Commits;
        state.repo.commits.items = (0..count)
            .map(|index| fixture_commit(&format!("{index:07x}"), &format!("commit {index}")))
            .collect();
        state
    }

    fn branch_scroll_state(count: usize) -> AppContext {
        let mut state = state_with_dirty_repo();
        state.ui.focus = PanelFocus::Branches;
        state.repo.branches.items = (0..count)
            .map(|index| fixture_branch(&format!("branch-{index}"), index == 0))
            .collect();
        state
    }

    fn commit_page(start: usize, count: usize) -> Vec<ratagit_core::CommitEntry> {
        (start..start + count)
            .map(|index| fixture_commit(&format!("{index:07x}"), &format!("commit {index}")))
            .collect()
    }

    fn has_search_span(line: &PanelLine, text: &str) -> bool {
        line.spans.as_ref().is_some_and(|spans| {
            spans
                .iter()
                .any(|span| span.text == text && span.style == row_style(RowRole::SearchMatch))
        })
    }

    #[test]
    fn files_panel_projects_tree_rows_and_selection() {
        let mut state = state_with_dirty_repo();
        state.ui.files.selected = 1;

        let lines = render_files_lines(&state, 4);

        assert_eq!(lines[0].text, "    README.md");
        assert_eq!(lines[1].text, "    src/");
        assert_eq!(lines[2].text, "      lib.rs");
        assert_eq!(lines[3].text, "      main.rs");
        assert!(lines[1].selected);
        assert!(!lines.iter().any(|line| line.text.contains('>')));
    }

    #[test]
    fn files_panel_projects_collapsed_directory_and_multi_select() {
        let mut state = state_with_dirty_repo();
        state.ui.files.selected = 1;
        update(&mut state, Action::Ui(UiAction::ToggleSelectedDirectory));
        update(&mut state, Action::Ui(UiAction::EnterFilesMultiSelect));

        let lines = render_files_lines(&state, 2);

        assert_eq!(lines[0].text, "    README.md");
        assert_eq!(lines[1].text, "✓   src/");
        assert_eq!(lines[1].role, RowRole::BatchSelected);
        assert!(lines[1].selected);
    }

    #[test]
    fn files_panel_marks_search_matches() {
        let mut state = state_with_dirty_repo();
        update(&mut state, Action::Ui(UiAction::StartSearch));
        update(&mut state, Action::Ui(UiAction::InputSearchChar('l')));
        update(&mut state, Action::Ui(UiAction::InputSearchChar('i')));

        let lines = render_files_lines(&state, 4);

        let line = lines
            .iter()
            .find(|line| line.text.contains("    lib.rs"))
            .expect("lib.rs row should be marked as a search match");
        assert_eq!(line.role, RowRole::Normal);
        assert!(has_search_span(line, "li"));
    }

    #[test]
    fn left_list_panels_mark_search_matches() {
        let mut state = state_with_dirty_repo();
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            }),
        );
        update(&mut state, Action::Ui(UiAction::StartSearch));
        for ch in "feature".chars() {
            update(&mut state, Action::Ui(UiAction::InputSearchChar(ch)));
        }
        let lines = render_branches_lines(&state, 3);
        assert_eq!(lines[1].role, RowRole::Normal);
        assert!(has_search_span(&lines[1], "feature"));

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            }),
        );
        update(&mut state, Action::Ui(UiAction::StartSearch));
        for ch in "wire".chars() {
            update(&mut state, Action::Ui(UiAction::InputSearchChar(ch)));
        }
        let lines = render_commits_lines(&state, 3);
        assert_eq!(lines[1].role, RowRole::Normal);
        assert!(has_search_span(&lines[1], "wire"));

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Stash,
            }),
        );
        update(&mut state, Action::Ui(UiAction::StartSearch));
        for ch in "wip".chars() {
            update(&mut state, Action::Ui(UiAction::InputSearchChar(ch)));
        }
        let lines = render_stash_lines(&state, 1);
        assert_eq!(lines[0].role, RowRole::Normal);
        assert!(has_search_span(&lines[0], "WIP"));
    }

    #[test]
    fn commit_files_panel_marks_search_matches() {
        let mut state = state_with_dirty_repo();
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            }),
        );
        let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
        let [Command::RefreshCommitFiles { commit_id }] = commands.as_slice() else {
            panic!("expected commit files refresh");
        };
        update(
            &mut state,
            Action::GitResult(GitResult::CommitFiles {
                commit_id: commit_id.clone(),
                result: Ok(vec![
                    ratagit_core::CommitFileEntry {
                        path: "README.md".to_string(),
                        old_path: None,
                        status: ratagit_core::CommitFileStatus::Modified,
                    },
                    ratagit_core::CommitFileEntry {
                        path: "src/lib.rs".to_string(),
                        old_path: None,
                        status: ratagit_core::CommitFileStatus::Added,
                    },
                ]),
            }),
        );
        update(&mut state, Action::Ui(UiAction::StartSearch));
        for ch in "lib".chars() {
            update(&mut state, Action::Ui(UiAction::InputSearchChar(ch)));
        }

        let lines = render_commits_lines(&state, 4);

        let line = lines
            .iter()
            .find(|line| line.text.contains("   A lib.rs"))
            .expect("lib.rs row should be marked as a search match");
        assert_eq!(line.role, RowRole::Normal);
        assert!(line.spans.as_ref().is_some_and(|spans| {
            spans
                .iter()
                .any(|span| span.text == "A" && span.style == row_style(RowRole::DiffAdd))
        }));
        assert!(has_search_span(line, "lib"));
    }

    #[test]
    fn commit_file_tree_colors_status_marker_not_file_name() {
        let mut state = state_with_dirty_repo();
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            }),
        );
        let commands = update(&mut state, Action::Ui(UiAction::OpenCommitFilesPanel));
        let [Command::RefreshCommitFiles { commit_id }] = commands.as_slice() else {
            panic!("expected commit files refresh");
        };
        update(
            &mut state,
            Action::GitResult(GitResult::CommitFiles {
                commit_id: commit_id.clone(),
                result: Ok(vec![ratagit_core::CommitFileEntry {
                    path: "src/lib.rs".to_string(),
                    old_path: None,
                    status: ratagit_core::CommitFileStatus::Added,
                }]),
            }),
        );

        let lines = render_commits_lines(&state, 4);
        let line = lines
            .iter()
            .find(|line| line.text.contains("A lib.rs"))
            .expect("added file row should render");
        let spans = line.spans.as_ref().expect("tree row should have spans");

        assert_eq!(line.role, RowRole::Normal);
        assert!(
            spans
                .iter()
                .any(|span| span.text == "A" && span.style == row_style(RowRole::DiffAdd))
        );
        assert!(
            spans.iter().any(
                |span| span.text == " lib.rs" && span.style == ratatui::style::Style::default()
            )
        );
    }

    #[test]
    fn scroll_window_uses_bottom_reserve_while_moving_down() {
        assert_eq!(scroll_window_start(30, 20, 0, 8), 16);
    }

    #[test]
    fn scroll_window_reverses_up_without_immediate_top_jump() {
        assert_eq!(scroll_window_start(30, 24, 21, 8), 21);
        assert_eq!(scroll_window_start(30, 23, 21, 8), 20);
    }

    #[test]
    fn scroll_window_reverses_down_without_immediate_bottom_jump() {
        assert_eq!(scroll_window_start(30, 21, 17, 8), 17);
        assert_eq!(scroll_window_start(30, 22, 17, 8), 18);
    }

    #[test]
    fn branches_panel_keeps_window_stable_when_reversing_inside_threshold() {
        let mut state = branch_scroll_state(30);
        for _ in 0..10 {
            update(
                &mut state,
                Action::Ui(UiAction::MoveDownInViewport { visible_lines: 8 }),
            );
        }
        let before = render_branches_lines(&state, 8);

        update(
            &mut state,
            Action::Ui(UiAction::MoveUpInViewport { visible_lines: 8 }),
        );
        update(
            &mut state,
            Action::Ui(UiAction::MoveDownInViewport { visible_lines: 8 }),
        );
        let after = render_branches_lines(&state, 8);

        assert_eq!(before[0].text, after[0].text);
        assert!(after[4].text.contains("branch-10"));
        assert!(after[4].selected);
    }

    #[test]
    fn commits_panel_keeps_window_stable_when_jk_repeats_in_middle() {
        let mut state = commit_scroll_state(30);
        for _ in 0..20 {
            update(
                &mut state,
                Action::Ui(UiAction::MoveDownInViewport { visible_lines: 8 }),
            );
        }
        for _ in 0..2 {
            update(
                &mut state,
                Action::Ui(UiAction::MoveUpInViewport { visible_lines: 8 }),
            );
        }
        let before = render_commits_lines(&state, 8);

        update(
            &mut state,
            Action::Ui(UiAction::MoveDownInViewport { visible_lines: 8 }),
        );
        update(
            &mut state,
            Action::Ui(UiAction::MoveUpInViewport { visible_lines: 8 }),
        );
        let after = render_commits_lines(&state, 8);

        assert_eq!(before[0].text, after[0].text);
        assert!(after[3].text.contains("commit 18"));
        assert!(after[3].selected);
    }

    #[test]
    fn branches_panel_projects_current_and_selected_rows() {
        let mut state = state_with_dirty_repo();
        state.ui.branches.selected = 1;

        let lines = render_branches_lines(&state, 2);

        assert_eq!(lines[0].text, " main");
        assert_eq!(lines[1].text, "  feature/mvp");
        assert!(lines[1].selected);
    }

    #[test]
    fn commits_panel_projects_four_commit_columns_and_selection() {
        let mut state = state_with_dirty_repo();
        state.ui.commits.selected = 1;

        let lines = render_commits_lines(&state, 3);

        assert_eq!(lines[0].text, "●  abc1234  RT  init project");
        assert_eq!(lines[1].text, "●  def5678  RT  wire commands");
        assert_eq!(lines[0].spans.as_ref().map(Vec::len), Some(7));
        assert!(lines[1].selected);
    }

    #[test]
    fn commits_panel_uses_three_row_threshold_scroll_window() {
        let mut state = commit_scroll_state(30);

        for _ in 0..4 {
            update(&mut state, Action::Ui(UiAction::MoveDown));
        }
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 0"));
        assert!(lines[4].selected);

        update(&mut state, Action::Ui(UiAction::MoveDown));
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 1"));
        assert!(lines[4].text.contains("commit 5"));
        assert!(lines[4].selected);
    }

    #[test]
    fn commits_panel_reversing_up_waits_for_top_threshold() {
        let mut state = commit_scroll_state(30);
        for _ in 0..10 {
            update(
                &mut state,
                Action::Ui(UiAction::MoveDownInViewport { visible_lines: 8 }),
            );
        }

        update(
            &mut state,
            Action::Ui(UiAction::MoveUpInViewport { visible_lines: 8 }),
        );
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 6"));
        assert!(lines[3].text.contains("commit 9"));
        assert!(lines[3].selected);

        update(
            &mut state,
            Action::Ui(UiAction::MoveUpInViewport { visible_lines: 8 }),
        );
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 5"));
        assert!(lines[3].text.contains("commit 8"));
        assert!(lines[3].selected);
    }

    #[test]
    fn commits_panel_keeps_window_continuous_after_page_append() {
        let mut state = commit_scroll_state(COMMITS_PAGE_SIZE);
        state.repo.commits.has_more = true;

        for _ in 0..COMMITS_PAGE_SIZE - 1 {
            update(&mut state, Action::Ui(UiAction::MoveDown));
        }
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 92"));
        assert!(lines[7].text.contains("commit 99"));
        assert!(lines[7].selected);

        update(&mut state, Action::Ui(UiAction::MoveDown));

        let epoch = state.repo.commits.pagination_epoch;
        update(
            &mut state,
            Action::GitResult(GitResult::CommitsPage {
                offset: COMMITS_PAGE_SIZE,
                limit: COMMITS_PAGE_SIZE,
                epoch,
                result: Ok(commit_page(COMMITS_PAGE_SIZE, COMMITS_PAGE_SIZE)),
            }),
        );

        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 96"));
        assert!(lines[4].text.contains("commit 100"));
        assert!(lines[4].selected);
    }

    #[test]
    fn stash_panel_projects_selected_entry() {
        let state = state_with_dirty_repo();

        let lines = render_stash_lines(&state, 1);

        assert_eq!(lines[0].text, " stash@{0} WIP on main: local test");
        assert!(lines[0].selected);
    }

    #[test]
    fn details_panel_uses_last_left_focus_projection() {
        let mut state = state_with_dirty_repo();
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            }),
        );
        update(
            &mut state,
            Action::GitResult(GitResult::BranchDetailsLog {
                branch: "main".to_string(),
                result: Ok("\u{1b}[33m*\u{1b}[m \u{1b}[33mcommit abc1234\u{1b}[m".to_string()),
            }),
        );
        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
        );

        let lines = render_details_lines(&state, 4);

        assert_eq!(lines[0].text, "  * commit abc1234");
        assert!(lines[0].spans.is_some());
    }

    #[test]
    fn details_panel_projects_files_diff_as_native_output_lines() {
        let state = state_with_dirty_repo();

        let lines = render_details_lines(&state, 5);

        assert_eq!(lines[0].text, "  ### unstaged");
        assert!(lines.iter().all(|line| line.role == RowRole::Normal));
        assert!(
            lines
                .iter()
                .all(|line| line.spans.as_ref().is_some_and(|spans| !spans.is_empty()))
        );
    }

    #[test]
    fn details_panel_projects_commit_diff_with_git_ansi_spans() {
        let mut state = state_with_dirty_repo();
        state.ui.last_left_focus = PanelFocus::Commits;
        state.repo.details.commit_diff_target = Some("abc1234".to_string());
        state.repo.details.commit_diff = concat!(
            "commit abc1234\n",
            "Author: ratagit-tests\n",
            "\n",
            "\u{1b}[1mdiff --git a/a.txt b/a.txt\u{1b}[m\n",
            "\u{1b}[36m@@ -1 +1 @@\u{1b}[m\n",
            "\u{1b}[31m-old\u{1b}[m\n",
            "\u{1b}[32m+new\u{1b}[m"
        )
        .to_string();

        let lines = render_details_lines(&state, 7);

        assert_eq!(lines[0].text, "  commit abc1234");
        assert_eq!(lines[0].role, RowRole::Normal);
        assert!(lines.iter().all(|line| line.role == RowRole::Normal));
        let add_spans = lines[6]
            .spans
            .as_ref()
            .expect("commit diff should preserve ansi spans");
        assert_eq!(add_spans[1].text, "+new");
        assert_eq!(
            add_spans[1].style,
            ratatui::style::Style::default().fg(ratatui::style::Color::Green)
        );
    }

    #[test]
    fn details_panel_applies_app_context_scroll_offset() {
        let mut state = state_with_dirty_repo();
        state.ui.details.scroll_offset = 2;

        let lines = render_details_lines(&state, 3);

        assert_eq!(lines[0].text, "  @@ -1 +1 @@");
        assert_eq!(lines[1].text, "  -old README.md");
        assert_eq!(lines[2].text, "  +new README.md");
    }

    #[test]
    fn log_panel_projects_error_and_recent_notices() {
        let mut state = state_with_dirty_repo();
        update(
            &mut state,
            Action::Ui(UiAction::CreateCommit {
                message: String::new(),
            }),
        );
        update(
            &mut state,
            Action::GitResult(ratagit_core::GitResult::CreateCommit {
                message: String::new(),
                result: Err("nothing staged".to_string()),
            }),
        );

        let lines = render_log_lines(&state, 3);

        assert!(lines[0].text.contains("error=Failed to create commit"));
        assert!(
            lines
                .iter()
                .any(|line| line.text.contains("notice=Failed to create commit"))
        );
    }

    #[test]
    fn log_panel_reports_huge_repo_status_scan_skip() {
        let mut state = AppContext::default();
        state.repo.status.large_repo_mode = true;
        state.repo.status.status_scan_skipped = true;
        state.repo.status.untracked_scan_skipped = true;

        let lines = render_log_lines(&state, 3);

        assert_eq!(
            lines
                .iter()
                .map(|line| line.text.as_str())
                .collect::<Vec<_>>(),
            vec![
                "  status=huge repo metadata-only; file scan skipped",
                "  tip=focus Commits/Branches or narrow Git outside ratagit",
                "  notice=Ready",
            ]
        );
    }

    #[test]
    fn empty_lists_and_panels_render_without_empty_placeholders() {
        let mut state = AppContext::default();
        let _commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_empty_repo())),
        );

        assert!(render_files_lines(&state, 5).is_empty());
        assert!(render_stash_lines(&state, 5).is_empty());
        assert!(
            render_branches_lines(&state, 5)
                .iter()
                .all(|line| { !line.text.contains("<empty>") && !line.text.contains("<none>") })
        );
        assert!(
            render_details_lines(&state, 5)
                .iter()
                .all(|line| { !line.text.contains("<empty>") && !line.text.contains("<none>") })
        );
        assert!(
            render_log_lines(&state, 5)
                .iter()
                .all(|line| { !line.text.contains("<empty>") && !line.text.contains("<none>") })
        );
    }

    #[test]
    fn keys_panel_follows_focus_and_search_mode() {
        let mut state = state_with_dirty_repo();
        let files_shortcuts = shortcuts_for_state(&state);
        assert!(!files_shortcuts.contains("keys(files):"));
        assert!(files_shortcuts.contains("d discard"));
        assert!(files_shortcuts.contains("c commit"));
        assert!(files_shortcuts.contains("s stash"));
        assert!(files_shortcuts.contains("D reset"));
        assert!(!files_shortcuts.contains("v multi"));

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            }),
        );
        assert_eq!(
            shortcuts_for_state(&state),
            "space checkout  n new  d delete  r rebase"
        );

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            }),
        );
        assert!(!shortcuts_for_state(&state).contains("v multi"));

        let mut empty = AppContext::default();
        update(
            &mut empty,
            Action::GitResult(GitResult::Refreshed(fixture_empty_repo())),
        );
        update(&mut empty, Action::Ui(UiAction::StartSearch));
        update(&mut empty, Action::Ui(UiAction::InputSearchChar('m')));
        assert_eq!(shortcuts_for_state(&empty), "search: m");
    }

    #[test]
    fn keys_panel_switches_to_editor_help_when_editor_is_open() {
        let mut state = state_with_dirty_repo();
        update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
        assert!(!shortcuts_for_state(&state).contains("commit editor:"));
        assert!(shortcuts_for_state(&state).contains("Ctrl+J"));

        update(&mut state, Action::Ui(UiAction::OpenStashEditor));
        assert_eq!(
            shortcuts_for_state(&state),
            "arrows/Home/End cursor  Enter confirm  Esc cancel"
        );

        state.ui.editor.kind = None;
        update(&mut state, Action::Ui(UiAction::OpenResetMenu));
        assert_eq!(
            shortcuts_for_state(&state),
            "j/k select  Enter confirm  Esc cancel"
        );

        update(&mut state, Action::Ui(UiAction::CancelResetMenu));
        update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));
        assert_eq!(shortcuts_for_state(&state), "Enter confirm  Esc cancel");
    }
}
