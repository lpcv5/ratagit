use std::collections::BTreeSet;

use crate::scroll::{move_selected_index, move_selected_index_with_scroll_offset};
use crate::{CommitEntry, CommitInputMode, CommitsUiState};

pub fn commit_key(entry: &CommitEntry) -> String {
    if entry.full_id.is_empty() {
        entry.id.clone()
    } else {
        entry.full_id.clone()
    }
}

pub fn selected_commit(items: &[CommitEntry], state: &CommitsUiState) -> Option<CommitEntry> {
    items.get(state.selected).cloned()
}

pub fn selected_commit_ids(items: &[CommitEntry], state: &CommitsUiState) -> Vec<String> {
    if state.mode == CommitInputMode::MultiSelect && !state.selected_rows.is_empty() {
        return items
            .iter()
            .filter(|entry| state.selected_rows.contains(&commit_key(entry)))
            .map(commit_key)
            .collect();
    }
    selected_commit(items, state)
        .map(|entry| vec![commit_key(&entry)])
        .unwrap_or_default()
}

pub fn selected_commits(items: &[CommitEntry], state: &CommitsUiState) -> Vec<CommitEntry> {
    if state.mode == CommitInputMode::MultiSelect && !state.selected_rows.is_empty() {
        return items
            .iter()
            .filter(|entry| state.selected_rows.contains(&commit_key(entry)))
            .cloned()
            .collect();
    }
    selected_commit(items, state).into_iter().collect()
}

pub fn enter_multi_select(items: &[CommitEntry], state: &mut CommitsUiState) {
    state.mode = CommitInputMode::MultiSelect;
    state.selection_anchor = selected_commit(items, state).map(|entry| commit_key(&entry));
    refresh_multi_select_range(items, state);
}

pub fn leave_multi_select(state: &mut CommitsUiState) {
    state.mode = CommitInputMode::Normal;
    state.selection_anchor = None;
    state.selected_rows.clear();
}

pub fn toggle_multi_select(items: &[CommitEntry], state: &mut CommitsUiState) {
    if state.mode == CommitInputMode::MultiSelect {
        leave_multi_select(state);
    } else {
        enter_multi_select(items, state);
    }
}

pub fn move_selected(items: &[CommitEntry], state: &mut CommitsUiState, move_up: bool) {
    let len = items.len();
    move_selected_index(&mut state.selected, len, move_up);
    if state.mode == CommitInputMode::MultiSelect {
        refresh_multi_select_range(items, state);
    }
}

pub fn move_selected_in_viewport(
    items: &[CommitEntry],
    state: &mut CommitsUiState,
    move_up: bool,
    visible_lines: usize,
) {
    let len = items.len();
    move_selected_index_with_scroll_offset(
        &mut state.selected,
        &mut state.scroll_offset,
        len,
        move_up,
        visible_lines,
    );
    if state.mode == CommitInputMode::MultiSelect {
        refresh_multi_select_range(items, state);
    }
}

pub fn reconcile_after_items_changed(items: &[CommitEntry], state: &mut CommitsUiState) {
    let valid_rows = items.iter().map(commit_key).collect::<BTreeSet<_>>();
    state.selected_rows.retain(|key| valid_rows.contains(key));
    clamp_selected(items, state);
    if state.mode == CommitInputMode::MultiSelect {
        ensure_valid_selection_anchor(items, state);
        refresh_multi_select_range(items, state);
    }
}

pub fn reconcile_after_items_appended(items: &[CommitEntry], state: &mut CommitsUiState) {
    let scroll_offset = state.scroll_offset;
    reconcile_after_items_changed(items, state);
    if !items.is_empty() {
        state.scroll_offset = scroll_offset.min(items.len() - 1);
    }
}

pub fn clamp_selected(items: &[CommitEntry], state: &mut CommitsUiState) {
    state.selected = if items.is_empty() {
        0
    } else {
        state.selected.min(items.len() - 1)
    };
    state.scroll_offset = 0;
}

pub fn is_selected_for_batch(state: &CommitsUiState, entry: &CommitEntry) -> bool {
    state.mode == CommitInputMode::MultiSelect && state.selected_rows.contains(&commit_key(entry))
}

fn ensure_valid_selection_anchor(items: &[CommitEntry], state: &mut CommitsUiState) {
    let anchor_valid = state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| items.iter().any(|entry| commit_key(entry) == *anchor));
    if !anchor_valid {
        state.selection_anchor = selected_commit(items, state).map(|entry| commit_key(&entry));
    }
}

fn refresh_multi_select_range(items: &[CommitEntry], state: &mut CommitsUiState) {
    state.selected_rows.clear();
    let Some(anchor) = state.selection_anchor.clone() else {
        return;
    };
    let Some(anchor_index) = items.iter().position(|entry| commit_key(entry) == anchor) else {
        return;
    };
    if items.is_empty() {
        return;
    }
    let selected = state.selected.min(items.len() - 1);
    let (start, end) = if anchor_index <= selected {
        (anchor_index, selected)
    } else {
        (selected, anchor_index)
    };
    for entry in &items[start..=end] {
        state.selected_rows.insert(commit_key(entry));
    }
}
