use std::collections::HashSet;

use ratagit_core::{
    AppContext, BranchesSubview, CommitEntry, CommitsUiState, FileTreeRow, PanelFocus, SearchScope,
    branch_is_selected_for_batch, commit_file_tree_rows_for_read, commit_is_selected_for_batch,
    commit_key, file_tree_rows, file_tree_rows_for_read,
};
use ratatui::style::Style;

use super::panel_format::{
    branch_entry_role, commit_entry_spans, file_tree_row_role, file_tree_row_spans,
    format_branch_entry, format_stash_entry,
};
use super::panel_scroll::render_indexed_entries_window_with;
use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::{PanelLabel, RowRole, panel_label, row_style};

pub(crate) fn panel_title(state: &AppContext, panel: PanelFocus) -> &'static str {
    if panel == PanelFocus::Branches && state.ui.branches.subview == BranchesSubview::Commits {
        "[2]  Branch Commits"
    } else if panel == PanelFocus::Branches
        && state.ui.branches.subview == BranchesSubview::CommitFiles
    {
        "[2]  Branch Commit Files"
    } else if panel == PanelFocus::Commits && state.ui.commits.files.active {
        "[3]  Commit Files"
    } else {
        match panel {
            PanelFocus::Files => "[1] 󰈙 Files",
            PanelFocus::Branches => "[2]  Branches",
            PanelFocus::Commits => "[3]  Commits",
            PanelFocus::Stash => "[4]  Stash",
            PanelFocus::Details => "[5]  Details",
            PanelFocus::Log => "[6] 󰌱 Log",
        }
    }
}

pub(crate) fn panel_title_label(state: &AppContext, panel: PanelFocus) -> PanelLabel {
    if panel == PanelFocus::Branches && state.ui.branches.subview == BranchesSubview::Commits {
        PanelLabel {
            badge: "2",
            body: " Branch Commits",
        }
    } else if panel == PanelFocus::Branches
        && state.ui.branches.subview == BranchesSubview::CommitFiles
    {
        PanelLabel {
            badge: "2",
            body: " Branch Commit Files",
        }
    } else if panel == PanelFocus::Commits && state.ui.commits.files.active {
        PanelLabel {
            badge: "3",
            body: " Commit Files",
        }
    } else {
        panel_label(panel)
    }
}

pub(crate) fn left_panel_content_len(state: &AppContext, panel: PanelFocus) -> usize {
    match panel {
        PanelFocus::Files => {
            if file_tree_rows(&state.ui.files).is_empty() && !state.repo.files.items.is_empty() {
                file_tree_rows_for_read(&state.repo.files.items, &state.ui.files).len()
            } else {
                file_tree_rows(&state.ui.files).len()
            }
        }
        PanelFocus::Commits if state.ui.commits.files.active => state.repo.commits.items.len(),
        PanelFocus::Branches => match state.ui.branches.subview {
            BranchesSubview::List => state.repo.branches.items.len(),
            BranchesSubview::Commits | BranchesSubview::CommitFiles => {
                state.repo.branches.items.len()
            }
        },
        PanelFocus::Commits => state.repo.commits.items.len(),
        PanelFocus::Stash => state.repo.stash.items.len(),
        PanelFocus::Details | PanelFocus::Log => 0,
    }
}

pub(crate) fn render_files_lines(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    let rows = file_tree_rows_for_read(&state.repo.files.items, &state.ui.files);
    render_file_tree_lines(
        state,
        rows.as_ref(),
        state.ui.files.selected,
        state.ui.files.scroll_offset,
        max_lines,
        SearchScope::Files,
    )
}

