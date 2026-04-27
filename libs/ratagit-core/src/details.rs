use crate::{
    AppState, BRANCH_DETAILS_LOG_MAX_COUNT, CachedBranchLog, CachedCommitDiff, CachedFilesDiff,
    Command, CommitFileDiffPath, CommitFileDiffTarget, DETAILS_DIFF_CACHE_LIMIT,
    FILES_DETAILS_DIFF_TARGET_LIMIT, FileDiffTarget, PanelFocus, push_notice,
    selected_commit_file_targets, selected_diff_targets, selected_target_paths, with_pending,
};

pub(crate) fn scroll_up(state: &mut AppState, lines: usize) {
    state.details.scroll_offset = state.details.scroll_offset.saturating_sub(lines);
}

pub(crate) fn scroll_down(state: &mut AppState, lines: usize, visible_lines: usize) {
    state.details.scroll_offset = state
        .details
        .scroll_offset
        .saturating_add(lines)
        .min(scroll_max_offset(state, visible_lines));
}

pub(crate) fn reset_after_snapshot(state: &mut AppState) {
    state.details.files_diff.clear();
    state.details.files_error = None;
    state.details.files_targets = selected_target_paths(&state.files);
    state.details.files_diff_truncated_from = None;
    state.details.branch_log.clear();
    state.details.branch_log_error = None;
    state.details.branch_log_target = crate::selected_branch_name(state);
    state.details.commit_diff.clear();
    state.details.commit_diff_error = None;
    state.details.commit_diff_target = crate::selected_commit_id(state);
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_error = None;
    state.details.commit_file_diff_target = None;
    reset_scroll(state);
    clear_caches(state);
}

