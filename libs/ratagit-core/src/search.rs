use crate::{
    AppContext, BranchInputMode, BranchesSubview, CommitEntry, CommitInputMode, FileInputMode,
    SearchScope, branches, commit_file_tree_rows_for_read, commit_key, file_tree_rows_for_read,
    leave_commit_files_multi_select, leave_commit_multi_select, leave_multi_select,
    select_commit_file_tree_path, select_file_tree_path,
};

pub(crate) fn start_search(state: &mut AppContext) {
    let Some(scope) = state.active_search_scope() else {
        return;
    };
    if scope == SearchScope::Files && state.ui.files.mode == FileInputMode::MultiSelect {
        leave_multi_select(&state.repo.files.items, &mut state.ui.files);
    }
    if scope == SearchScope::Branches && state.ui.branches.mode == BranchInputMode::MultiSelect {
        branches::leave_multi_select(&mut state.ui.branches);
    }
    if scope == SearchScope::Commits
        && state.ui.branches.subview == BranchesSubview::Commits
        && state.ui.branches.commits.mode == CommitInputMode::MultiSelect
    {
        leave_commit_multi_select(&mut state.ui.branches.commits);
    } else if scope == SearchScope::Commits && state.ui.commits.mode == CommitInputMode::MultiSelect
    {
        leave_commit_multi_select(&mut state.ui.commits);
    }
    if scope == SearchScope::CommitFiles
        && state.ui.branches.subview == BranchesSubview::CommitFiles
        && state.ui.branches.commit_files.mode == FileInputMode::MultiSelect
    {
        leave_commit_files_multi_select(
            &state.repo.branches.commit_files.items,
            &mut state.ui.branches.commit_files,
        );
    } else if scope == SearchScope::CommitFiles
        && state.ui.commits.files.mode == FileInputMode::MultiSelect
    {
        leave_commit_files_multi_select(
            &state.repo.commits.files.items,
            &mut state.ui.commits.files,
        );
    }
    state.ui.search.active = true;
    state.ui.search.scope = Some(scope);
    state.ui.search.query.clear();
    state.ui.search.matches.clear();
    state.ui.search.current_match = None;
}

pub(crate) fn input_search_char(state: &mut AppContext, ch: char) {
    if !search_input_is_current(state) {
        return;
    }
    state.ui.search.query.push(ch);
    recompute_search_matches(state);
}

pub(crate) fn backspace_search(state: &mut AppContext) {
    if !search_input_is_current(state) {
        return;
    }
    state.ui.search.query.pop();
    recompute_search_matches(state);
}

pub(crate) fn confirm_search(state: &mut AppContext) -> bool {
    if !search_input_is_current(state) {
        return false;
    }
    state.ui.search.active = false;
    recompute_search_matches(state);
    if state.ui.search.matches.is_empty() {
        return false;
    }
    state.ui.search.current_match = Some(0);
    select_current_search_match(state)
}

pub(crate) fn cancel_search(state: &mut AppContext) {
    if state.ui.search.scope == state.active_search_scope()
        && (state.ui.search.active || !state.ui.search.query.is_empty())
    {
        state.ui.search.clear();
    }
}

pub(crate) fn jump_search_match(state: &mut AppContext, previous: bool) -> bool {
    if state.ui.search.scope != state.active_search_scope() || state.ui.search.query.is_empty() {
        return false;
    }
    recompute_search_matches(state);
    if state.ui.search.matches.is_empty() {
        return false;
    }
    let len = state.ui.search.matches.len();
    let next = match (state.ui.search.current_match, previous) {
        (Some(index), true) => (index + len - 1) % len,
        (Some(index), false) => (index + 1) % len,
        (None, _) => 0,
    };
    state.ui.search.current_match = Some(next);
    select_current_search_match(state)
}

pub(crate) fn clear_search_if_incompatible(state: &mut AppContext) {
    if state.ui.search.scope.is_some() && state.ui.search.scope != state.active_search_scope() {
        state.ui.search.clear();
    }
}

