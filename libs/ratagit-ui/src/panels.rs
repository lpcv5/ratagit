use ratagit_core::{
    AppState, BranchEntry, CommitEntry, FileInputMode, FileRowKind, FileTreeRow, PanelFocus,
    ScrollDirection, StashEntry, build_file_tree_rows, selected_row, selected_target_paths,
};

pub(crate) fn panel_title(panel: PanelFocus) -> &'static str {
    match panel {
        PanelFocus::Files => "Files",
        PanelFocus::Branches => "Branches",
        PanelFocus::Commits => "Commits",
        PanelFocus::Stash => "Stash",
        PanelFocus::Details => "Details",
        PanelFocus::Log => "Log",
    }
}

pub(crate) fn render_files_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let rows = build_file_tree_rows(&state.files);
    render_indexed_entries_with_direction(
        &rows,
        state.files.selected,
        state.files.last_scroll_direction,
        max_lines,
        format_file_tree_row,
    )
}

pub(crate) fn render_branches_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    render_indexed_entries(
        &state.branches.items,
        state.branches.selected,
        max_lines,
        format_branch_entry,
    )
}

pub(crate) fn render_commits_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let mut lines = render_indexed_entries(
        &state.commits.items,
        state.commits.selected,
        max_lines.saturating_sub(1),
        format_commit_entry,
    );
    if max_lines > 0 {
        lines.push(format!("  draft={}", state.commits.draft_message));
    }
    lines
}

pub(crate) fn render_stash_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    render_indexed_entries(
        &state.stash.items,
        state.stash.selected,
        max_lines,
        format_stash_entry,
    )
}

pub(crate) fn render_details_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!("  current={:?}", state.last_left_focus));
    match state.last_left_focus {
        PanelFocus::Files => {
            if let Some(row) = selected_row(&state.files) {
                let target_count = selected_target_paths(&state.files).len();
                lines.push(format!(
                    "  {}={}",
                    if row.kind == FileRowKind::Directory {
                        "dir"
                    } else {
                        "file"
                    },
                    row.path
                ));
                lines.push(format!("  targets={target_count}"));
                lines.push(format!(
                    "  staged={}",
                    if row.staged { "yes" } else { "no" }
                ));
            } else {
                lines.push("  file=<empty>".to_string());
            }
        }
        PanelFocus::Branches => {
            if let Some(entry) = state.branches.items.get(state.branches.selected) {
                lines.push(format!("  branch={}", entry.name));
                lines.push(format!(
                    "  is_current={}",
                    if entry.is_current { "yes" } else { "no" }
                ));
            } else {
                lines.push("  branch=<empty>".to_string());
            }
        }
        PanelFocus::Commits => {
            if let Some(entry) = state.commits.items.get(state.commits.selected) {
                lines.push(format!("  commit={} {}", entry.id, entry.summary));
            } else {
                lines.push("  commit=<empty>".to_string());
            }
        }
        PanelFocus::Stash => {
            if let Some(entry) = state.stash.items.get(state.stash.selected) {
                lines.push(format!("  stash={} {}", entry.id, entry.summary));
            } else {
                lines.push("  stash=<empty>".to_string());
            }
        }
        PanelFocus::Details | PanelFocus::Log => {}
    }
    lines.push(format!("  summary={}", state.status.summary));
    lines.into_iter().take(max_lines).collect()
}

pub(crate) fn render_log_lines(state: &AppState, max_lines: usize) -> Vec<String> {
    let mut lines = Vec::new();
    if let Some(error) = &state.status.last_error {
        lines.push(format!("  error={error}"));
    } else {
        lines.push("  error=<none>".to_string());
    }

    let keep = max_lines.saturating_sub(lines.len());
    if keep > 0 {
        let start = state.notices.len().saturating_sub(keep);
        for notice in &state.notices[start..] {
            lines.push(format!("  notice={notice}"));
        }
    }
    lines.into_iter().take(max_lines).collect()
}

pub(crate) fn shortcuts_for_state(state: &AppState) -> String {
    if state.focus == PanelFocus::Files && state.files.mode == FileInputMode::SearchInput {
        return format!("search: {}", state.files.search_query);
    }
    match state.focus {
        PanelFocus::Files => {
            "keys(files): space stage/unstage | s stash | v multi | enter expand | / search"
                .to_string()
        }
        PanelFocus::Branches => "keys(branches): b create branch | o checkout".to_string(),
        PanelFocus::Commits => "keys(commits): c commit".to_string(),
        PanelFocus::Stash => "keys(stash): p stash push | O stash pop".to_string(),
        PanelFocus::Details | PanelFocus::Log => String::new(),
    }
}

fn render_indexed_entries<T>(
    items: &[T],
    selected: usize,
    max_lines: usize,
    format_item: impl Fn(&T) -> String,
) -> Vec<String> {
    render_indexed_entries_with_direction(
        items,
        selected,
        ScrollDirection::Down,
        max_lines,
        format_item,
    )
}

fn render_indexed_entries_with_direction<T>(
    items: &[T],
    selected: usize,
    direction: ScrollDirection,
    max_lines: usize,
    format_item: impl Fn(&T) -> String,
) -> Vec<String> {
    const SCROLL_RESERVE: usize = 3;

    if max_lines == 0 {
        return Vec::new();
    }
    if items.is_empty() {
        return vec!["  <empty>".to_string()];
    }
    let max_start = items.len().saturating_sub(max_lines);
    let start = match direction {
        ScrollDirection::Up => selected.saturating_sub(SCROLL_RESERVE),
        ScrollDirection::Down => selected
            .saturating_add(1 + SCROLL_RESERVE)
            .saturating_sub(max_lines),
    }
    .min(max_start);
    items
        .iter()
        .enumerate()
        .skip(start)
        .take(max_lines)
        .map(|(index, item)| {
            if index == selected {
                format!("> {}", format_item(item))
            } else {
                format!("  {}", format_item(item))
            }
        })
        .collect()
}

