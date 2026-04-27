use crate::{
    AppState, BranchInputMode, CommitEntry, CommitInputMode, FileInputMode, SearchScope, branches,
    commit_file_tree_rows_for_read, commit_key, file_tree_rows_for_read,
    leave_commit_files_multi_select, leave_commit_multi_select, leave_multi_select,
    select_commit_file_tree_path, select_file_tree_path,
};

pub(crate) fn start_search(state: &mut AppState) {
    let Some(scope) = state.active_search_scope() else {
        return;
    };
    if scope == SearchScope::Files && state.files.mode == FileInputMode::MultiSelect {
        leave_multi_select(&mut state.files);
    }
    if scope == SearchScope::Branches && state.branches.mode == BranchInputMode::MultiSelect {
        branches::leave_multi_select(&mut state.branches);
    }
    if scope == SearchScope::Commits && state.commits.mode == CommitInputMode::MultiSelect {
        leave_commit_multi_select(&mut state.commits);
    }
    if scope == SearchScope::CommitFiles && state.commits.files.mode == FileInputMode::MultiSelect {
        leave_commit_files_multi_select(&mut state.commits.files);
    }
    state.search.active = true;
    state.search.scope = Some(scope);
    state.search.query.clear();
    state.search.matches.clear();
    state.search.current_match = None;
}

pub(crate) fn input_search_char(state: &mut AppState, ch: char) {
    if !search_input_is_current(state) {
        return;
    }
    state.search.query.push(ch);
    recompute_search_matches(state);
}

pub(crate) fn backspace_search(state: &mut AppState) {
    if !search_input_is_current(state) {
        return;
    }
    state.search.query.pop();
    recompute_search_matches(state);
}

pub(crate) fn confirm_search(state: &mut AppState) -> bool {
    if !search_input_is_current(state) {
        return false;
    }
    state.search.active = false;
    recompute_search_matches(state);
    if state.search.matches.is_empty() {
        return false;
    }
    state.search.current_match = Some(0);
    select_current_search_match(state)
}

pub(crate) fn cancel_search(state: &mut AppState) {
    if state.search.scope == state.active_search_scope()
        && (state.search.active || !state.search.query.is_empty())
    {
        state.search.clear();
    }
}

pub(crate) fn jump_search_match(state: &mut AppState, previous: bool) -> bool {
    if state.search.scope != state.active_search_scope() || state.search.query.is_empty() {
        return false;
    }
    recompute_search_matches(state);
    if state.search.matches.is_empty() {
        return false;
    }
    let len = state.search.matches.len();
    let next = match (state.search.current_match, previous) {
        (Some(index), true) => (index + len - 1) % len,
        (Some(index), false) => (index + 1) % len,
        (None, _) => 0,
    };
    state.search.current_match = Some(next);
    select_current_search_match(state)
}

pub(crate) fn clear_search_if_incompatible(state: &mut AppState) {
    if state.search.scope.is_some() && state.search.scope != state.active_search_scope() {
        state.search.clear();
    }
}

pub(crate) fn recompute_search_matches(state: &mut AppState) {
    if state.search.query.is_empty() {
        state.search.matches.clear();
        state.search.current_match = None;
        return;
    }
    let Some(scope) = state.search.scope else {
        state.search.matches.clear();
        state.search.current_match = None;
        return;
    };
    let query = state.search.query.to_lowercase();
    state.search.matches = match scope {
        SearchScope::Files => {
            let rows = file_tree_rows_for_read(&state.files);
            collect_matches(
                rows.iter(),
                &query,
                |row| row.path.clone(),
                |row| row.path.clone(),
            )
        }
        SearchScope::Branches => collect_matches(
            state.branches.items.iter(),
            &query,
            |branch| branch.name.clone(),
            |branch| branch.name.clone(),
        ),
        SearchScope::Commits => collect_matches(
            state.commits.items.iter(),
            &query,
            commit_key,
            commit_search_text,
        ),
        SearchScope::Stash => collect_matches(
            state.stash.items.iter(),
            &query,
            |stash| stash.id.clone(),
            |stash| format!("{} {}", stash.id, stash.summary),
        ),
        SearchScope::CommitFiles => {
            let rows = commit_file_tree_rows_for_read(&state.commits.files);
            collect_matches(
                rows.iter(),
                &query,
                |row| row.path.clone(),
                |row| row.path.clone(),
            )
        }
    };
    if state.search.matches.is_empty() {
        state.search.current_match = None;
    } else {
        state.search.current_match = Some(
            state
                .search
                .current_match
                .unwrap_or(0)
                .min(state.search.matches.len() - 1),
        );
    }
}

fn collect_matches<'a, T: 'a>(
    items: impl Iterator<Item = &'a T>,
    query: &str,
    key: impl Fn(&T) -> String,
    text: impl Fn(&T) -> String,
) -> Vec<String> {
    items
        .filter(|item| text(item).to_lowercase().contains(query))
        .map(key)
        .collect()
}

fn search_input_is_current(state: &AppState) -> bool {
    state.search.active && state.search.scope == state.active_search_scope()
}

fn select_current_search_match(state: &mut AppState) -> bool {
    let Some(scope) = state.search.scope else {
        return false;
    };
    let Some(index) = state.search.current_match else {
        return false;
    };
    let Some(key) = state.search.matches.get(index).cloned() else {
        return false;
    };
    match scope {
        SearchScope::Files => select_file_tree_path(&mut state.files, &key),
        SearchScope::Branches => select_index_by_key(
            &mut state.branches.selected,
            state
                .branches
                .items
                .iter()
                .map(|branch| branch.name.clone()),
            &key,
        ),
        SearchScope::Commits => select_index_by_key(
            &mut state.commits.selected,
            state.commits.items.iter().map(commit_key),
            &key,
        ),
        SearchScope::Stash => select_index_by_key(
            &mut state.stash.selected,
            state.stash.items.iter().map(|stash| stash.id.clone()),
            &key,
        ),
        SearchScope::CommitFiles => select_commit_file_tree_path(&mut state.commits.files, &key),
    }
}

fn select_index_by_key(
    selected: &mut usize,
    keys: impl Iterator<Item = String>,
    expected: &str,
) -> bool {
    let Some(index) = keys
        .enumerate()
        .find_map(|(index, key)| (key == expected).then_some(index))
    else {
        return false;
    };
    *selected = index;
    true
}

fn commit_search_text(commit: &CommitEntry) -> String {
    format!(
        "{} {} {}",
        commit.id,
        author_initials(&commit.author_name),
        commit_message_summary(commit)
    )
}

fn commit_message_summary(commit: &CommitEntry) -> String {
    if commit.summary.is_empty() {
        commit
            .message
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        commit.summary.clone()
    }
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
