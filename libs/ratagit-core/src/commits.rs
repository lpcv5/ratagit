use crate::state::{
    clamp_linear_selection, ensure_linear_selection_anchor, enter_linear_range_select,
    leave_linear_range_select, linear_key_at_selection, linear_key_is_selected,
    move_linear_selection, move_linear_selection_in_viewport, reconcile_linear_valid_keys,
    refresh_linear_range,
};
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
    let keys = commit_keys(items);
    let selected_key = linear_key_at_selection(&state.selection, &keys);
    enter_linear_range_select(&mut state.selection, selected_key);
    refresh_linear_range(&mut state.selection, &keys);
}

pub fn leave_multi_select(state: &mut CommitsUiState) {
    state.mode = CommitInputMode::Normal;
    leave_linear_range_select(&mut state.selection);
}

pub fn toggle_multi_select(items: &[CommitEntry], state: &mut CommitsUiState) {
    if state.mode == CommitInputMode::MultiSelect {
        leave_multi_select(state);
    } else {
        enter_multi_select(items, state);
    }
}

pub fn move_selected(items: &[CommitEntry], state: &mut CommitsUiState, move_up: bool) {
    move_linear_selection(&mut state.selection, items.len(), move_up);
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
    move_linear_selection_in_viewport(&mut state.selection, items.len(), move_up, visible_lines);
    if state.mode == CommitInputMode::MultiSelect {
        refresh_multi_select_range(items, state);
    }
}

pub fn reconcile_after_items_changed(items: &[CommitEntry], state: &mut CommitsUiState) {
    let keys = commit_keys(items);
    reconcile_linear_valid_keys(&mut state.selection, &keys);
    clamp_selected(items, state);
    if state.mode == CommitInputMode::MultiSelect {
        ensure_linear_selection_anchor(&mut state.selection, &keys);
        refresh_linear_range(&mut state.selection, &keys);
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
    clamp_linear_selection(&mut state.selection, items.len());
}

pub fn is_selected_for_batch(state: &CommitsUiState, entry: &CommitEntry) -> bool {
    state.mode == CommitInputMode::MultiSelect
        && linear_key_is_selected(&state.selection, &commit_key(entry))
}

fn refresh_multi_select_range(items: &[CommitEntry], state: &mut CommitsUiState) {
    refresh_linear_range(&mut state.selection, &commit_keys(items));
}

fn commit_keys(items: &[CommitEntry]) -> Vec<String> {
    items.iter().map(commit_key).collect()
}