pub(crate) fn render_branches_lines(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    if state.ui.branches.subview == BranchesSubview::CommitFiles {
        return render_branch_commit_file_lines(state, max_lines);
    }
    if state.ui.branches.subview == BranchesSubview::Commits {
        return render_commit_list_lines(
            state,
            &state.repo.branches.commits,
            &state.ui.branches.commits,
            max_lines,
        );
    }
    let matches = SearchMatches::new(search_matches_for(state, SearchScope::Branches));
    render_indexed_entries_window_with(
        &state.repo.branches.items,
        state.ui.branches.selected,
        state.ui.branches.scroll_offset,
        max_lines,
        |index, branch| {
            let role = if branch_is_selected_for_batch(&state.ui.branches, &branch.name) {
                RowRole::BatchSelected
            } else {
                branch_entry_role(branch)
            };
            let line = PanelLine::new(format_branch_entry(branch), role)
                .selected(index == state.ui.branches.selected);
            if matches.contains(&branch.name) {
                highlight_search_query(line, state, SearchScope::Branches)
            } else {
                line
            }
        },
    )
}

pub(crate) fn render_commits_lines(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    if state.ui.commits.files.active {
        return render_commit_file_lines(state, max_lines);
    }
    render_commit_list_lines(
        state,
        &state.repo.commits.items,
        &state.ui.commits,
        max_lines,
    )
}

fn render_commit_list_lines(
    state: &AppContext,
    items: &[CommitEntry],
    ui: &CommitsUiState,
    max_lines: usize,
) -> Vec<PanelLine> {
    let matches = SearchMatches::new(search_matches_for(state, SearchScope::Commits));
    render_indexed_entries_window_with(
        items,
        ui.selected,
        ui.scroll_offset,
        max_lines,
        |index, entry| {
            let role = if commit_is_selected_for_batch(ui, entry) {
                RowRole::BatchSelected
            } else {
                RowRole::Normal
            };
            let line = PanelLine::from_spans(commit_entry_spans(entry), role)
                .selected(index == ui.selected);
            if matches.contains(&commit_key(entry)) {
                highlight_search_query(line, state, SearchScope::Commits)
            } else {
                line
            }
        },
    )
}

pub(crate) fn render_stash_lines(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    let matches = SearchMatches::new(search_matches_for(state, SearchScope::Stash));
    render_indexed_entries_window_with(
        &state.repo.stash.items,
        state.ui.stash.selected,
        state.ui.stash.scroll_offset,
        max_lines,
        |index, stash| {
            let line = PanelLine::new(format_stash_entry(stash), RowRole::Normal)
                .selected(index == state.ui.stash.selected);
            if matches.contains(&stash.id) {
                highlight_search_query(line, state, SearchScope::Stash)
            } else {
                line
            }
        },
    )
}

fn render_commit_file_lines(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    let rows =
        commit_file_tree_rows_for_read(&state.repo.commits.files.items, &state.ui.commits.files);
    render_file_tree_lines(
        state,
        rows.as_ref(),
        state.ui.commits.files.selected,
        state.ui.commits.files.scroll_offset,
        max_lines,
        SearchScope::CommitFiles,
    )
}

fn render_branch_commit_file_lines(state: &AppContext, max_lines: usize) -> Vec<PanelLine> {
    let rows = commit_file_tree_rows_for_read(
        &state.repo.branches.commit_files.items,
        &state.ui.branches.commit_files,
    );
    render_file_tree_lines(
        state,
        rows.as_ref(),
        state.ui.branches.commit_files.selected,
        state.ui.branches.commit_files.scroll_offset,
        max_lines,
        SearchScope::CommitFiles,
    )
}

fn render_file_tree_lines(
    state: &AppContext,
    rows: &[FileTreeRow],
    selected: usize,
    scroll_offset: usize,
    max_lines: usize,
    search_scope: SearchScope,
) -> Vec<PanelLine> {
    let matches = SearchMatches::new(search_matches_for(state, search_scope));
    render_indexed_entries_window_with(rows, selected, scroll_offset, max_lines, |index, row| {
        let matched = matches.contains(&row.path);
        let mut visible_row;
        let row = if row.matched == matched {
            row
        } else {
            visible_row = row.clone();
            visible_row.matched = matched;
            &visible_row
        };
        let line = PanelLine::from_spans(file_tree_row_spans(row), file_tree_row_role(row))
            .selected(index == selected);
        if matched || line_contains_search_query(&line, state, search_scope) {
            highlight_search_query(line, state, search_scope)
        } else {
            line
        }
    })
}