pub(crate) fn recompute_search_matches(state: &mut AppContext) {
    if state.ui.search.query.is_empty() {
        state.ui.search.matches.clear();
        state.ui.search.current_match = None;
        return;
    }
    let Some(scope) = state.ui.search.scope else {
        state.ui.search.matches.clear();
        state.ui.search.current_match = None;
        return;
    };
    let query = state.ui.search.query.to_lowercase();
    state.ui.search.matches = match scope {
        SearchScope::Files => {
            let rows = file_tree_rows_for_read(&state.repo.files.items, &state.ui.files);
            collect_matches(
                rows.iter(),
                &query,
                |row| row.path.clone(),
                |row| row.path.clone(),
            )
        }
        SearchScope::Branches => collect_matches(
            state.repo.branches.items.iter(),
            &query,
            |branch| branch.name.clone(),
            |branch| branch.name.clone(),
        ),
        SearchScope::Commits => {
            if state.ui.branches.subview == BranchesSubview::Commits {
                collect_matches(
                    state.repo.branches.commits.iter(),
                    &query,
                    commit_key,
                    commit_search_text,
                )
            } else {
                collect_matches(
                    state.repo.commits.items.iter(),
                    &query,
                    commit_key,
                    commit_search_text,
                )
            }
        }
        SearchScope::Stash => collect_matches(
            state.repo.stash.items.iter(),
            &query,
            |stash| stash.id.clone(),
            |stash| format!("{} {}", stash.id, stash.summary),
        ),
        SearchScope::CommitFiles => {
            let rows = if state.ui.branches.subview == BranchesSubview::CommitFiles {
                commit_file_tree_rows_for_read(
                    &state.repo.branches.commit_files.items,
                    &state.ui.branches.commit_files,
                )
            } else {
                commit_file_tree_rows_for_read(
                    &state.repo.commits.files.items,
                    &state.ui.commits.files,
                )
            };
            collect_matches(
                rows.iter(),
                &query,
                |row| row.path.clone(),
                |row| row.path.clone(),
            )
        }
    };
    if state.ui.search.matches.is_empty() {
        state.ui.search.current_match = None;
    } else {
        state.ui.search.current_match = Some(
            state
                .ui
                .search
                .current_match
                .unwrap_or(0)
                .min(state.ui.search.matches.len() - 1),
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

fn search_input_is_current(state: &AppContext) -> bool {
    state.ui.search.active && state.ui.search.scope == state.active_search_scope()
}

fn select_current_search_match(state: &mut AppContext) -> bool {
    let Some(scope) = state.ui.search.scope else {
        return false;
    };
    let Some(index) = state.ui.search.current_match else {
        return false;
    };
    let Some(key) = state.ui.search.matches.get(index).cloned() else {
        return false;
    };
    match scope {
        SearchScope::Files => {
            select_file_tree_path(&state.repo.files.items, &mut state.ui.files, &key)
        }
        SearchScope::Branches => select_index_by_key(
            &mut state.ui.branches.selected,
            state
                .repo
                .branches
                .items
                .iter()
                .map(|branch| branch.name.clone()),
            &key,
        ),
        SearchScope::Commits => {
            if state.ui.branches.subview == BranchesSubview::Commits {
                select_index_by_key(
                    &mut state.ui.branches.commits.selected,
                    state.repo.branches.commits.iter().map(commit_key),
                    &key,
                )
            } else {
                select_index_by_key(
                    &mut state.ui.commits.selected,
                    state.repo.commits.items.iter().map(commit_key),
                    &key,
                )
            }
        }
        SearchScope::Stash => select_index_by_key(
            &mut state.ui.stash.selected,
            state.repo.stash.items.iter().map(|stash| stash.id.clone()),
            &key,
        ),
        SearchScope::CommitFiles => {
            if state.ui.branches.subview == BranchesSubview::CommitFiles {
                select_commit_file_tree_path(
                    &state.repo.branches.commit_files.items,
                    &mut state.ui.branches.commit_files,
                    &key,
                )
            } else {
                select_commit_file_tree_path(
                    &state.repo.commits.files.items,
                    &mut state.ui.commits.files,
                    &key,
                )
            }
        }
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
