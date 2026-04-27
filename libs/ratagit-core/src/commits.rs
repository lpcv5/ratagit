use std::collections::BTreeSet;

use crate::scroll::{move_selected_index, move_selected_index_with_scroll_offset};
use crate::{CommitEntry, CommitInputMode, CommitsPanelState};

pub fn commit_key(entry: &CommitEntry) -> String {
    if entry.full_id.is_empty() {
        entry.id.clone()
    } else {
        entry.full_id.clone()
    }
}

pub fn selected_commit(state: &CommitsPanelState) -> Option<CommitEntry> {
    state.items.get(state.selected).cloned()
}

pub fn selected_commit_ids(state: &CommitsPanelState) -> Vec<String> {
    if state.mode == CommitInputMode::MultiSelect && !state.selected_rows.is_empty() {
        return state
            .items
            .iter()
            .filter(|entry| state.selected_rows.contains(&commit_key(entry)))
            .map(commit_key)
            .collect();
    }
    selected_commit(state)
        .map(|entry| vec![commit_key(&entry)])
        .unwrap_or_default()
}

pub fn selected_commits(state: &CommitsPanelState) -> Vec<CommitEntry> {
    if state.mode == CommitInputMode::MultiSelect && !state.selected_rows.is_empty() {
        return state
            .items
            .iter()
            .filter(|entry| state.selected_rows.contains(&commit_key(entry)))
            .cloned()
            .collect();
    }
    selected_commit(state).into_iter().collect()
}

pub fn enter_multi_select(state: &mut CommitsPanelState) {
    state.mode = CommitInputMode::MultiSelect;
    state.selection_anchor = selected_commit(state).map(|entry| commit_key(&entry));
    refresh_multi_select_range(state);
}

pub fn leave_multi_select(state: &mut CommitsPanelState) {
    state.mode = CommitInputMode::Normal;
    state.selection_anchor = None;
    state.selected_rows.clear();
}

pub fn toggle_multi_select(state: &mut CommitsPanelState) {
    if state.mode == CommitInputMode::MultiSelect {
        leave_multi_select(state);
    } else {
        enter_multi_select(state);
    }
}

pub fn move_selected(state: &mut CommitsPanelState, move_up: bool) {
    let len = state.items.len();
    move_selected_index(&mut state.selected, len, move_up);
    if state.mode == CommitInputMode::MultiSelect {
        refresh_multi_select_range(state);
    }
}

pub fn move_selected_in_viewport(
    state: &mut CommitsPanelState,
    move_up: bool,
    visible_lines: usize,
) {
    let len = state.items.len();
    move_selected_index_with_scroll_offset(
        &mut state.selected,
        &mut state.scroll_offset,
        len,
        move_up,
        visible_lines,
    );
    if state.mode == CommitInputMode::MultiSelect {
        refresh_multi_select_range(state);
    }
}

pub fn reconcile_after_items_changed(state: &mut CommitsPanelState) {
    let valid_rows = state.items.iter().map(commit_key).collect::<BTreeSet<_>>();
    state.selected_rows.retain(|key| valid_rows.contains(key));
    clamp_selected(state);
    if state.mode == CommitInputMode::MultiSelect {
        ensure_valid_selection_anchor(state);
        refresh_multi_select_range(state);
    }
}

pub fn reconcile_after_items_appended(state: &mut CommitsPanelState) {
    let scroll_offset = state.scroll_offset;
    reconcile_after_items_changed(state);
    if !state.items.is_empty() {
        state.scroll_offset = scroll_offset.min(state.items.len() - 1);
    }
}

pub fn clamp_selected(state: &mut CommitsPanelState) {
    state.selected = if state.items.is_empty() {
        0
    } else {
        state.selected.min(state.items.len() - 1)
    };
    state.scroll_offset = 0;
}

pub fn is_selected_for_batch(state: &CommitsPanelState, entry: &CommitEntry) -> bool {
    state.mode == CommitInputMode::MultiSelect && state.selected_rows.contains(&commit_key(entry))
}

fn ensure_valid_selection_anchor(state: &mut CommitsPanelState) {
    let anchor_valid = state
        .selection_anchor
        .as_ref()
        .is_some_and(|anchor| state.items.iter().any(|entry| commit_key(entry) == *anchor));
    if !anchor_valid {
        state.selection_anchor = selected_commit(state).map(|entry| commit_key(&entry));
    }
}

fn refresh_multi_select_range(state: &mut CommitsPanelState) {
    state.selected_rows.clear();
    let Some(anchor) = state.selection_anchor.clone() else {
        return;
    };
    let Some(anchor_index) = state
        .items
        .iter()
        .position(|entry| commit_key(entry) == anchor)
    else {
        return;
    };
    if state.items.is_empty() {
        return;
    }
    let selected = state.selected.min(state.items.len() - 1);
    let (start, end) = if anchor_index <= selected {
        (anchor_index, selected)
    } else {
        (selected, anchor_index)
    };
    for entry in &state.items[start..=end] {
        state.selected_rows.insert(commit_key(entry));
    }
}