fn search_matches_for(state: &AppContext, scope: SearchScope) -> Option<&[String]> {
    if state.ui.search.has_query_for(scope) {
        Some(&state.ui.search.matches)
    } else {
        None
    }
}

struct SearchMatches<'a> {
    matches: Option<&'a [String]>,
    set: Option<HashSet<&'a str>>,
}

impl<'a> SearchMatches<'a> {
    fn new(matches: Option<&'a [String]>) -> Self {
        let set = matches.and_then(|matches| {
            (matches.len() > 32).then(|| matches.iter().map(String::as_str).collect())
        });
        Self { matches, set }
    }

    fn contains(&self, key: &str) -> bool {
        if let Some(set) = &self.set {
            return set.contains(key);
        }
        self.matches
            .is_some_and(|matches| matches.iter().any(|matched| matched == key))
    }
}

fn highlight_search_query(
    mut line: PanelLine,
    state: &AppContext,
    scope: SearchScope,
) -> PanelLine {
    if !state.ui.search.has_query_for(scope) {
        return line;
    }
    let query = state.ui.search.query.as_str();
    if query.is_empty() {
        return line;
    }
    let mut highlighted = Vec::new();
    for span in std::mem::take(&mut line.spans) {
        let (mut split, _) = highlight_text_segments(&span.text, span.style, query);
        highlighted.append(&mut split);
    }
    line.spans = highlighted;
    line
}

fn line_contains_search_query(line: &PanelLine, state: &AppContext, scope: SearchScope) -> bool {
    if !state.ui.search.has_query_for(scope) || state.ui.search.query.is_empty() {
        return false;
    }
    let query = state.ui.search.query.to_lowercase();
    line.spans
        .iter()
        .any(|span| span.text.to_lowercase().contains(&query))
}

fn highlight_text_segments(text: &str, base_style: Style, query: &str) -> (Vec<PanelSpan>, bool) {
    let ranges = case_insensitive_match_ranges(text, query);
    if ranges.is_empty() {
        return (
            vec![PanelSpan {
                text: text.to_string(),
                style: base_style,
            }],
            false,
        );
    }

    let mut spans = Vec::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if cursor < start {
            spans.push(PanelSpan {
                text: text[cursor..start].to_string(),
                style: base_style,
            });
        }
        spans.push(PanelSpan {
            text: text[start..end].to_string(),
            style: row_style(RowRole::SearchMatch),
        });
        cursor = end;
    }
    if cursor < text.len() {
        spans.push(PanelSpan {
            text: text[cursor..].to_string(),
            style: base_style,
        });
    }
    (spans, true)
}

fn case_insensitive_match_ranges(text: &str, query: &str) -> Vec<(usize, usize)> {
    let query_lower = query.to_lowercase();
    if query_lower.is_empty() {
        return Vec::new();
    }
    let chars = text.char_indices().collect::<Vec<_>>();
    let mut ranges = Vec::new();
    let mut start_index = 0;
    while start_index < chars.len() {
        let start = chars[start_index].0;
        let mut found = None;
        for end_index in start_index + 1..=chars.len() {
            let end = chars
                .get(end_index)
                .map(|(index, _)| *index)
                .unwrap_or(text.len());
            let candidate = text[start..end].to_lowercase();
            if candidate == query_lower {
                found = Some((end_index, end));
                break;
            }
            if candidate.len() >= query_lower.len() && !query_lower.starts_with(&candidate) {
                break;
            }
        }
        if let Some((next_index, end)) = found {
            ranges.push((start, end));
            start_index = next_index;
        } else {
            start_index += 1;
        }
    }
    ranges
}
