use ratagit_core::{
    AppState, BranchEntry, CommitEntry, FileInputMode, FileRowKind, FileTreeRow, PanelFocus,
    ScrollDirection, StashEntry, build_file_tree_rows, selected_row, selected_target_paths,
};

use crate::theme::{
    ICON_BATCH_SELECTED, ICON_BRANCH, ICON_COMMIT, ICON_DIRECTORY_CLOSED, ICON_DIRECTORY_OPEN,
    ICON_FILE, ICON_FILE_STAGED, ICON_FILE_UNTRACKED, ICON_SEARCH_MATCH, ICON_STASH, RowRole,
    panel_label,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: String,
    pub(crate) selected: bool,
    pub(crate) role: RowRole,
}

impl PanelLine {
    fn new(text: impl Into<String>, role: RowRole) -> Self {
        Self {
            text: text.into(),
            selected: false,
            role,
        }
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }
}

pub(crate) fn panel_title(panel: PanelFocus) -> &'static str {
    panel_label(panel)
}

pub(crate) fn left_panel_content_len(state: &AppState, panel: PanelFocus) -> usize {
    match panel {
        PanelFocus::Files => build_file_tree_rows(&state.files).len(),
        PanelFocus::Branches => state.branches.items.len(),
        PanelFocus::Commits => state.commits.items.len().saturating_add(1),
        PanelFocus::Stash => state.stash.items.len(),
        PanelFocus::Details | PanelFocus::Log => 0,
    }
}

pub(crate) fn render_files_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let rows = build_file_tree_rows(&state.files);
    render_indexed_entries(
        &rows,
        state.files.selected,
        state.files.scroll_direction,
        state.files.scroll_direction_origin,
        max_lines,
        format_file_tree_row,
        file_tree_row_role,
    )
}

pub(crate) fn render_branches_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    render_indexed_entries(
        &state.branches.items,
        state.branches.selected,
        None,
        state.branches.selected,
        max_lines,
        format_branch_entry,
        branch_entry_role,
    )
}

pub(crate) fn render_commits_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let mut lines = render_indexed_entries(
        &state.commits.items,
        state.commits.selected,
        None,
        state.commits.selected,
        max_lines.saturating_sub(1),
        format_commit_entry,
        |_| RowRole::Normal,
    );
    if max_lines > 0 {
        lines.push(PanelLine::new(
            format!("  draft={}", state.commits.draft_message),
            RowRole::Muted,
        ));
    }
    lines
}

pub(crate) fn render_stash_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    render_indexed_entries(
        &state.stash.items,
        state.stash.selected,
        None,
        state.stash.selected,
        max_lines,
        format_stash_entry,
        |_| RowRole::Normal,
    )
}

pub(crate) fn render_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let mut lines = Vec::new();
    lines.push(PanelLine::new(
        format!("  current={:?}", state.last_left_focus),
        RowRole::Muted,
    ));
    match state.last_left_focus {
        PanelFocus::Files => {
            if let Some(row) = selected_row(&state.files) {
                let target_count = selected_target_paths(&state.files).len();
                lines.push(PanelLine::new(
                    format!(
                        "  {}={}",
                        if row.kind == FileRowKind::Directory {
                            "dir"
                        } else {
                            "file"
                        },
                        row.path
                    ),
                    RowRole::Normal,
                ));
                lines.push(PanelLine::new(
                    format!("  targets={target_count}"),
                    RowRole::Muted,
                ));
                lines.push(PanelLine::new(
                    format!("  staged={}", if row.staged { "yes" } else { "no" }),
                    if row.staged {
                        RowRole::FileStaged
                    } else {
                        RowRole::Muted
                    },
                ));
            }
        }
        PanelFocus::Branches => {
            if let Some(entry) = state.branches.items.get(state.branches.selected) {
                lines.push(PanelLine::new(
                    format!("  branch={}", entry.name),
                    branch_entry_role(entry),
                ));
                lines.push(PanelLine::new(
                    format!(
                        "  is_current={}",
                        if entry.is_current { "yes" } else { "no" }
                    ),
                    if entry.is_current {
                        RowRole::CurrentBranch
                    } else {
                        RowRole::Muted
                    },
                ));
            }
        }
        PanelFocus::Commits => {
            if let Some(entry) = state.commits.items.get(state.commits.selected) {
                lines.push(PanelLine::new(
                    format!("  commit={} {}", entry.id, entry.summary),
                    RowRole::Normal,
                ));
            }
        }
        PanelFocus::Stash => {
            if let Some(entry) = state.stash.items.get(state.stash.selected) {
                lines.push(PanelLine::new(
                    format!("  stash={} {}", entry.id, entry.summary),
                    RowRole::Normal,
                ));
            }
        }
        PanelFocus::Details | PanelFocus::Log => {}
    }
    lines.push(PanelLine::new(
        format!("  summary={}", state.status.summary),
        RowRole::Muted,
    ));
    lines.into_iter().take(max_lines).collect()
}