pub fn format_file_tree_row(row: &FileTreeRow) -> String {
    let indent = "  ".repeat(row.depth);
    let batch = if row.selected_for_batch { "*" } else { " " };
    let matched = if row.matched { "!" } else { " " };
    let body = match row.kind {
        FileRowKind::Directory => {
            let marker = if row.expanded { "[-]" } else { "[+]" };
            format!("{marker} {}/", row.name)
        }
        FileRowKind::File => {
            let marker = if row.untracked {
                "[?]"
            } else if row.staged {
                "[S]"
            } else {
                "[ ]"
            };
            format!("{marker} {}", row.name)
        }
    };
    format!("{batch}{matched}{indent}{body}")
}

pub fn format_commit_entry(entry: &CommitEntry) -> String {
    format!("{} {}", entry.id, entry.summary)
}

pub fn format_branch_entry(entry: &BranchEntry) -> String {
    format!(
        "{} {}",
        if entry.is_current { "*" } else { " " },
        entry.name
    )
}

pub fn format_stash_entry(entry: &StashEntry) -> String {
    format!("{} {}", entry.id, entry.summary)
}

#[cfg(test)]
mod tests {
    use ratagit_core::{Action, GitResult, PanelFocus, UiAction, update};
    use ratagit_testkit::{fixture_dirty_repo, fixture_empty_repo};

    use super::*;

    fn state_with_dirty_repo() -> AppState {
        let mut state = AppState::default();
        let commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
        );
        assert!(commands.is_empty());
        state
    }

    #[test]
    fn files_panel_projects_tree_rows_and_selection() {
        let mut state = state_with_dirty_repo();
        state.files.selected = 1;

        let lines = render_files_lines(&state, 4);

        assert_eq!(lines[0], "    [?] README.md");
        assert_eq!(lines[1], ">   [-] src/");
        assert_eq!(lines[2], "      [ ] lib.rs");
        assert_eq!(lines[3], "      [S] main.rs");
    }

    #[test]
    fn files_panel_projects_collapsed_directory_and_multi_select() {
        let mut state = state_with_dirty_repo();
        state.files.selected = 1;
        update(&mut state, Action::Ui(UiAction::ToggleSelectedDirectory));
        update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));

        let lines = render_files_lines(&state, 2);

        assert_eq!(lines[0], "    [?] README.md");
        assert_eq!(lines[1], "> * [+] src/");
    }

    #[test]
    fn files_panel_marks_search_matches() {
        let mut state = state_with_dirty_repo();
        update(&mut state, Action::Ui(UiAction::StartFileSearch));
        update(&mut state, Action::Ui(UiAction::InputFileSearchChar('l')));
        update(&mut state, Action::Ui(UiAction::InputFileSearchChar('i')));

        let lines = render_files_lines(&state, 4);

        assert!(lines.iter().any(|line| line.contains("!  [ ] lib.rs")));
    }

    #[test]
    fn branches_panel_projects_current_and_selected_rows() {
        let mut state = state_with_dirty_repo();
        state.branches.selected = 1;

        let lines = render_branches_lines(&state, 2);

        assert_eq!(lines[0], "  * main");
        assert_eq!(lines[1], ">   feature/mvp");
    }

    #[test]
    fn commits_panel_projects_selected_commit_and_draft() {
        let mut state = state_with_dirty_repo();
        state.commits.selected = 1;
        state.commits.draft_message = "ship it".to_string();

        let lines = render_commits_lines(&state, 3);

        assert_eq!(lines[0], "  abc1234 init project");
        assert_eq!(lines[1], "> def5678 wire commands");
        assert_eq!(lines[2], "  draft=ship it");
    }

    #[test]
    fn stash_panel_projects_selected_entry() {
        let state = state_with_dirty_repo();

        let lines = render_stash_lines(&state, 1);

        assert_eq!(lines[0], "> stash@{0} WIP on main: local test");
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
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
        );

        let lines = render_details_lines(&state, 4);

        assert_eq!(lines[0], "  current=Branches");
        assert_eq!(lines[1], "  branch=main");
        assert_eq!(lines[2], "  is_current=yes");
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

        assert!(lines[0].contains("error=Failed to create commit"));
        assert!(
            lines
                .iter()
                .any(|line| line.contains("notice=Failed to create commit"))
        );
    }

    #[test]
    fn keys_panel_follows_focus_and_search_mode() {
        let mut state = state_with_dirty_repo();
        assert!(shortcuts_for_state(&state).contains("keys(files):"));

        update(
            &mut state,
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            }),
        );
        assert_eq!(
            shortcuts_for_state(&state),
            "keys(branches): b create branch | o checkout"
        );

        let mut empty = AppState::default();
        update(
            &mut empty,
            Action::GitResult(GitResult::Refreshed(fixture_empty_repo())),
        );
        update(&mut empty, Action::Ui(UiAction::StartFileSearch));
        update(&mut empty, Action::Ui(UiAction::InputFileSearchChar('m')));
        assert_eq!(shortcuts_for_state(&empty), "search: m");
    }
}
