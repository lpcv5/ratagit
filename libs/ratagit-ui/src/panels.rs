use ratagit_core::{
    AppState, BranchEntry, CommitEntry, FileInputMode, FileRowKind, FileTreeRow, PanelFocus,
    ScrollDirection, StashEntry, build_file_tree_rows,
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
    match state.last_left_focus {
        PanelFocus::Files => render_files_details_lines(state, max_lines),
        // TODO(details-branches): replace placeholder with selected-branch git log graph.
        PanelFocus::Branches => render_placeholder_details_lines(
            "  details(branches): pending git log --graph implementation",
            max_lines,
        ),
        // TODO(details-commits): replace placeholder with commit-focused details projection.
        PanelFocus::Commits => render_placeholder_details_lines(
            "  details(commits): pending details implementation",
            max_lines,
        ),
        // TODO(details-stash): replace placeholder with stash entry details projection.
        PanelFocus::Stash => render_placeholder_details_lines(
            "  details(stash): pending details implementation",
            max_lines,
        ),
        PanelFocus::Details | PanelFocus::Log => Vec::new(),
    }
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

fn render_files_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }

    if let Some(error) = &state.details.files_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.details.files_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(files): no diff for current selection",
            RowRole::Muted,
        )];
    }

    // TODO(files-hunks): upgrade details rows into selectable hunk models for partial staging.
    state
        .details
        .files_diff
        .lines()
        .map(|line| PanelLine::new(format!("  {line}"), classify_diff_row_role(line)))
        .take(max_lines)
        .collect()
}

fn render_placeholder_details_lines(message: &str, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }
    vec![PanelLine::new(message, RowRole::Muted)]
}

fn classify_diff_row_role(line: &str) -> RowRole {
    if line.starts_with("### ") {
        return RowRole::DiffSection;
    }
    if line.starts_with("diff --git")
        || line.starts_with("index ")
        || line.starts_with("--- ")
        || line.starts_with("+++ ")
    {
        return RowRole::DiffMeta;
    }
    if line.starts_with("@@") {
        return RowRole::DiffHunk;
    }
    if line.starts_with('+') && !line.starts_with("+++") {
        return RowRole::DiffAdd;
    }
    if line.starts_with('-') && !line.starts_with("---") {
        return RowRole::DiffRemove;
    }
    RowRole::Normal
}

pub(crate) fn shortcuts_for_state(state: &AppState) -> String {
    if let Some(editor) = &state.editor.kind {
        return match editor {
            ratagit_core::EditorKind::Commit { .. } => {
                "commit editor: Tab switch | Ctrl+J newline(body) | Enter confirm | Esc cancel"
                    .to_string()
            }
            ratagit_core::EditorKind::Stash { .. } => {
                "stash editor: Enter confirm | Esc cancel".to_string()
            }
        };
    }

    if state.focus == PanelFocus::Files && state.files.mode == FileInputMode::SearchInput {
        return format!("search: {}", state.files.search_query);
    }
    match state.focus {
        PanelFocus::Files => {
            "keys(files): space stage/unstage | c commit | s stash(all|selected) | v multi | enter expand | / search".to_string()
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
    use ratagit_core::{Action, Command, GitResult, PanelFocus, UiAction, update};
    use ratagit_testkit::{fixture_dirty_repo, fixture_empty_repo};

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

        assert_eq!(
            lines[0].text,
            "  details(branches): pending git log --graph implementation"
        );
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
        assert!(files_shortcuts.contains("c commit"));
        assert!(files_shortcuts.contains("s stash(all|selected)"));

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

    #[test]
    fn keys_panel_switches_to_editor_help_when_editor_is_open() {
        let mut state = state_with_dirty_repo();
        update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
        assert!(shortcuts_for_state(&state).contains("commit editor:"));
        assert!(shortcuts_for_state(&state).contains("Ctrl+J"));

        update(&mut state, Action::Ui(UiAction::OpenStashEditor));
        assert_eq!(
            shortcuts_for_state(&state),
            "stash editor: Enter confirm | Esc cancel"
        );
    }
}