pub(crate) fn apply_files_diff_result(
    state: &mut AppState,
    targets: Vec<FileDiffTarget>,
    truncated_from: Option<usize>,
    result: Result<String, String>,
) -> Vec<Command> {
    if state.last_left_focus != PanelFocus::Files {
        return Vec::new();
    }
    let current_request = files_diff_request_for_selection(state);
    if targets != current_request.targets || truncated_from != current_request.truncated_from {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("details".to_string());
    let paths = file_diff_target_paths(&targets);
    state.details.files_targets = paths.clone();
    state.details.files_diff_truncated_from = truncated_from;
    reset_scroll(state);
    match result {
        Ok(diff) => {
            cache_files_diff(state, &paths, &diff);
            state.details.files_diff = diff;
            state.details.files_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh files details diff: {error}");
            state.details.files_diff.clear();
            state.details.files_error = Some(message.clone());
            state.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn apply_branch_log_result(
    state: &mut AppState,
    branch: String,
    result: Result<String, String>,
) -> Vec<Command> {
    if state.last_left_focus != PanelFocus::Branches {
        return Vec::new();
    }
    if Some(branch.as_str()) != crate::selected_branch_name(state).as_deref() {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("branch_details".to_string());
    state.details.branch_log_target = Some(branch.clone());
    reset_scroll(state);
    match result {
        Ok(log) => {
            cache_branch_log(state, &branch, &log);
            state.details.branch_log = log;
            state.details.branch_log_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh branch log graph: {error}");
            state.details.branch_log.clear();
            state.details.branch_log_error = Some(message.clone());
            state.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn apply_commit_diff_result(
    state: &mut AppState,
    commit_id: String,
    result: Result<String, String>,
) -> Vec<Command> {
    if state.last_left_focus != PanelFocus::Commits {
        return Vec::new();
    }
    if state.commits.files.active {
        return Vec::new();
    }
    if Some(commit_id.as_str()) != crate::selected_commit_id(state).as_deref() {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("commit_details".to_string());
    state.details.commit_diff_target = Some(commit_id.clone());
    reset_scroll(state);
    match result {
        Ok(diff) => {
            cache_commit_diff(state, &commit_id, &diff);
            state.details.commit_diff = diff;
            state.details.commit_diff_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh commit details diff: {error}");
            state.details.commit_diff.clear();
            state.details.commit_diff_error = Some(message.clone());
            state.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn apply_commit_file_diff_result(
    state: &mut AppState,
    target: CommitFileDiffTarget,
    result: Result<String, String>,
) -> Vec<Command> {
    if !commit_file_diff_target_matches_selection(state, &target) {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("commit_file_details".to_string());
    state.details.commit_file_diff_target = Some(target.clone());
    reset_scroll(state);
    match result {
        Ok(diff) => {
            state.details.commit_file_diff = diff;
            state.details.commit_file_diff_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh commit file diff: {error}");
            state.details.commit_file_diff.clear();
            state.details.commit_file_diff_error = Some(message.clone());
            state.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn refresh_for_focus(state: &mut AppState) -> Vec<Command> {
    if state.focus == PanelFocus::Files {
        return refresh_files_details(state);
    }
    if state.focus == PanelFocus::Branches {
        return refresh_branch_log(state);
    }
    if state.focus == PanelFocus::Commits {
        if state.commits.files.active {
            return refresh_commit_file_diff(state);
        }
        return refresh_commit_diff(state);
    }
    Vec::new()
}

pub(crate) fn refresh_on_focus(state: &mut AppState) -> Vec<Command> {
    refresh_for_focus(state)
}

pub(crate) fn refresh_on_navigation(state: &mut AppState) -> Vec<Command> {
    refresh_for_focus(state)
}

pub(crate) fn refresh_files_details(state: &mut AppState) -> Vec<Command> {
    let request = files_diff_request_for_selection(state);
    let paths = file_diff_target_paths(&request.targets);
    let target_changed = state.details.files_targets != paths;
    state.details.files_targets = paths.clone();
    if target_changed {
        reset_scroll(state);
    }
    if request.targets.is_empty() {
        state.details.files_diff.clear();
        state.details.files_error = None;
        state.details.files_diff_truncated_from = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    if request.targets.iter().any(|target| {
        target.untracked && target.is_directory_marker && state.status.untracked_scan_skipped
    }) {
        let message = "details(files): untracked directory scan skipped in large repo mode";
        state.details.files_diff = message.to_string();
        state.details.files_error = None;
        state.details.files_diff_truncated_from = None;
        state.work.details_pending = false;
        push_notice(state, message);
        return Vec::new();
    }
    if let Some(diff) = cached_files_diff(state, &paths) {
        state.details.files_diff = diff;
        state.details.files_error = None;
        state.details.files_diff_truncated_from = request.truncated_from;
        state.work.details_pending = false;
        return Vec::new();
    }
    if let Some(total) = request.truncated_from {
        push_notice(
            state,
            &format!(
                "details diff limited to first {FILES_DETAILS_DIFF_TARGET_LIMIT} of {total} files"
            ),
        );
    }
    with_pending(
        state,
        vec![Command::RefreshFilesDetailsDiff {
            targets: request.targets,
            truncated_from: request.truncated_from,
        }],
    )
}

pub(crate) fn refresh_commit_diff(state: &mut AppState) -> Vec<Command> {
    let Some(commit_id) = crate::selected_commit_id(state) else {
        let target_changed = state.details.commit_diff_target.is_some();
        state.details.commit_diff.clear();
        state.details.commit_diff_target = None;
        state.details.commit_diff_error = None;
        if target_changed {
            reset_scroll(state);
        }
        state.work.details_pending = false;
        return Vec::new();
    };
    let target_changed = state.details.commit_diff_target.as_ref() != Some(&commit_id);
    state.details.commit_diff_target = Some(commit_id.clone());
    if target_changed {
        reset_scroll(state);
    }
    if let Some(diff) = cached_commit_diff(state, &commit_id) {
        state.details.commit_diff = diff;
        state.details.commit_diff_error = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    with_pending(state, vec![Command::RefreshCommitDetailsDiff { commit_id }])
}

pub(crate) fn refresh_commit_file_diff(state: &mut AppState) -> Vec<Command> {
    if !state.commits.files.active {
        return Vec::new();
    }
    let Some(commit_id) = state.commits.files.commit_id.clone() else {
        clear_commit_file_details(state);
        return Vec::new();
    };
    let files = selected_commit_file_targets(&state.commits.files);
    if files.is_empty() {
        clear_commit_file_details(state);
        return Vec::new();
    }
    let target = CommitFileDiffTarget {
        commit_id,
        paths: files
            .into_iter()
            .map(|file| CommitFileDiffPath {
                path: file.path,
                old_path: file.old_path,
            })
            .collect(),
    };
    let target_changed = state.details.commit_file_diff_target.as_ref() != Some(&target);
    state.details.commit_file_diff_target = Some(target.clone());
    if target_changed {
        reset_scroll(state);
        state.details.commit_file_diff.clear();
    }
    with_pending(state, vec![Command::RefreshCommitFileDiff { target }])
}

pub(crate) fn clear_commit_file_details(state: &mut AppState) {
    let target_changed = state.details.commit_file_diff_target.is_some();
    state.details.commit_file_diff.clear();
    state.details.commit_file_diff_target = None;
    state.details.commit_file_diff_error = None;
    if target_changed {
        reset_scroll(state);
    }
    state.work.details_pending = false;
}

pub(crate) fn reset_scroll(state: &mut AppState) {
    state.details.scroll_offset = 0;
}

pub(crate) fn clear_caches(state: &mut AppState) {
    state.details.cached_files_diffs.clear();
    state.details.cached_branch_logs.clear();
    state.details.cached_commit_diffs.clear();
}

fn refresh_branch_log(state: &mut AppState) -> Vec<Command> {
    let Some(branch) = crate::selected_branch_name(state) else {
        let target_changed = state.details.branch_log_target.is_some();
        state.details.branch_log.clear();
        state.details.branch_log_target = None;
        state.details.branch_log_error = None;
        if target_changed {
            reset_scroll(state);
        }
        state.work.details_pending = false;
        return Vec::new();
    };
    let target_changed = state.details.branch_log_target.as_ref() != Some(&branch);
    state.details.branch_log_target = Some(branch.clone());
    if target_changed {
        reset_scroll(state);
    }
    if let Some(log) = cached_branch_log(state, &branch) {
        state.details.branch_log = log;
        state.details.branch_log_error = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    with_pending(
        state,
        vec![Command::RefreshBranchDetailsLog {
            branch,
            max_count: BRANCH_DETAILS_LOG_MAX_COUNT,
        }],
    )
}

fn commit_file_diff_target_matches_selection(
    state: &AppState,
    target: &CommitFileDiffTarget,
) -> bool {
    if state.last_left_focus != PanelFocus::Commits || !state.commits.files.active {
        return false;
    }
    let Some(commit_id) = state.commits.files.commit_id.as_deref() else {
        return false;
    };
    if commit_id != target.commit_id {
        return false;
    }
    let selected = selected_commit_file_targets(&state.commits.files)
        .into_iter()
        .map(|file| CommitFileDiffPath {
            path: file.path,
            old_path: file.old_path,
        })
        .collect::<Vec<_>>();
    selected == target.paths
}

fn cached_files_diff(state: &AppState, paths: &[String]) -> Option<String> {
    cached_entry(
        &state.details.cached_files_diffs,
        paths,
        |entry, paths| entry.paths.as_slice() == paths,
        |entry| &entry.diff,
    )
}

fn cache_files_diff(state: &mut AppState, paths: &[String], diff: &str) {
    cache_entry(
        &mut state.details.cached_files_diffs,
        paths,
        diff,
        |entry, paths| entry.paths.as_slice() == paths,
        |paths, diff| CachedFilesDiff {
            paths: paths.to_vec(),
            diff: diff.to_string(),
        },
    );
}

fn cached_branch_log(state: &AppState, branch: &str) -> Option<String> {
    cached_entry(
        &state.details.cached_branch_logs,
        branch,
        |entry, branch| entry.branch.as_str() == branch,
        |entry| &entry.log,
    )
}

fn cache_branch_log(state: &mut AppState, branch: &str, log: &str) {
    cache_entry(
        &mut state.details.cached_branch_logs,
        branch,
        log,
        |entry, branch| entry.branch.as_str() == branch,
        |branch, log| CachedBranchLog {
            branch: branch.to_string(),
            log: log.to_string(),
        },
    );
}

fn cached_commit_diff(state: &AppState, commit_id: &str) -> Option<String> {
    cached_entry(
        &state.details.cached_commit_diffs,
        commit_id,
        |entry, commit_id| entry.commit_id.as_str() == commit_id,
        |entry| &entry.diff,
    )
}

fn cache_commit_diff(state: &mut AppState, commit_id: &str, diff: &str) {
    cache_entry(
        &mut state.details.cached_commit_diffs,
        commit_id,
        diff,
        |entry, commit_id| entry.commit_id.as_str() == commit_id,
        |commit_id, diff| CachedCommitDiff {
            commit_id: commit_id.to_string(),
            diff: diff.to_string(),
        },
    );
}

fn cached_entry<K: ?Sized, E>(
    entries: &[E],
    key: &K,
    mut matches_key: impl FnMut(&E, &K) -> bool,
    value: impl Fn(&E) -> &str,
) -> Option<String> {
    entries
        .iter()
        .find(|entry| matches_key(entry, key))
        .map(|entry| value(entry).to_string())
}

fn cache_entry<K: ?Sized, E>(
    entries: &mut Vec<E>,
    key: &K,
    value: &str,
    mut matches_key: impl FnMut(&E, &K) -> bool,
    build: impl FnOnce(&K, &str) -> E,
) {
    entries.retain(|entry| !matches_key(entry, key));
    entries.insert(0, build(key, value));
    entries.truncate(DETAILS_DIFF_CACHE_LIMIT);
}

fn scroll_max_offset(state: &AppState, visible_lines: usize) -> usize {
    match state.last_left_focus {
        PanelFocus::Files => state
            .details
            .files_diff
            .lines()
            .count()
            .saturating_sub(visible_lines),
        PanelFocus::Branches => state
            .details
            .branch_log
            .lines()
            .count()
            .saturating_sub(visible_lines),
        PanelFocus::Commits => {
            if state.commits.files.active {
                state
                    .details
                    .commit_file_diff
                    .lines()
                    .count()
                    .saturating_sub(visible_lines)
            } else {
                state
                    .details
                    .commit_diff
                    .lines()
                    .count()
                    .saturating_sub(visible_lines)
            }
        }
        PanelFocus::Stash | PanelFocus::Details | PanelFocus::Log => 0,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilesDiffRequest {
    targets: Vec<FileDiffTarget>,
    truncated_from: Option<usize>,
}

fn files_diff_request_for_selection(state: &AppState) -> FilesDiffRequest {
    let all_targets = selected_diff_targets(&state.files);
    let total = all_targets.len();
    let truncated_from = (total > FILES_DETAILS_DIFF_TARGET_LIMIT).then_some(total);
    let targets = all_targets
        .into_iter()
        .take(FILES_DETAILS_DIFF_TARGET_LIMIT)
        .collect();
    FilesDiffRequest {
        targets,
        truncated_from,
    }
}

fn file_diff_target_paths(targets: &[FileDiffTarget]) -> Vec<String> {
    targets.iter().map(|target| target.path.clone()).collect()
}
