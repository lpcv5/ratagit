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
    left_panel_content_len, panel_title, render_branches_lines, render_commits_lines,
    render_files_lines, render_stash_lines,
};
#[cfg(test)]
use panel_scroll::scroll_window_start;
pub(crate) use panel_shortcuts::shortcuts_for_state;
pub(crate) use panel_types::PanelLine;
#[cfg(test)]
use ratagit_core::{AppState, ScrollDirection};
#[cfg(test)]
mod tests {
    use ratagit_core::{
        Action, COMMITS_PAGE_SIZE, Command, GitResult, PanelFocus, UiAction, update,
    };
    use ratagit_testkit::{fixture_commit, fixture_dirty_repo, fixture_empty_repo};

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

    fn state_with_dirty_repo() -> AppState {
        let mut state = AppState::default();
        let commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
        );
        if let [Command::RefreshFilesDetailsDiff { paths }] = commands.as_slice() {
            let follow_up = update(
                &mut state,
                Action::GitResult(GitResult::FilesDetailsDiff {
                    paths: paths.clone(),
                    result: Ok(mock_diff_for_paths(paths)),
                }),
            );
            assert!(follow_up.is_empty());
        } else {
            panic!("unexpected commands after refresh: {commands:?}");
        }
        state
    }

    fn commit_scroll_state(count: usize) -> AppState {
        let mut state = state_with_dirty_repo();
        state.focus = PanelFocus::Commits;
        state.commits.items = (0..count)
            .map(|index| fixture_commit(&format!("{index:07x}"), &format!("commit {index}")))
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
        state.files.selected = 1;

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
        state.files.selected = 1;
        update(&mut state, Action::Ui(UiAction::ToggleSelectedDirectory));
        update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));

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
        assert_eq!(line.role, RowRole::DiffAdd);
        assert!(has_search_span(line, "lib"));
    }

    #[test]
    fn scroll_window_uses_bottom_reserve_while_moving_down() {
        assert_eq!(
            scroll_window_start(30, 20, Some(ScrollDirection::Down), 0, 8, 3),
            16
        );
    }

    #[test]
    fn scroll_window_reverses_up_without_immediate_top_jump() {
        assert_eq!(
            scroll_window_start(30, 24, Some(ScrollDirection::Up), 25, 8, 3),
            21
        );
        assert_eq!(
            scroll_window_start(30, 23, Some(ScrollDirection::Up), 25, 8, 3),
            20
        );
    }

    #[test]
    fn scroll_window_reverses_down_without_immediate_bottom_jump() {
        assert_eq!(
            scroll_window_start(30, 21, Some(ScrollDirection::Down), 20, 8, 3),
            17
        );
        assert_eq!(
            scroll_window_start(30, 22, Some(ScrollDirection::Down), 20, 8, 3),
            18
        );
    }

    #[test]
    fn branches_panel_projects_current_and_selected_rows() {
        let mut state = state_with_dirty_repo();
        state.branches.selected = 1;

        let lines = render_branches_lines(&state, 2);

        assert_eq!(lines[0].text, " main");
        assert_eq!(lines[1].text, "  feature/mvp");
        assert!(lines[1].selected);
    }

    #[test]
    fn commits_panel_projects_four_commit_columns_and_selection() {
        let mut state = state_with_dirty_repo();
        state.commits.selected = 1;

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
            update(&mut state, Action::Ui(UiAction::MoveDown));
        }

        update(&mut state, Action::Ui(UiAction::MoveUp));
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 6"));
        assert!(lines[3].text.contains("commit 9"));
        assert!(lines[3].selected);

        update(&mut state, Action::Ui(UiAction::MoveUp));
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 5"));
        assert!(lines[3].text.contains("commit 8"));
        assert!(lines[3].selected);
    }

    #[test]
    fn commits_panel_keeps_window_continuous_after_page_append() {
        let mut state = commit_scroll_state(COMMITS_PAGE_SIZE);
        state.commits.has_more = true;

        for _ in 0..COMMITS_PAGE_SIZE - 1 {
            update(&mut state, Action::Ui(UiAction::MoveDown));
        }
        let lines = render_commits_lines(&state, 8);
        assert!(lines[0].text.contains("commit 92"));
        assert!(lines[7].text.contains("commit 99"));
        assert!(lines[7].selected);

        update(&mut state, Action::Ui(UiAction::MoveDown));

        let epoch = state.commits.pagination_epoch;
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
    fn details_panel_projects_files_diff_with_colored_roles() {
        let state = state_with_dirty_repo();

        let lines = render_details_lines(&state, 5);

        assert_eq!(lines[0].text, "  ### unstaged");
        assert_eq!(lines[0].role, RowRole::DiffSection);
        assert_eq!(lines[1].role, RowRole::DiffMeta);
        assert_eq!(lines[2].role, RowRole::DiffHunk);
        assert_eq!(lines[3].role, RowRole::DiffRemove);
        assert_eq!(lines[4].role, RowRole::DiffAdd);
    }

    #[test]
    fn details_panel_projects_commit_diff_with_colored_patch_roles() {
        let mut state = state_with_dirty_repo();
        state.last_left_focus = PanelFocus::Commits;
        state.details.commit_diff_target = Some("abc1234".to_string());
        state.details.commit_diff =
            "commit abc1234\nAuthor: ratagit-tests\n\ndiff --git a/a.txt b/a.txt\n@@ -1 +1 @@\n-old\n+new"
                .to_string();

        let lines = render_details_lines(&state, 7);

        assert_eq!(lines[0].text, "  commit abc1234");
        assert_eq!(lines[0].role, RowRole::Normal);
        assert_eq!(lines[3].role, RowRole::DiffMeta);
        assert_eq!(lines[4].role, RowRole::DiffHunk);
        assert_eq!(lines[5].role, RowRole::DiffRemove);
        assert_eq!(lines[6].role, RowRole::DiffAdd);
    }

    #[test]
    fn details_panel_applies_app_state_scroll_offset() {
        let mut state = state_with_dirty_repo();
        state.details.scroll_offset = 2;

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
    fn empty_lists_and_panels_render_without_empty_placeholders() {
        let mut state = AppState::default();
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
        assert!(files_shortcuts.contains("keys(files):"));
        assert!(files_shortcuts.contains("d discard"));
        assert!(files_shortcuts.contains("c commit"));
        assert!(files_shortcuts.contains("s stash(all|selected)"));
        assert!(files_shortcuts.contains("D reset"));

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            }),
        );
        assert_eq!(
            shortcuts_for_state(&state),
            "keys(branches): space checkout | n new | d delete | r rebase"
        );

        let mut empty = AppState::default();
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
        assert!(shortcuts_for_state(&state).contains("commit editor:"));
        assert!(shortcuts_for_state(&state).contains("Ctrl+J"));

        update(&mut state, Action::Ui(UiAction::OpenStashEditor));
        assert_eq!(
            shortcuts_for_state(&state),
            "stash editor: arrows/Home/End cursor | Enter confirm | Esc cancel"
        );

        state.editor.kind = None;
        update(&mut state, Action::Ui(UiAction::OpenResetMenu));
        assert_eq!(
            shortcuts_for_state(&state),
            "reset: j/k select | Enter confirm | Esc cancel"
        );

        update(&mut state, Action::Ui(UiAction::CancelResetMenu));
        update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));
        assert_eq!(
            shortcuts_for_state(&state),
            "discard: Enter confirm | Esc cancel"
        );
    }
}
