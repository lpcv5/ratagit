use ratagit_core::{
    AppState, BranchEntry, CommitEntry, CommitHashStatus, FileInputMode, FileRowKind, FileTreeRow,
    PanelFocus, ScrollDirection, StashEntry, build_file_tree_rows, commit_is_selected_for_batch,
    file_tree_rows,
};
use ratatui::style::{Color, Modifier, Style};
use unicode_width::UnicodeWidthChar;

use crate::theme::{
    ICON_BATCH_SELECTED, ICON_BRANCH, ICON_DIRECTORY_CLOSED, ICON_DIRECTORY_OPEN, ICON_FILE,
    ICON_FILE_STAGED, ICON_FILE_UNTRACKED, ICON_SEARCH_MATCH, ICON_STASH, RowRole, panel_label,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelLine {
    pub(crate) text: String,
    pub(crate) selected: bool,
    pub(crate) role: RowRole,
    pub(crate) spans: Option<Vec<PanelSpan>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PanelSpan {
    pub(crate) text: String,
    pub(crate) style: Style,
}

impl PanelLine {
    fn new(text: impl Into<String>, role: RowRole) -> Self {
        Self {
            text: text.into(),
            selected: false,
            role,
            spans: None,
        }
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn styled_spans(mut self, spans: Vec<PanelSpan>) -> Self {
        self.spans = Some(spans);
        self
    }
}

pub(crate) fn panel_title(panel: PanelFocus) -> &'static str {
    panel_label(panel)
}

pub(crate) fn left_panel_content_len(state: &AppState, panel: PanelFocus) -> usize {
    match panel {
        PanelFocus::Files => {
            if file_tree_rows(&state.files).is_empty() && !state.files.items.is_empty() {
                build_file_tree_rows(&state.files).len()
            } else {
                file_tree_rows(&state.files).len()
            }
        }
        PanelFocus::Branches => state.branches.items.len(),
        PanelFocus::Commits => state.commits.items.len(),
        PanelFocus::Stash => state.stash.items.len(),
        PanelFocus::Details | PanelFocus::Log => 0,
    }
}

pub(crate) fn render_files_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let rows = if file_tree_rows(&state.files).is_empty() && !state.files.items.is_empty() {
        build_file_tree_rows(&state.files)
    } else {
        file_tree_rows(&state.files).to_vec()
    };
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
    render_indexed_entries_window_with(
        &state.commits.items,
        state.commits.selected,
        state.commits.scroll_direction,
        state.commits.scroll_direction_origin,
        max_lines,
        |index, entry| {
            let role = if commit_is_selected_for_batch(&state.commits, entry) {
                RowRole::BatchSelected
            } else {
                RowRole::Normal
            };
            PanelLine::new(format_commit_entry(entry), role)
                .selected(index == state.commits.selected)
                .styled_spans(commit_entry_spans(entry))
        },
    )
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
        PanelFocus::Branches => render_branch_details_lines(state, max_lines),
        PanelFocus::Commits => render_commit_details_lines(state, max_lines),
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
    if state.work.refresh_pending {
        lines.push(PanelLine::new(
            "  work=refreshing repository",
            RowRole::Notice,
        ));
    }
    if let Some(operation) = &state.work.operation_pending {
        lines.push(PanelLine::new(
            format!("  work=running {operation}"),
            RowRole::Notice,
        ));
    }
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

    if state.work.details_pending && state.details.files_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(files): loading diff",
            RowRole::Muted,
        )];
    }

    if state.details.files_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(files): no diff for current selection",
            RowRole::Muted,
        )];
    }

    // TODO(files-hunks): upgrade details rows into selectable hunk models for partial staging.
    let lines = state.details.files_diff.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| PanelLine::new(format!("  {line}"), classify_diff_row_role(line)))
        .take(max_lines)
        .collect()
}

fn render_branch_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }

    if state.details.branch_log_target.is_none() {
        return vec![PanelLine::new(
            "  details(branches): no branch selected",
            RowRole::Muted,
        )];
    }

    if let Some(error) = &state.details.branch_log_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.work.details_pending && state.details.branch_log.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(branches): loading log graph",
            RowRole::Muted,
        )];
    }

    if state.details.branch_log.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(branches): no log graph for current selection",
            RowRole::Muted,
        )];
    }

    let lines = state.details.branch_log.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| ansi_branch_log_line(line, "  "))
        .take(max_lines)
        .collect()
}

