use crate::{
    AppContext, BranchDeleteMode, Command, GitErrorKind, GitFailure, ResetMode, branches, details,
    push_notice, with_pending,
};

pub(crate) fn handle_stage_files_result(
    state: &mut AppContext,
    paths: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "stage",
        format!("Staged {}", format_paths(&paths)),
        format!("Failed to stage {}", format_paths(&paths)),
    )
}

pub(crate) fn handle_unstage_files_result(
    state: &mut AppContext,
    paths: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "unstage",
        format!("Unstaged {}", format_paths(&paths)),
        format!("Failed to unstage {}", format_paths(&paths)),
    )
}

pub(crate) fn handle_stash_files_result(
    state: &mut AppContext,
    message: String,
    paths: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "stash_files",
        format!("Stashed {}: {message}", format_paths(&paths)),
        format!("Failed to stash {}", format_paths(&paths)),
    )
}

pub(crate) fn handle_reset_result(
    state: &mut AppContext,
    mode: ResetMode,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        &format!("reset_{}", reset_mode_name(mode)),
        format!("Reset {} to HEAD", reset_mode_name(mode)),
        format!("Failed to reset {}", reset_mode_name(mode)),
    )
}

pub(crate) fn handle_nuke_result(
    state: &mut AppContext,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "nuke",
        "Nuked working tree".to_string(),
        "Failed to nuke working tree".to_string(),
    )
}

pub(crate) fn handle_discard_files_result(
    state: &mut AppContext,
    paths: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "discard_files",
        format!("Discarded {}", format_paths(&paths)),
        format!("Failed to discard {}", format_paths(&paths)),
    )
}

pub(crate) fn handle_create_commit_result(
    state: &mut AppContext,
    message: String,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "commit",
        format!("Commit created: {message}"),
        "Failed to create commit".to_string(),
    )
}

pub(crate) fn handle_pull_result(
    state: &mut AppContext,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "pull",
        "Pulled from remote".to_string(),
        "Failed to pull".to_string(),
    )
}

pub(crate) fn handle_push_result(
    state: &mut AppContext,
    force: bool,
    result: Result<(), GitFailure>,
) -> Vec<Command> {
    if let Err(error) = &result
        && !force
        && error.kind == GitErrorKind::DivergentPush
    {
        record_operation(state, "push");
        let full_error = format!("Push requires confirmation: {error}");
        state.repo.status.last_error = Some(full_error.clone());
        push_notice(state, &full_error);
        state.ui.push_force_confirm.active = true;
        state.ui.push_force_confirm.reason = error.message.clone();
        return Vec::new();
    }

    handle_operation_result(
        state,
        result.map_err(|error| error.to_string()),
        if force { "force_push" } else { "push" },
        if force {
            "Force pushed to remote".to_string()
        } else {
            "Pushed to remote".to_string()
        },
        if force {
            "Failed to force push".to_string()
        } else {
            "Failed to push".to_string()
        },
    )
}

pub(crate) fn handle_create_branch_result(
    state: &mut AppContext,
    name: String,
    start_point: String,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "create_branch",
        format!("Branch created: {name} from {start_point}"),
        format!("Failed to create branch: {name}"),
    )
}

pub(crate) fn handle_checkout_branch_result(
    state: &mut AppContext,
    name: String,
    auto_stash: bool,
    result: Result<(), String>,
) -> Vec<Command> {
    let success_message = if auto_stash {
        format!("Checked out with auto-stash: {name}")
    } else {
        format!("Checked out: {name}")
    };
    let failure_prefix = format!("Failed to checkout branch: {name}");
    if auto_stash {
        handle_operation_result_refreshing_after_failure(
            state,
            result,
            "checkout_branch",
            success_message,
            failure_prefix,
        )
    } else {
        handle_operation_result(
            state,
            result,
            "checkout_branch",
            success_message,
            failure_prefix,
        )
    }
}

pub(crate) fn handle_delete_branch_result(
    state: &mut AppContext,
    name: String,
    mode: BranchDeleteMode,
    force: bool,
    result: Result<(), GitFailure>,
) -> Vec<Command> {
    if let Err(error) = &result
        && !force
        && branches::delete_mode_includes_local(mode)
        && error.kind == GitErrorKind::UnmergedBranchDelete
    {
        record_operation(state, &format!("delete_branch_{}", delete_mode_name(mode)));
        state.repo.status.last_error = Some(format!(
            "Branch is not fully merged; confirmation required: {error}"
        ));
        branches::open_force_delete_confirm(state, name, mode, error.message.clone());
        return Vec::new();
    }

    handle_operation_result(
        state,
        result.map_err(|error| error.to_string()),
        &format!("delete_branch_{}", delete_mode_name(mode)),
        if force {
            format!("Force deleted {} branch: {name}", delete_mode_label(mode))
        } else {
            format!("Deleted {} branch: {name}", delete_mode_label(mode))
        },
        format!(
            "Failed to delete {} branch: {name}",
            delete_mode_label(mode)
        ),
    )
}

pub(crate) fn handle_rebase_branch_result(
    state: &mut AppContext,
    target: String,
    interactive: bool,
    auto_stash: bool,
    result: Result<(), String>,
) -> Vec<Command> {
    let operation_key = if interactive {
        "rebase_branch_interactive"
    } else {
        "rebase_branch_simple"
    };
    let mode = if interactive { "interactive" } else { "simple" };
    let success_message = if auto_stash {
        format!("Rebased with auto-stash ({mode}) onto {target}")
    } else {
        format!("Rebased ({mode}) onto {target}")
    };
    let failure_prefix = format!("Failed to rebase onto {target}");
    if auto_stash {
        handle_operation_result_refreshing_after_failure(
            state,
            result,
            operation_key,
            success_message,
            failure_prefix,
        )
    } else {
        handle_operation_result(
            state,
            result,
            operation_key,
            success_message,
            failure_prefix,
        )
    }
}

