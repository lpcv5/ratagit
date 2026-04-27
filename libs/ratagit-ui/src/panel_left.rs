use ratagit_core::{
    AppState, FileTreeRow, PanelFocus, SearchScope, commit_file_tree_rows,
    commit_file_tree_rows_for_read, commit_is_selected_for_batch, commit_key, file_tree_rows,
    file_tree_rows_for_read,
};
use ratatui::style::Style;

use super::panel_format::{
    branch_entry_role, commit_entry_spans, file_tree_row_role, file_tree_row_spans,
    format_branch_entry, format_commit_entry, format_file_tree_row, format_stash_entry,
};
use super::panel_scroll::render_indexed_entries_window_with;
use super::panel_types::{PanelLine, PanelSpan};
use crate::theme::{RowRole, panel_label, row_style};

pub(crate) fn panel_title(state: &AppState, panel: PanelFocus) -> &'static str {
    if panel == PanelFocus::Commits && state.commits.files.active {
        "[3]  Commit Files"
    } else {
        panel_label(panel)
    }
}

pub(crate) fn left_panel_content_len(state: &AppState, panel: PanelFocus) -> usize {
    match panel {
        PanelFocus::Files => {
            if file_tree_rows(&state.files).is_empty() && !state.files.items.is_empty() {
                file_tree_rows_for_read(&state.files).len()
            } else {
                file_tree_rows(&state.files).len()
            }
        }
        PanelFocus::Commits if state.commits.files.active => {
            if state.commits.files.loading && state.commits.files.items.is_empty() {
                state.commits.items.len()
            } else if commit_file_tree_rows(&state.commits.files).is_empty()
                && !state.commits.files.items.is_empty()
            {
                commit_file_tree_rows_for_read(&state.commits.files).len()
            } else {
                commit_file_tree_rows(&state.commits.files).len()
            }
        }
        PanelFocus::Branches => state.branches.items.len(),
        PanelFocus::Commits => state.commits.items.len(),
        PanelFocus::Stash => state.stash.items.len(),
        PanelFocus::Details | PanelFocus::Log => 0,
    }
}

pub(crate) fn render_files_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let mut rows = file_tree_rows_for_read(&state.files);
    if state.search.has_query_for(SearchScope::Files) {
        apply_tree_search_matches(rows.to_mut(), state, SearchScope::Files);
    }
    render_indexed_entries_window_with(
        rows.as_ref(),
        state.files.selected,
        state.files.scroll_direction,
        state.files.scroll_direction_origin,
        max_lines,
        |index, row| {
            PanelLine::new(format_file_tree_row(row), file_tree_row_role(row))
                .selected(index == state.files.selected)
                .styled_spans(file_tree_row_spans(row))
        },
    )
    .into_iter()
    .map(|line| highlight_search_query(line, state, SearchScope::Files))
    .collect()
}

pub(crate) fn render_branches_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let matches = search_matches_for(state, SearchScope::Branches);
    render_indexed_entries_window_with(
        &state.branches.items,
        state.branches.selected,
        None,
        state.branches.selected,
        max_lines,
        |index, branch| {
            let line = PanelLine::new(format_branch_entry(branch), branch_entry_role(branch))
                .selected(index == state.branches.selected);
            if search_matches_contains(matches, &branch.name) {
                highlight_search_query(line, state, SearchScope::Branches)
            } else {
                line
            }
        },
    )
}

pub(crate) fn render_commits_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    if state.commits.files.active {
        return render_commit_file_lines(state, max_lines);
    }
    let matches = search_matches_for(state, SearchScope::Commits);
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
            let line = PanelLine::new(format_commit_entry(entry), role)
                .selected(index == state.commits.selected)
                .styled_spans(commit_entry_spans(entry));
            if search_matches_contains(matches, &commit_key(entry)) {
                highlight_search_query(line, state, SearchScope::Commits)
            } else {
                line
            }
        },
    )
}

pub(crate) fn render_stash_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let matches = search_matches_for(state, SearchScope::Stash);
    render_indexed_entries_window_with(
        &state.stash.items,
        state.stash.selected,
        None,
        state.stash.selected,
        max_lines,
        |index, stash| {
            let line = PanelLine::new(format_stash_entry(stash), RowRole::Normal)
                .selected(index == state.stash.selected);
            if search_matches_contains(matches, &stash.id) {
                highlight_search_query(line, state, SearchScope::Stash)
            } else {
                line
            }
        },
    )
}

fn render_commit_file_lines(state: &AppState, max_lines: usize) -> Vec<PanelLine> {
    let mut rows = commit_file_tree_rows_for_read(&state.commits.files);
    if state.search.has_query_for(SearchScope::CommitFiles) {
        apply_tree_search_matches(rows.to_mut(), state, SearchScope::CommitFiles);
    }
    render_indexed_entries_window_with(
        rows.as_ref(),
        state.commits.files.selected,
        state.commits.files.scroll_direction,
        state.commits.files.scroll_direction_origin,
        max_lines,
        |index, row| {
            PanelLine::new(format_file_tree_row(row), file_tree_row_role(row))
                .selected(index == state.commits.files.selected)
                .styled_spans(file_tree_row_spans(row))
        },
    )
    .into_iter()
    .map(|line| highlight_search_query(line, state, SearchScope::CommitFiles))
    .collect()
}

fn apply_tree_search_matches(rows: &mut [FileTreeRow], state: &AppState, scope: SearchScope) {
    let Some(matches) = search_matches_for(state, scope) else {
        return;
    };
    for row in rows {
        row.matched = matches.iter().any(|matched| matched == &row.path);
    }
}

fn search_matches_for(state: &AppState, scope: SearchScope) -> Option<&[String]> {
    if state.search.has_query_for(scope) {
        Some(&state.search.matches)
    } else {
        None
    }
}

fn search_matches_contains(matches: Option<&[String]>, key: &str) -> bool {
    matches.is_some_and(|matches| matches.iter().any(|matched| matched == key))
}

fn highlight_search_query(mut line: PanelLine, state: &AppState, scope: SearchScope) -> PanelLine {
    if !state.search.has_query_for(scope) {
        return line;
    }
    let query = state.search.query.as_str();
    if query.is_empty() {
        return line;
    }
    match line.spans.take() {
        Some(spans) => {
            let mut highlighted = Vec::new();
            for span in spans {
                let (mut split, _) = highlight_text_segments(&span.text, span.style, query);
                highlighted.append(&mut split);
            }
            line.spans = Some(highlighted);
        }
        None => {
            let (spans, changed) = highlight_text_segments(&line.text, Style::default(), query);
            if changed {
                line.spans = Some(spans);
            }
        }
    }
    line
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