fn render_commit_details_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if max_lines == 0 {
        return Vec::new();
    }

    if state.details.commit_diff_target.is_none() {
        return vec![PanelLine::new(
            "  details(commits): no commit selected",
            RowRole::Muted,
        )];
    }

    if let Some(error) = &state.details.commit_diff_error {
        return vec![PanelLine::new(format!("  error={error}"), RowRole::Error)];
    }

    if state.work.details_pending && state.details.commit_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(commits): loading diff",
            RowRole::Muted,
        )];
    }

    if state.details.commit_diff.trim().is_empty() {
        return vec![PanelLine::new(
            "  details(commits): no diff for current selection",
            RowRole::Muted,
        )];
    }

    let lines = state.details.commit_diff.lines().collect::<Vec<_>>();
    let start = details_scroll_start(lines.len(), state.details.scroll_offset, max_lines);
    lines
        .into_iter()
        .skip(start)
        .map(|line| PanelLine::new(format!("  {line}"), classify_diff_row_role(line)))
        .take(max_lines)
        .collect()
}

fn details_scroll_start(content_len: usize, requested_offset: usize, max_lines: usize) -> usize {
    requested_offset.min(content_len.saturating_sub(max_lines))
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

fn ansi_branch_log_line(line: &str, prefix: &str) -> PanelLine {
    let mut text = prefix.to_string();
    let mut spans = vec![PanelSpan {
        text: prefix.to_string(),
        style: Style::default(),
    }];
    let mut style = Style::default();
    let mut plain = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' && chars.peek() == Some(&'[') {
            chars.next();
            let mut code = String::new();
            for next in chars.by_ref() {
                if next == 'm' {
                    break;
                }
                code.push(next);
            }
            if !plain.is_empty() {
                text.push_str(&plain);
                spans.push(PanelSpan {
                    text: std::mem::take(&mut plain),
                    style,
                });
            }
            style = apply_sgr_codes(style, &code);
        } else {
            plain.push(ch);
        }
    }

    if !plain.is_empty() {
        text.push_str(&plain);
        spans.push(PanelSpan { text: plain, style });
    }

    PanelLine::new(text, RowRole::Normal).styled_spans(spans)
}

fn apply_sgr_codes(mut style: Style, code: &str) -> Style {
    let parts = if code.is_empty() {
        vec![0]
    } else {
        code.split(';')
            .filter_map(|part| part.parse::<u8>().ok())
            .collect::<Vec<_>>()
    };
    let mut index = 0;
    while index < parts.len() {
        match parts[index] {
            0 => style = Style::default(),
            1 => style = style.add_modifier(Modifier::BOLD),
            22 => style = style.remove_modifier(Modifier::BOLD),
            30..=37 => style = style.fg(ansi_color(parts[index] - 30, false)),
            39 => style = style.fg(Color::Reset),
            90..=97 => style = style.fg(ansi_color(parts[index] - 90, true)),
            38 if parts.get(index + 1) == Some(&5) => {
                if let Some(color) = parts.get(index + 2) {
                    style = style.fg(Color::Indexed(*color));
                    index += 2;
                }
            }
            38 if parts.get(index + 1) == Some(&2) => {
                if let (Some(red), Some(green), Some(blue)) = (
                    parts.get(index + 2),
                    parts.get(index + 3),
                    parts.get(index + 4),
                ) {
                    style = style.fg(Color::Rgb(*red, *green, *blue));
                    index += 4;
                }
            }
            _ => {}
        }
        index += 1;
    }
    style
}

fn ansi_color(index: u8, bright: bool) -> Color {
    match (index, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::Reset,
    }
}