pub(crate) fn render_log_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let mut lines = Vec::new();
    if let Some(error) = &state.status.last_error {
        lines.push(PanelLine::new(format!("  error={error}"), RowRole::Error));
    }

    let keep = max_lines.saturating_sub(lines.len());
    if keep > 0 {
        let start = state.notices.len().saturating_sub(keep);
        for notice in &state.notices[start..] {
            lines.push(PanelLine::new(
                format!("  notice={notice}"),
                RowRole::Notice,
            ));
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
    scroll_direction: Option<ScrollDirection>,
    scroll_direction_origin: usize,
    max_lines: usize,
    format_item: impl Fn(&T) -> String,
    item_role: impl Fn(&T) -> RowRole,
) -> Vec<PanelLine> {
    render_indexed_entries_window(
        items,
        selected,
        scroll_direction,
        scroll_direction_origin,
        max_lines,
        format_item,
        item_role,
    )
}

fn render_indexed_entries_window<T>(
    items: &[T],
    selected: usize,
    scroll_direction: Option<ScrollDirection>,
    scroll_direction_origin: usize,
    max_lines: usize,
    format_item: impl Fn(&T) -> String,
    item_role: impl Fn(&T) -> RowRole,
) -> Vec<PanelLine> {
    const SCROLL_RESERVE: usize = 3;

    if max_lines == 0 {
        return Vec::new();
    }
    if items.is_empty() {
        return Vec::new();
    }
    let start = scroll_window_start(
        items.len(),
        selected,
        scroll_direction,
        scroll_direction_origin,
        max_lines,
        SCROLL_RESERVE,
    );
    items
        .iter()
        .enumerate()
        .skip(start)
        .take(max_lines)
        .map(|(index, item)| {
            PanelLine::new(format_item(item), item_role(item)).selected(index == selected)
        })
        .collect()
}

fn scroll_window_start(
    len: usize,
    selected: usize,
    scroll_direction: Option<ScrollDirection>,
    scroll_direction_origin: usize,
    max_lines: usize,
    reserve: usize,
) -> usize {
    if max_lines == 0 || len <= max_lines {
        return 0;
    }
    let max_start = len.saturating_sub(max_lines);
    let selected = selected.min(len - 1);
    match scroll_direction {
        Some(ScrollDirection::Up) => {
            let previous_start =
                bottom_reserve_start(scroll_direction_origin, max_lines, max_start, reserve);
            if selected >= previous_start.saturating_add(reserve) {
                previous_start
            } else {
                top_reserve_start(selected, max_start, reserve)
            }
        }
        Some(ScrollDirection::Down) => {
            let previous_start = top_reserve_start(scroll_direction_origin, max_start, reserve);
            let bottom_threshold = previous_start
                .saturating_add(max_lines.saturating_sub(1))
                .saturating_sub(reserve);
            if selected <= bottom_threshold {
                previous_start
            } else {
                bottom_reserve_start(selected, max_lines, max_start, reserve)
            }
        }
        None => bottom_reserve_start(selected, max_lines, max_start, reserve),
    }
}

fn top_reserve_start(selected: usize, max_start: usize, reserve: usize) -> usize {
    selected.saturating_sub(reserve).min(max_start)
}

fn bottom_reserve_start(
    selected: usize,
    max_lines: usize,
    max_start: usize,
    reserve: usize,
) -> usize {
    selected
        .saturating_add(1 + reserve)
        .saturating_sub(max_lines)
        .min(max_start)
}

pub fn format_file_tree_row(row: &FileTreeRow) -> String {
    let indent = "  ".repeat(row.depth);
    let batch = if row.selected_for_batch {
        ICON_BATCH_SELECTED
    } else {
        " "
    };
    let matched = if row.matched { ICON_SEARCH_MATCH } else { " " };
    let body = match row.kind {
        FileRowKind::Directory => {
            let marker = if row.expanded {
                ICON_DIRECTORY_OPEN
            } else {
                ICON_DIRECTORY_CLOSED
            };
            format!("{marker} {}/", row.name)
        }
        FileRowKind::File => {
            let marker = if row.untracked {
                ICON_FILE_UNTRACKED
            } else if row.staged {
                ICON_FILE_STAGED
            } else {
                ICON_FILE
            };
            format!("{marker} {}", row.name)
        }
    };
    format!("{batch}{matched} {indent}{body}")
}

pub fn format_commit_entry(entry: &CommitEntry) -> String {
    format!("{ICON_COMMIT} {} {}", entry.id, entry.summary)
}

pub fn format_branch_entry(entry: &BranchEntry) -> String {
    if entry.is_current {
        format!("{ICON_BRANCH} {}", entry.name)
    } else {
        format!("  {}", entry.name)
    }
}

pub fn format_stash_entry(entry: &StashEntry) -> String {
    format!("{ICON_STASH} {} {}", entry.id, entry.summary)
}

fn file_tree_row_role(row: &FileTreeRow) -> RowRole {
    if row.selected_for_batch {
        RowRole::BatchSelected
    } else if row.matched {
        RowRole::SearchMatch
    } else if row.untracked {
        RowRole::FileUntracked
    } else if row.staged {
        RowRole::FileStaged
    } else {
        RowRole::Normal
    }
}

fn branch_entry_role(entry: &BranchEntry) -> RowRole {
    if entry.is_current {
        RowRole::CurrentBranch
    } else {
        RowRole::Normal
    }
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
        update(&mut state, Action::Ui(UiAction::StartFileSearch));
        update(&mut state, Action::Ui(UiAction::InputFileSearchChar('l')));
        update(&mut state, Action::Ui(UiAction::InputFileSearchChar('i')));

        let lines = render_files_lines(&state, 4);

        assert!(lines.iter().any(|line| line.text.contains("    lib.rs")));
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
    fn commits_panel_projects_selected_commit_and_draft() {
        let mut state = state_with_dirty_repo();
        state.commits.selected = 1;
        state.commits.draft_message = "ship it".to_string();

        let lines = render_commits_lines(&state, 3);

        assert_eq!(lines[0].text, " abc1234 init project");
        assert_eq!(lines[1].text, " def5678 wire commands");
        assert_eq!(lines[2].text, "  draft=ship it");
        assert!(lines[1].selected);
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
            Action::Ui(UiAction::FocusPanel {
                panel: PanelFocus::Details,
            }),
        );

        let lines = render_details_lines(&state, 4);

        assert_eq!(lines[0].text, "  current=Branches");
        assert_eq!(lines[1].text, "  branch=main");
        assert_eq!(lines[2].text, "  is_current=yes");
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
        let commands = update(
            &mut state,
            Action::GitResult(GitResult::Refreshed(fixture_empty_repo())),
        );
        assert!(commands.is_empty());

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