pub(crate) fn handle_squash_commits_result(
    state: &mut AppContext,
    commit_ids: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "squash_commits",
        format!("Squashed {}", format_commit_count(commit_ids.len())),
        "Failed to squash commits".to_string(),
    )
}

pub(crate) fn handle_fixup_commits_result(
    state: &mut AppContext,
    commit_ids: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "fixup_commits",
        format!("Fixed up {}", format_commit_count(commit_ids.len())),
        "Failed to fixup commits".to_string(),
    )
}

pub(crate) fn handle_reword_commit_result(
    state: &mut AppContext,
    commit_id: String,
    message: String,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "reword_commit",
        format!("Reworded {commit_id}: {}", first_line(&message)),
        format!("Failed to reword commit: {commit_id}"),
    )
}

pub(crate) fn handle_delete_commits_result(
    state: &mut AppContext,
    commit_ids: Vec<String>,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "delete_commits",
        format!("Deleted {}", format_commit_count(commit_ids.len())),
        "Failed to delete commits".to_string(),
    )
}

pub(crate) fn handle_checkout_commit_detached_result(
    state: &mut AppContext,
    commit_id: String,
    auto_stash: bool,
    result: Result<(), String>,
) -> Vec<Command> {
    let success_message = if auto_stash {
        format!("Checked out detached with auto-stash: {commit_id}")
    } else {
        format!("Checked out detached: {commit_id}")
    };
    let failure_prefix = format!("Failed to checkout detached: {commit_id}");
    if auto_stash {
        handle_operation_result_refreshing_after_failure(
            state,
            result,
            "checkout_detached",
            success_message,
            failure_prefix,
        )
    } else {
        handle_operation_result(
            state,
            result,
            "checkout_detached",
            success_message,
            failure_prefix,
        )
    }
}

pub(crate) fn handle_stash_push_result(
    state: &mut AppContext,
    message: String,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "stash_push",
        format!("Stash pushed: {message}"),
        "Failed to stash push".to_string(),
    )
}

pub(crate) fn handle_stash_pop_result(
    state: &mut AppContext,
    stash_id: String,
    result: Result<(), String>,
) -> Vec<Command> {
    handle_operation_result(
        state,
        result,
        "stash_pop",
        format!("Stash popped: {stash_id}"),
        format!("Failed to stash pop: {stash_id}"),
    )
}

fn handle_operation_result(
    state: &mut AppContext,
    result: Result<(), String>,
    operation_key: &str,
    success_message: String,
    failure_prefix: String,
) -> Vec<Command> {
    match result {
        Ok(()) => {
            details::clear_caches(state);
            record_operation(state, operation_key);
            push_notice(state, &success_message);
            state.repo.status.last_error = None;
            with_pending(state, Command::refresh_all_commands())
        }
        Err(error_message) => {
            record_operation(state, operation_key);
            let full_error = format!("{failure_prefix}: {error_message}");
            state.repo.status.last_error = Some(full_error.clone());
            push_notice(state, &full_error);
            Vec::new()
        }
    }
}

fn handle_operation_result_refreshing_after_failure(
    state: &mut AppContext,
    result: Result<(), String>,
    operation_key: &str,
    success_message: String,
    failure_prefix: String,
) -> Vec<Command> {
    match result {
        Ok(()) => handle_operation_result(
            state,
            Ok(()),
            operation_key,
            success_message,
            failure_prefix,
        ),
        Err(error_message) => {
            record_operation(state, operation_key);
            let full_error = format!("{failure_prefix}: {error_message}");
            state.repo.status.last_error = Some(full_error.clone());
            push_notice(state, &full_error);
            with_pending(state, Command::refresh_all_commands())
        }
    }
}

fn record_operation(state: &mut AppContext, operation_key: &str) {
    let operation_key = operation_key.to_string();
    state.work.record_operation_completed(operation_key.clone());
    state.last_operation = Some(operation_key);
}

fn first_line(message: &str) -> &str {
    message.lines().next().unwrap_or("").trim()
}

fn format_paths(paths: &[String]) -> String {
    match paths {
        [] => "<none>".to_string(),
        [only] => only.clone(),
        _ => format!("{} files", paths.len()),
    }
}

pub(crate) fn format_commit_count(count: usize) -> String {
    if count == 1 {
        "1 commit".to_string()
    } else {
        format!("{count} commits")
    }
}

pub(crate) fn reset_mode_name(mode: ResetMode) -> &'static str {
    match mode {
        ResetMode::Mixed => "mixed",
        ResetMode::Soft => "soft",
        ResetMode::Hard => "hard",
    }
}

pub(crate) fn delete_mode_name(mode: BranchDeleteMode) -> &'static str {
    match mode {
        BranchDeleteMode::Local => "local",
        BranchDeleteMode::Remote => "remote",
        BranchDeleteMode::Both => "both",
    }
}

fn delete_mode_label(mode: BranchDeleteMode) -> &'static str {
    match mode {
        BranchDeleteMode::Local => "local",
        BranchDeleteMode::Remote => "remote",
        BranchDeleteMode::Both => "local and remote",
    }
}