pub(crate) fn shortcuts_for_state(state: &AppState) -> String {
    if let Some(editor) = &state.editor.kind {
        return match editor {
            ratagit_core::EditorKind::Commit { .. } => {
                "commit editor: Tab field | arrows/Home/End cursor | Ctrl+J newline | Enter confirm | Esc cancel"
                    .to_string()
            }
            ratagit_core::EditorKind::Stash { .. } => {
                "stash editor: arrows/Home/End cursor | Enter confirm | Esc cancel".to_string()
            }
        };
    }

    if state.branches.create.active {
        return "branch name: arrows/Home/End cursor | Enter create | Esc cancel".to_string();
    }

    if state.branches.delete_menu.active {
        return "delete branch: j/k select | Enter delete | Esc cancel".to_string();
    }

    if state.branches.force_delete_confirm.active {
        return "force delete branch: Enter force delete | Esc cancel".to_string();
    }

    if state.branches.rebase_menu.active {
        return "rebase: j/k select | Enter rebase | Esc cancel".to_string();
    }

    if state.branches.auto_stash_confirm.active {
        return "auto stash: Enter confirm | Esc cancel".to_string();
    }

    if state.reset_menu.active {
        return "reset: j/k select | Enter confirm | Esc cancel".to_string();
    }

    if state.discard_confirm.active {
        return "discard: Enter confirm | Esc cancel".to_string();
    }

    if state.focus == PanelFocus::Files && state.files.mode == FileInputMode::SearchInput {
        return format!("search: {}", state.files.search_query);
    }
    match state.focus {
        PanelFocus::Files => {
            "keys(files): space stage/unstage | d discard | c commit | s stash(all|selected) | D reset | v multi | enter expand | / search".to_string()
        }
        PanelFocus::Branches => {
            "keys(branches): space checkout | n new | d delete | r rebase".to_string()
        }
        PanelFocus::Commits => {
            "keys(commits): s squash | f fixup | r reword | d delete | space detach | v multi | c commit"
                .to_string()
        }
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
    render_indexed_entries_window_with(
        items,
        selected,
        scroll_direction,
        scroll_direction_origin,
        max_lines,
        |index, item| {
            PanelLine::new(format_item(item), item_role(item)).selected(index == selected)
        },
    )
}

fn render_indexed_entries_window_with<T>(
    items: &[T],
    selected: usize,
    scroll_direction: Option<ScrollDirection>,
    scroll_direction_origin: usize,
    max_lines: usize,
    render_item: impl Fn(usize, &T) -> PanelLine,
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
        .map(|(index, item)| render_item(index, item))
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
    let graph = fixed_width(commit_graph(entry), 1);
    let hash = fixed_width(&entry.id, 7);
    let author = fixed_width(&author_initials(&entry.author_name), 2);
    format!(
        "{}  {}  {}  {}",
        graph,
        hash,
        author,
        commit_message_summary(entry)
    )
}

fn commit_entry_spans(entry: &CommitEntry) -> Vec<PanelSpan> {
    let graph = fixed_width(commit_graph(entry), 1);
    let hash = fixed_width(&entry.id, 7);
    let author = fixed_width(&author_initials(&entry.author_name), 2);
    vec![
        PanelSpan {
            text: graph,
            style: Style::default().fg(Color::DarkGray),
        },
        PanelSpan {
            text: "  ".to_string(),
            style: Style::default(),
        },
        PanelSpan {
            text: hash,
            style: commit_hash_style(entry.hash_status),
        },
        PanelSpan {
            text: "  ".to_string(),
            style: Style::default(),
        },
        PanelSpan {
            text: author,
            style: author_style(&entry.author_name),
        },
        PanelSpan {
            text: "  ".to_string(),
            style: Style::default(),
        },
        PanelSpan {
            text: commit_message_summary(entry),
            style: Style::default(),
        },
    ]
}

fn fixed_width(text: &str, width: usize) -> String {
    let mut output = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let ch_width = UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + ch_width > width {
            break;
        }
        output.push(ch);
        used += ch_width;
    }
    format!("{}{}", output, " ".repeat(width.saturating_sub(used)))
}

fn commit_graph(entry: &CommitEntry) -> &str {
    if entry.graph.is_empty() {
        "●"
    } else {
        &entry.graph
    }
}

fn commit_message_summary(entry: &CommitEntry) -> String {
    if entry.summary.is_empty() {
        entry
            .message
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        entry.summary.clone()
    }
}

fn commit_hash_style(status: CommitHashStatus) -> Style {
    match status {
        CommitHashStatus::MergedToMain => Style::default().fg(Color::Green),
        CommitHashStatus::Pushed => Style::default().fg(Color::Yellow),
        CommitHashStatus::Unpushed => Style::default().fg(Color::Red),
    }
}

fn author_style(author_name: &str) -> Style {
    const PALETTE: [Color; 8] = [
        Color::Cyan,
        Color::Magenta,
        Color::Blue,
        Color::LightGreen,
        Color::LightBlue,
        Color::LightMagenta,
        Color::LightCyan,
        Color::White,
    ];
    let hash = author_name.bytes().fold(0usize, |accumulator, byte| {
        accumulator.wrapping_mul(31).wrapping_add(byte as usize)
    });
    Style::default()
        .fg(PALETTE[hash % PALETTE.len()])
        .add_modifier(Modifier::BOLD)
}

fn author_initials(author_name: &str) -> String {
    let words = author_name
        .split(|ch: char| !ch.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>();
    let mut chars = if words.len() >= 2 {
        words
            .iter()
            .filter_map(|word| word.chars().next())
            .take(2)
            .collect::<Vec<_>>()
    } else {
        author_name
            .chars()
            .filter(|ch| ch.is_alphanumeric())
            .take(2)
            .collect::<Vec<_>>()
    };
    while chars.len() < 2 {
        chars.push('?');
    }
    chars
        .into_iter()
        .flat_map(char::to_uppercase)
        .take(2)
        .collect()
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
