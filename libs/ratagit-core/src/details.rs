use crate::{
    AppContext, BRANCH_DETAILS_LOG_MAX_COUNT, CachedBranchLog, CachedCommitDiff, CachedFilesDiff,
    Command, CommitFileDiffPath, CommitFileDiffTarget, DETAILS_DIFF_CACHE_LIMIT,
    FILES_DETAILS_DIFF_TARGET_LIMIT, FileDiffTarget, PanelFocus, push_notice,
    selected_commit_file_targets, selected_diff_targets, selected_target_paths, with_pending,
};

pub(crate) fn scroll_up(state: &mut AppContext, lines: usize) {
    state.ui.details.scroll_offset = state.ui.details.scroll_offset.saturating_sub(lines);
}

pub(crate) fn scroll_down(state: &mut AppContext, lines: usize, visible_lines: usize) {
    state.ui.details.scroll_offset = state
        .ui
        .details
        .scroll_offset
        .saturating_add(lines)
        .min(scroll_max_offset(state, visible_lines));
}

pub(crate) fn reset_after_snapshot(state: &mut AppContext) {
    state.repo.details.files_diff.clear();
    state.repo.details.files_error = None;
    state.repo.details.files_targets =
        selected_target_paths(&state.repo.files.items, &state.ui.files);
    state.repo.details.files_diff_truncated_from = None;
    state.repo.details.branch_log.clear();
    state.repo.details.branch_log_error = None;
    state.repo.details.branch_log_target = crate::selected_branch_name(state);
    state.repo.details.commit_diff.clear();
    state.repo.details.commit_diff_error = None;
    state.repo.details.commit_diff_target = crate::selected_commit_id(state);
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_error = None;
    state.repo.details.commit_file_diff_target = None;
    reset_scroll(state);
    clear_caches(state);
}

pub(crate) fn apply_files_diff_result(
    state: &mut AppContext,
    targets: Vec<FileDiffTarget>,
    truncated_from: Option<usize>,
    result: Result<String, String>,
) -> Vec<Command> {
    if state.ui.last_left_focus != PanelFocus::Files {
        return Vec::new();
    }
    let current_request = files_diff_request_for_selection(state);
    if targets != current_request.targets || truncated_from != current_request.truncated_from {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("details".to_string());
    let paths = file_diff_target_paths(&targets);
    state.repo.details.files_targets = paths.clone();
    state.repo.details.files_diff_truncated_from = truncated_from;
    reset_scroll(state);
    match result {
        Ok(diff) => {
            cache_files_diff(state, &paths, &diff);
            state.repo.details.files_diff = diff;
            state.repo.details.files_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh files details diff: {error}");
            state.repo.details.files_diff.clear();
            state.repo.details.files_error = Some(message.clone());
            state.repo.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn apply_branch_log_result(
    state: &mut AppContext,
    branch: String,
    result: Result<String, String>,
) -> Vec<Command> {
    if state.ui.last_left_focus != PanelFocus::Branches {
        return Vec::new();
    }
    if Some(branch.as_str()) != crate::selected_branch_name(state).as_deref() {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("branch_details".to_string());
    state.repo.details.branch_log_target = Some(branch.clone());
    reset_scroll(state);
    match result {
        Ok(log) => {
            cache_branch_log(state, &branch, &log);
            state.repo.details.branch_log = log;
            state.repo.details.branch_log_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh branch log graph: {error}");
            state.repo.details.branch_log.clear();
            state.repo.details.branch_log_error = Some(message.clone());
            state.repo.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn apply_commit_diff_result(
    state: &mut AppContext,
    commit_id: String,
    result: Result<String, String>,
) -> Vec<Command> {
    if state.ui.last_left_focus != PanelFocus::Commits {
        return Vec::new();
    }
    if state.ui.commits.files.active {
        return Vec::new();
    }
    if Some(commit_id.as_str()) != crate::selected_commit_id(state).as_deref() {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("commit_details".to_string());
    state.repo.details.commit_diff_target = Some(commit_id.clone());
    reset_scroll(state);
    match result {
        Ok(diff) => {
            cache_commit_diff(state, &commit_id, &diff);
            state.repo.details.commit_diff = diff;
            state.repo.details.commit_diff_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh commit details diff: {error}");
            state.repo.details.commit_diff.clear();
            state.repo.details.commit_diff_error = Some(message.clone());
            state.repo.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn apply_commit_file_diff_result(
    state: &mut AppContext,
    target: CommitFileDiffTarget,
    result: Result<String, String>,
) -> Vec<Command> {
    if !commit_file_diff_target_matches_selection(state, &target) {
        return Vec::new();
    }
    state.work.details_pending = false;
    state.work.last_completed_command = Some("commit_file_details".to_string());
    state.repo.details.commit_file_diff_target = Some(target.clone());
    reset_scroll(state);
    match result {
        Ok(diff) => {
            state.repo.details.commit_file_diff = diff;
            state.repo.details.commit_file_diff_error = None;
        }
        Err(error) => {
            let message = format!("Failed to refresh commit file diff: {error}");
            state.repo.details.commit_file_diff.clear();
            state.repo.details.commit_file_diff_error = Some(message.clone());
            state.repo.status.last_error = Some(message.clone());
            push_notice(state, &message);
        }
    }
    Vec::new()
}

pub(crate) fn refresh_for_focus(state: &mut AppContext) -> Vec<Command> {
    if state.ui.focus == PanelFocus::Files {
        return refresh_files_details(state);
    }
    if state.ui.focus == PanelFocus::Branches {
        return refresh_branch_log(state);
    }
    if state.ui.focus == PanelFocus::Commits {
        if state.ui.commits.files.active {
            return refresh_commit_file_diff(state);
        }
        return refresh_commit_diff(state);
    }
    Vec::new()
}

pub(crate) fn refresh_on_focus(state: &mut AppContext) -> Vec<Command> {
    refresh_for_focus(state)
}

pub(crate) fn refresh_on_navigation(state: &mut AppContext) -> Vec<Command> {
    refresh_for_focus(state)
}

pub(crate) fn refresh_files_details(state: &mut AppContext) -> Vec<Command> {
    let request = files_diff_request_for_selection(state);
    let paths = file_diff_target_paths(&request.targets);
    let target_changed = state.repo.details.files_targets != paths;
    state.repo.details.files_targets = paths.clone();
    if target_changed {
        reset_scroll(state);
    }
    if request.targets.is_empty() {
        state.repo.details.files_diff.clear();
        state.repo.details.files_error = None;
        state.repo.details.files_diff_truncated_from = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    if request.targets.iter().any(|target| {
        target.untracked && target.is_directory_marker && state.repo.status.untracked_scan_skipped
    }) {
        let message = "details(files): untracked directory scan skipped in large repo mode";
        state.repo.details.files_diff = message.to_string();
        state.repo.details.files_error = None;
        state.repo.details.files_diff_truncated_from = None;
        state.work.details_pending = false;
        push_notice(state, message);
        return Vec::new();
    }
    if let Some(diff) = cached_files_diff(state, &paths) {
        state.repo.details.files_diff = diff;
        state.repo.details.files_error = None;
        state.repo.details.files_diff_truncated_from = request.truncated_from;
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

pub(crate) fn refresh_commit_diff(state: &mut AppContext) -> Vec<Command> {
    let Some(commit_id) = crate::selected_commit_id(state) else {
        let target_changed = state.repo.details.commit_diff_target.is_some();
        state.repo.details.commit_diff.clear();
        state.repo.details.commit_diff_target = None;
        state.repo.details.commit_diff_error = None;
        if target_changed {
            reset_scroll(state);
        }
        state.work.details_pending = false;
        return Vec::new();
    };
    let target_changed = state.repo.details.commit_diff_target.as_ref() != Some(&commit_id);
    state.repo.details.commit_diff_target = Some(commit_id.clone());
    if target_changed {
        reset_scroll(state);
    }
    if let Some(diff) = cached_commit_diff(state, &commit_id) {
        state.repo.details.commit_diff = diff;
        state.repo.details.commit_diff_error = None;
        state.work.details_pending = false;
        return Vec::new();
    }
    with_pending(state, vec![Command::RefreshCommitDetailsDiff { commit_id }])
}

pub(crate) fn refresh_commit_file_diff(state: &mut AppContext) -> Vec<Command> {
    if !state.ui.commits.files.active {
        return Vec::new();
    }
    let Some(commit_id) = state.ui.commits.files.commit_id.clone() else {
        clear_commit_file_details(state);
        return Vec::new();
    };
    let files =
        selected_commit_file_targets(&state.repo.commits.files.items, &state.ui.commits.files);
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
    let target_changed = state.repo.details.commit_file_diff_target.as_ref() != Some(&target);
    state.repo.details.commit_file_diff_target = Some(target.clone());
    if target_changed {
        reset_scroll(state);
    }
    with_pending(state, vec![Command::RefreshCommitFileDiff { target }])
}

pub(crate) fn clear_commit_file_details(state: &mut AppContext) {
    let target_changed = state.repo.details.commit_file_diff_target.is_some();
    state.repo.details.commit_file_diff.clear();
    state.repo.details.commit_file_diff_target = None;
    state.repo.details.commit_file_diff_error = None;
    if target_changed {
        reset_scroll(state);
    }
    state.work.details_pending = false;
}

pub(crate) fn reset_scroll(state: &mut AppContext) {
    state.ui.details.scroll_offset = 0;
}

pub(crate) fn clear_caches(state: &mut AppContext) {
    state.repo.details.cached_files_diffs.clear();
    state.repo.details.cached_branch_logs.clear();
    state.repo.details.cached_commit_diffs.clear();
}

fn refresh_branch_log(state: &mut AppContext) -> Vec<Command> {
    let Some(branch) = crate::selected_branch_name(state) else {
        let target_changed = state.repo.details.branch_log_target.is_some();
        state.repo.details.branch_log.clear();
        state.repo.details.branch_log_target = None;
        state.repo.details.branch_log_error = None;
        if target_changed {
            reset_scroll(state);
        }
        state.work.details_pending = false;
        return Vec::new();
    };
    let target_changed = state.repo.details.branch_log_target.as_ref() != Some(&branch);
    state.repo.details.branch_log_target = Some(branch.clone());
    if target_changed {
        reset_scroll(state);
    }
    if let Some(log) = cached_branch_log(state, &branch) {
        state.repo.details.branch_log = log;
        state.repo.details.branch_log_error = None;
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
    state: &AppContext,
    target: &CommitFileDiffTarget,
) -> bool {
    if state.ui.last_left_focus != PanelFocus::Commits || !state.ui.commits.files.active {
        return false;
    }
    let Some(commit_id) = state.ui.commits.files.commit_id.as_deref() else {
        return false;
    };
    if commit_id != target.commit_id {
        return false;
    }
    let selected =
        selected_commit_file_targets(&state.repo.commits.files.items, &state.ui.commits.files)
            .into_iter()
            .map(|file| CommitFileDiffPath {
                path: file.path,
                old_path: file.old_path,
            })
            .collect::<Vec<_>>();
    selected == target.paths
}

fn cached_files_diff(state: &AppContext, paths: &[String]) -> Option<String> {
    cached_entry(
        &state.repo.details.cached_files_diffs,
        paths,
        |entry, paths| entry.paths.as_slice() == paths,
        |entry| &entry.diff,
    )
}

fn cache_files_diff(state: &mut AppContext, paths: &[String], diff: &str) {
    cache_entry(
        &mut state.repo.details.cached_files_diffs,
        paths,
        diff,
        |entry, paths| entry.paths.as_slice() == paths,
        |paths, diff| CachedFilesDiff {
            paths: paths.to_vec(),
            diff: diff.to_string(),
        },
    );
}

fn cached_branch_log(state: &AppContext, branch: &str) -> Option<String> {
    cached_entry(
        &state.repo.details.cached_branch_logs,
        branch,
        |entry, branch| entry.branch.as_str() == branch,
        |entry| &entry.log,
    )
}

fn cache_branch_log(state: &mut AppContext, branch: &str, log: &str) {
    cache_entry(
        &mut state.repo.details.cached_branch_logs,
        branch,
        log,
        |entry, branch| entry.branch.as_str() == branch,
        |branch, log| CachedBranchLog {
            branch: branch.to_string(),
            log: log.to_string(),
        },
    );
}

fn cached_commit_diff(state: &AppContext, commit_id: &str) -> Option<String> {
    cached_entry(
        &state.repo.details.cached_commit_diffs,
        commit_id,
        |entry, commit_id| entry.commit_id.as_str() == commit_id,
        |entry| &entry.diff,
    )
}

fn cache_commit_diff(state: &mut AppContext, commit_id: &str, diff: &str) {
    cache_entry(
        &mut state.repo.details.cached_commit_diffs,
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

fn scroll_max_offset(state: &AppContext, visible_lines: usize) -> usize {
    match state.ui.last_left_focus {
        PanelFocus::Files => state
            .repo
            .details
            .files_diff
            .lines()
            .count()
            .saturating_sub(visible_lines),
        PanelFocus::Branches => state
            .repo
            .details
            .branch_log
            .lines()
            .count()
            .saturating_sub(visible_lines),
        PanelFocus::Commits => {
            if state.ui.commits.files.active {
                state
                    .repo
                    .details
                    .commit_file_diff
                    .lines()
                    .count()
                    .saturating_sub(visible_lines)
            } else {
                state
                    .repo
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

fn files_diff_request_for_selection(state: &AppContext) -> FilesDiffRequest {
    let all_targets = selected_diff_targets(&state.repo.files.items, &state.ui.files);
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

#[cfg(test)]
mod tests {
    use crate::{
        AppContext, Command, CommitFileDiffPath, CommitFileDiffTarget, CommitFileEntry,
        CommitFileStatus, CommitFilesUiState, PanelFocus, initialize_commit_files_tree,
        move_commit_file_selected,
    };

    #[test]
    fn commit_file_details_keep_previous_diff_while_new_target_is_pending() {
        let previous_target = CommitFileDiffTarget {
            commit_id: "abc1234".to_string(),
            paths: vec![CommitFileDiffPath {
                path: "README.md".to_string(),
                old_path: None,
            }],
        };
        let mut state = AppContext::default();
        state.ui.focus = PanelFocus::Commits;
        state.ui.last_left_focus = PanelFocus::Commits;
        state.repo.commits.files.items = vec![
            CommitFileEntry {
                path: "README.md".to_string(),
                old_path: None,
                status: CommitFileStatus::Modified,
            },
            CommitFileEntry {
                path: "src/lib.rs".to_string(),
                old_path: None,
                status: CommitFileStatus::Added,
            },
        ];
        state.ui.commits.files = CommitFilesUiState {
            active: true,
            commit_id: Some("abc1234".to_string()),
            ..CommitFilesUiState::default()
        };
        initialize_commit_files_tree(&state.repo.commits.files.items, &mut state.ui.commits.files);
        state.repo.details.commit_file_diff_target = Some(previous_target);
        state.repo.details.commit_file_diff = "diff --git a/README.md b/README.md".to_string();

        move_commit_file_selected(&mut state.ui.commits.files, false);
        let commands = super::refresh_commit_file_diff(&mut state);

        assert!(matches!(
            commands.as_slice(),
            [Command::RefreshCommitFileDiff { .. }]
        ));
        assert_eq!(
            state.repo.details.commit_file_diff,
            "diff --git a/README.md b/README.md"
        );
        assert!(state.work.details_pending);
    }
}
