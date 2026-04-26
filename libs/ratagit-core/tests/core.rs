use ratagit_core::{
    Action, AppState, AutoStashOperation, BranchDeleteChoice, BranchDeleteMode, BranchEntry,
    BranchRebaseChoice, COMMITS_PAGE_SIZE, COMMITS_PREFETCH_THRESHOLD, Command, CommitEditorIntent,
    CommitEntry, CommitField, CommitHashStatus, CommitInputMode, EditorKind, FileEntry,
    FileInputMode, GitResult, PanelFocus, RepoSnapshot, ResetChoice, ResetMode, StashScope,
    UiAction, refresh_tree_projection, update,
};

fn commit_entry(id: &str, summary: &str) -> CommitEntry {
    CommitEntry {
        id: id.to_string(),
        full_id: format!("{id}-full"),
        summary: summary.to_string(),
        message: summary.to_string(),
        author_name: "ratagit-tests".to_string(),
        graph: "●".to_string(),
        hash_status: CommitHashStatus::Unpushed,
        is_merge: false,
    }
}

fn commit_entries(count: usize) -> Vec<CommitEntry> {
    (0..count)
        .map(|index| commit_entry(&format!("{index:07x}"), &format!("commit {index}")))
        .collect()
}

#[test]
fn refresh_action_emits_refresh_command() {
    let mut state = AppState::default();
    let commands = update(&mut state, Action::Ui(UiAction::RefreshAll));
    assert_eq!(commands, vec![Command::RefreshAll]);
    assert!(state.work.refresh_pending);
}

#[test]
fn refreshed_commit_page_tracks_pagination_state() {
    let mut state = AppState::default();

    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: commit_entries(COMMITS_PAGE_SIZE),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );

    assert_eq!(state.commits.items.len(), COMMITS_PAGE_SIZE);
    assert!(state.commits.has_more);
    assert!(!state.commits.loading_more);
    assert!(!state.commits.pending_select_after_load);
    assert_eq!(state.commits.pagination_epoch, 1);
}

#[test]
fn commits_move_down_past_loaded_page_requests_and_appends_next_page() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        last_left_focus: PanelFocus::Commits,
        ..AppState::default()
    };
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: commit_entries(COMMITS_PAGE_SIZE),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );
    state.commits.selected = COMMITS_PAGE_SIZE - 1;

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));

    assert_eq!(
        commands,
        vec![Command::LoadMoreCommits {
            offset: COMMITS_PAGE_SIZE,
            limit: COMMITS_PAGE_SIZE,
            epoch: state.commits.pagination_epoch,
        }]
    );
    assert!(state.commits.loading_more);
    assert!(state.commits.pending_select_after_load);

    let next_page = (COMMITS_PAGE_SIZE..COMMITS_PAGE_SIZE * 2)
        .map(|index| commit_entry(&format!("{index:07x}"), &format!("commit {index}")))
        .collect::<Vec<_>>();
    let epoch = state.commits.pagination_epoch;
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::CommitsPage {
            offset: COMMITS_PAGE_SIZE,
            limit: COMMITS_PAGE_SIZE,
            epoch,
            result: Ok(next_page),
        }),
    );

    assert!(commands.is_empty());
    assert_eq!(state.commits.items.len(), COMMITS_PAGE_SIZE * 2);
    assert_eq!(state.commits.selected, COMMITS_PAGE_SIZE);
    assert!(state.commits.has_more);
    assert!(!state.commits.loading_more);
}

#[test]
fn commits_prefetch_before_loaded_tail_without_jumping_selection() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        last_left_focus: PanelFocus::Commits,
        ..AppState::default()
    };
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: commit_entries(COMMITS_PAGE_SIZE),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );
    state.commits.selected = COMMITS_PAGE_SIZE - COMMITS_PREFETCH_THRESHOLD - 2;

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));

    assert_eq!(
        commands,
        vec![Command::LoadMoreCommits {
            offset: COMMITS_PAGE_SIZE,
            limit: COMMITS_PAGE_SIZE,
            epoch: state.commits.pagination_epoch,
        }]
    );
    assert!(state.commits.loading_more);
    assert!(!state.commits.pending_select_after_load);
    assert_eq!(
        state.commits.selected,
        COMMITS_PAGE_SIZE - COMMITS_PREFETCH_THRESHOLD - 1
    );

    let next_page = (COMMITS_PAGE_SIZE..COMMITS_PAGE_SIZE * 2)
        .map(|index| commit_entry(&format!("{index:07x}"), &format!("commit {index}")))
        .collect::<Vec<_>>();
    let epoch = state.commits.pagination_epoch;
    update(
        &mut state,
        Action::GitResult(GitResult::CommitsPage {
            offset: COMMITS_PAGE_SIZE,
            limit: COMMITS_PAGE_SIZE,
            epoch,
            result: Ok(next_page),
        }),
    );

    assert_eq!(state.commits.items.len(), COMMITS_PAGE_SIZE * 2);
    assert_eq!(
        state.commits.selected,
        COMMITS_PAGE_SIZE - COMMITS_PREFETCH_THRESHOLD - 1
    );
    assert!(!state.commits.loading_more);
}

#[test]
fn commits_prefetch_pending_advances_when_user_reaches_loaded_tail() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        last_left_focus: PanelFocus::Commits,
        ..AppState::default()
    };
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: commit_entries(COMMITS_PAGE_SIZE),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );
    state.commits.selected = COMMITS_PAGE_SIZE - COMMITS_PREFETCH_THRESHOLD - 2;
    update(&mut state, Action::Ui(UiAction::MoveDown));
    state.commits.selected = COMMITS_PAGE_SIZE - 1;

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));

    assert!(commands.is_empty());
    assert!(state.commits.loading_more);
    assert!(state.commits.pending_select_after_load);

    let next_page = (COMMITS_PAGE_SIZE..COMMITS_PAGE_SIZE * 2)
        .map(|index| commit_entry(&format!("{index:07x}"), &format!("commit {index}")))
        .collect::<Vec<_>>();
    let epoch = state.commits.pagination_epoch;
    update(
        &mut state,
        Action::GitResult(GitResult::CommitsPage {
            offset: COMMITS_PAGE_SIZE,
            limit: COMMITS_PAGE_SIZE,
            epoch,
            result: Ok(next_page),
        }),
    );

    assert_eq!(state.commits.selected, COMMITS_PAGE_SIZE);
    assert!(!state.commits.pending_select_after_load);
}

#[test]
fn short_commit_page_stops_incremental_loading() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        last_left_focus: PanelFocus::Commits,
        ..AppState::default()
    };
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: commit_entries(COMMITS_PAGE_SIZE),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );
    state.commits.selected = COMMITS_PAGE_SIZE - 1;
    update(&mut state, Action::Ui(UiAction::MoveDown));

    let epoch = state.commits.pagination_epoch;
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::CommitsPage {
            offset: COMMITS_PAGE_SIZE,
            limit: COMMITS_PAGE_SIZE,
            epoch,
            result: Ok(vec![commit_entry("page101", "last commit")]),
        }),
    );

    assert!(commands.is_empty());
    assert_eq!(state.commits.items.len(), COMMITS_PAGE_SIZE + 1);
    assert!(!state.commits.has_more);
    state.commits.selected = COMMITS_PAGE_SIZE;
    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    assert!(commands.is_empty());
}

#[test]
fn branch_create_input_confirms_from_selected_start_point() {
    let mut state = state_with_branches_and_files(Vec::new());
    state.branches.selected = 1;

    assert!(update(&mut state, Action::Ui(UiAction::OpenBranchCreateInput)).is_empty());
    for ch in "feature/new".chars() {
        update(&mut state, Action::Ui(UiAction::BranchCreateInputChar(ch)));
    }
    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchCreate));

    assert_eq!(
        commands,
        vec![Command::CreateBranch {
            name: "feature/new".to_string(),
            start_point: "feature/mvp".to_string(),
        }]
    );
    assert!(!state.branches.create.active);
}

#[test]
fn branch_create_input_rejects_empty_name_and_can_cancel() {
    let mut state = state_with_branches_and_files(Vec::new());

    update(&mut state, Action::Ui(UiAction::OpenBranchCreateInput));
    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchCreate));
    assert!(commands.is_empty());
    assert!(state.branches.create.active);
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.contains("Branch name cannot be empty"))
    );

    update(&mut state, Action::Ui(UiAction::CancelBranchCreate));
    assert!(!state.branches.create.active);
}

#[test]
fn dirty_checkout_opens_auto_stash_confirm_then_confirms_command() {
    let mut state = state_with_branches_and_files(vec![FileEntry {
        path: "dirty.txt".to_string(),
        staged: false,
        untracked: false,
    }]);
    state.branches.selected = 1;

    let commands = update(&mut state, Action::Ui(UiAction::CheckoutSelectedBranch));
    assert!(commands.is_empty());
    assert_eq!(
        state.branches.auto_stash_confirm.operation,
        Some(AutoStashOperation::Checkout {
            branch: "feature/mvp".to_string()
        })
    );

    let commands = update(&mut state, Action::Ui(UiAction::ConfirmAutoStash));
    assert_eq!(
        commands,
        vec![Command::CheckoutBranch {
            name: "feature/mvp".to_string(),
            auto_stash: true,
        }]
    );
}

#[test]
fn dirty_checkout_auto_stash_can_cancel() {
    let mut state = state_with_branches_and_files(vec![FileEntry {
        path: "dirty.txt".to_string(),
        staged: false,
        untracked: false,
    }]);
    state.branches.selected = 1;

    update(&mut state, Action::Ui(UiAction::CheckoutSelectedBranch));
    let commands = update(&mut state, Action::Ui(UiAction::CancelAutoStash));

    assert!(commands.is_empty());
    assert!(!state.branches.auto_stash_confirm.active);
}

#[test]
fn branch_delete_menu_blocks_current_local_delete() {
    let mut state = state_with_branches_and_files(Vec::new());

    update(&mut state, Action::Ui(UiAction::OpenBranchDeleteMenu));
    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchDeleteMenu));

    assert!(commands.is_empty());
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.contains("Cannot delete current branch"))
    );
}

#[test]
fn branch_delete_menu_selects_mode_and_emits_command() {
    let mut state = state_with_branches_and_files(Vec::new());
    state.branches.selected = 1;

    update(&mut state, Action::Ui(UiAction::OpenBranchDeleteMenu));
    update(&mut state, Action::Ui(UiAction::MoveBranchDeleteMenuDown));
    assert_eq!(
        state.branches.delete_menu.selected,
        BranchDeleteChoice::Remote
    );
    update(&mut state, Action::Ui(UiAction::MoveBranchDeleteMenuDown));
    assert_eq!(
        state.branches.delete_menu.selected,
        BranchDeleteChoice::Both
    );
    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchDeleteMenu));

    assert_eq!(
        commands,
        vec![Command::DeleteBranch {
            name: "feature/mvp".to_string(),
            mode: BranchDeleteMode::Both,
            force: false,
        }]
    );
}

#[test]
fn unmerged_branch_delete_opens_force_confirm_and_can_force_delete() {
    let mut state = state_with_branches_and_files(Vec::new());

    let commands = update(
        &mut state,
        Action::GitResult(GitResult::DeleteBranch {
            name: "feature/mvp".to_string(),
            mode: BranchDeleteMode::Local,
            force: false,
            result: Err("error: The branch 'feature/mvp' is not fully merged.".to_string()),
        }),
    );

    assert!(commands.is_empty());
    assert!(state.branches.force_delete_confirm.active);
    assert_eq!(state.work.operation_pending, None);
    assert_eq!(
        state.branches.force_delete_confirm.target_branch,
        "feature/mvp"
    );

    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchForceDelete));
    assert_eq!(
        commands,
        vec![Command::DeleteBranch {
            name: "feature/mvp".to_string(),
            mode: BranchDeleteMode::Local,
            force: true,
        }]
    );
}

#[test]
fn force_delete_confirm_can_cancel() {
    let mut state = state_with_branches_and_files(Vec::new());

    update(
        &mut state,
        Action::GitResult(GitResult::DeleteBranch {
            name: "feature/mvp".to_string(),
            mode: BranchDeleteMode::Local,
            force: false,
            result: Err("not fully merged".to_string()),
        }),
    );
    let commands = update(&mut state, Action::Ui(UiAction::CancelBranchForceDelete));

    assert!(commands.is_empty());
    assert!(!state.branches.force_delete_confirm.active);
}

#[test]
fn branch_rebase_menu_selects_mode_and_dirty_rebase_confirms_auto_stash() {
    let mut state = state_with_branches_and_files(vec![FileEntry {
        path: "dirty.txt".to_string(),
        staged: false,
        untracked: false,
    }]);
    state.branches.selected = 1;

    update(&mut state, Action::Ui(UiAction::OpenBranchRebaseMenu));
    update(&mut state, Action::Ui(UiAction::MoveBranchRebaseMenuDown));
    assert_eq!(
        state.branches.rebase_menu.selected,
        BranchRebaseChoice::Interactive
    );
    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchRebaseMenu));

    assert!(commands.is_empty());
    assert_eq!(
        state.branches.auto_stash_confirm.operation,
        Some(AutoStashOperation::Rebase {
            target: "feature/mvp".to_string(),
            interactive: true,
        })
    );

    let commands = update(&mut state, Action::Ui(UiAction::ConfirmAutoStash));
    assert_eq!(
        commands,
        vec![Command::RebaseBranch {
            target: "feature/mvp".to_string(),
            interactive: true,
            auto_stash: true,
        }]
    );
}

#[test]
fn branch_rebase_origin_main_emits_fixed_target() {
    let mut state = state_with_branches_and_files(Vec::new());
    state.branches.selected = 1;

    update(&mut state, Action::Ui(UiAction::OpenBranchRebaseMenu));
    update(&mut state, Action::Ui(UiAction::MoveBranchRebaseMenuDown));
    update(&mut state, Action::Ui(UiAction::MoveBranchRebaseMenuDown));
    let commands = update(&mut state, Action::Ui(UiAction::ConfirmBranchRebaseMenu));

    assert_eq!(
        commands,
        vec![Command::RebaseBranch {
            target: "origin/main".to_string(),
            interactive: false,
            auto_stash: false,
        }]
    );
}

fn assert_details_refresh_for_paths(commands: Vec<Command>, expected_paths: Vec<String>) {
    assert_eq!(
        commands,
        vec![Command::RefreshFilesDetailsDiff {
            paths: expected_paths
        }]
    );
}

fn assert_branch_log_refresh(commands: Vec<Command>, expected_branch: &str) {
    assert_eq!(
        commands,
        vec![Command::RefreshBranchDetailsLog {
            branch: expected_branch.to_string(),
            max_count: ratagit_core::BRANCH_DETAILS_LOG_MAX_COUNT,
        }]
    );
}

fn state_with_branches_and_files(files: Vec<FileEntry>) -> AppState {
    let mut state = AppState {
        focus: PanelFocus::Branches,
        last_left_focus: PanelFocus::Branches,
        ..AppState::default()
    };
    state.branches.items = vec![
        BranchEntry {
            name: "main".to_string(),
            is_current: true,
        },
        BranchEntry {
            name: "feature/mvp".to_string(),
            is_current: false,
        },
    ];
    state.files.items = files;
    state
}

#[test]
fn branch_focus_requests_selected_branch_log_graph() {
    let mut state = state_with_branches_and_files(Vec::new());
    state.focus = PanelFocus::Files;
    state.last_left_focus = PanelFocus::Files;

    let commands = update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Branches,
        }),
    );

    assert_branch_log_refresh(commands, "main");
    assert_eq!(state.details.branch_log_target, Some("main".to_string()));
    assert!(state.work.details_pending);
}

#[test]
fn branch_selection_navigation_requests_selected_branch_log_graph() {
    let mut state = state_with_branches_and_files(Vec::new());

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));

    assert_branch_log_refresh(commands, "feature/mvp");
    assert_eq!(
        state.details.branch_log_target,
        Some("feature/mvp".to_string())
    );
}

#[test]
fn branch_log_cache_serves_repeated_selection_without_git_command() {
    let mut state = state_with_branches_and_files(Vec::new());

    update(
        &mut state,
        Action::GitResult(GitResult::BranchDetailsLog {
            branch: "main".to_string(),
            result: Ok("\u{1b}[33m*\u{1b}[m commit abc1234".to_string()),
        }),
    );
    assert_eq!(state.details.cached_branch_logs.len(), 1);

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    assert_branch_log_refresh(commands, "feature/mvp");

    let commands = update(&mut state, Action::Ui(UiAction::MoveUp));
    assert!(commands.is_empty());
    assert_eq!(
        state.details.branch_log,
        "\u{1b}[33m*\u{1b}[m commit abc1234"
    );
    assert!(!state.work.details_pending);
}

#[test]
fn stale_branch_log_result_is_ignored() {
    let mut state = state_with_branches_and_files(Vec::new());
    state.branches.selected = 1;
    state.details.branch_log_target = Some("feature/mvp".to_string());
    state.work.details_pending = true;

    let commands = update(
        &mut state,
        Action::GitResult(GitResult::BranchDetailsLog {
            branch: "main".to_string(),
            result: Ok("stale".to_string()),
        }),
    );

    assert!(commands.is_empty());
    assert!(state.details.branch_log.is_empty());
    assert!(state.work.details_pending);
}

#[test]
fn stage_selected_file_emits_stage_command() {
    let mut state = AppState {
        focus: PanelFocus::Files,
        ..AppState::default()
    };
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    state.files.selected = 1;
    let commands = update(&mut state, Action::Ui(UiAction::StageSelectedFile));
    assert_eq!(
        commands,
        vec![Command::StageFiles {
            paths: vec!["b.txt".to_string()]
        }]
    );
}

#[test]
fn toggle_selected_file_stage_stages_only_unstaged_directory_targets() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "dirty".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: vec![
                FileEntry {
                    path: "src/main.rs".to_string(),
                    staged: true,
                    untracked: false,
                },
                FileEntry {
                    path: "src/lib.rs".to_string(),
                    staged: false,
                    untracked: false,
                },
            ],
            commits: Vec::new(),
            branches: vec![BranchEntry {
                name: "main".to_string(),
                is_current: true,
            }],
            stashes: Vec::new(),
        })),
    );
    assert_details_refresh_for_paths(
        commands,
        vec!["src/lib.rs".to_string(), "src/main.rs".to_string()],
    );
    state.files.selected = 0;
    let commands = update(&mut state, Action::Ui(UiAction::ToggleSelectedFileStage));
    assert_eq!(
        commands,
        vec![Command::StageFiles {
            paths: vec!["src/lib.rs".to_string()]
        }]
    );
}

#[test]
fn v_visual_mode_extends_range_with_jk_movement() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "dirty".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: vec![
                FileEntry {
                    path: "a.txt".to_string(),
                    staged: false,
                    untracked: false,
                },
                FileEntry {
                    path: "b.txt".to_string(),
                    staged: false,
                    untracked: false,
                },
            ],
            commits: Vec::new(),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );
    assert_details_refresh_for_paths(commands, vec!["a.txt".to_string()]);

    update(&mut state, Action::Ui(UiAction::ToggleFilesMultiSelect));
    assert_eq!(state.files.selection_anchor, Some("a.txt".to_string()));
    assert_eq!(
        state
            .files
            .selected_rows
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["a.txt".to_string()]
    );
    assert_eq!(state.files.mode, FileInputMode::MultiSelect);

    update(&mut state, Action::Ui(UiAction::MoveDown));
    assert_eq!(
        state
            .files
            .selected_rows
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["a.txt".to_string(), "b.txt".to_string()]
    );

    update(&mut state, Action::Ui(UiAction::MoveUp));
    assert_eq!(
        state
            .files
            .selected_rows
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["a.txt".to_string()]
    );

    update(&mut state, Action::Ui(UiAction::MoveDown));
    let commands = update(&mut state, Action::Ui(UiAction::ToggleSelectedFileStage));
    assert_eq!(
        commands,
        vec![Command::StageFiles {
            paths: vec!["a.txt".to_string(), "b.txt".to_string()]
        }]
    );
}

#[test]
fn commit_visual_mode_extends_range_and_squash_uses_selected_commits() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        last_left_focus: PanelFocus::Commits,
        ..AppState::default()
    };
    state.commits.items = vec![
        commit_entry("aaa1111", "head"),
        commit_entry("bbb2222", "middle"),
        commit_entry("ccc3333", "base"),
    ];

    update(&mut state, Action::Ui(UiAction::ToggleCommitsMultiSelect));
    assert_eq!(state.commits.mode, CommitInputMode::MultiSelect);
    update(&mut state, Action::Ui(UiAction::MoveDown));
    assert_eq!(
        state
            .commits
            .selected_rows
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        vec!["aaa1111-full".to_string(), "bbb2222-full".to_string()]
    );

    let commands = update(&mut state, Action::Ui(UiAction::SquashSelectedCommits));
    assert_eq!(
        commands,
        vec![Command::SquashCommits {
            commit_ids: vec!["aaa1111-full".to_string(), "bbb2222-full".to_string()]
        }]
    );
    assert_eq!(state.commits.mode, CommitInputMode::Normal);
}

#[test]
fn commit_rewrite_requires_clean_worktree() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        ..AppState::default()
    };
    state.commits.items = vec![commit_entry("aaa1111", "head")];
    state.files.items = vec![FileEntry {
        path: "dirty.txt".to_string(),
        staged: false,
        untracked: false,
    }];

    let commands = update(&mut state, Action::Ui(UiAction::DeleteSelectedCommits));

    assert!(commands.is_empty());
    assert!(
        state
            .notices
            .iter()
            .any(|notice| { notice.contains("Commit rewrite requires a clean working tree") })
    );
}

#[test]
fn commit_rewrite_blocks_pushed_or_merged_commits() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        ..AppState::default()
    };
    let mut commit = commit_entry("aaa1111", "already public");
    commit.hash_status = CommitHashStatus::Pushed;
    state.commits.items = vec![commit];

    let commands = update(&mut state, Action::Ui(UiAction::SquashSelectedCommits));

    assert!(commands.is_empty());
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.contains("only supports unpushed commits"))
    );
}

#[test]
fn commit_reword_reuses_commit_editor_modal_and_confirms_command() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        ..AppState::default()
    };
    let mut commit = commit_entry("aaa1111", "feat: old");
    commit.message = "feat: old\n\nbody line".to_string();
    state.commits.items = vec![commit];

    update(&mut state, Action::Ui(UiAction::OpenCommitRewordEditor));

    assert!(matches!(
        state.editor.kind,
        Some(EditorKind::Commit {
            ref message,
            ref body,
            intent: CommitEditorIntent::Reword { ref commit_id },
            ..
        }) if message == "feat: old" && body == "body line" && commit_id == "aaa1111-full"
    ));

    let commands = update(&mut state, Action::Ui(UiAction::EditorConfirm));
    assert_eq!(
        commands,
        vec![Command::RewordCommit {
            commit_id: "aaa1111-full".to_string(),
            message: "feat: old\n\nbody line".to_string(),
        }]
    );
}

#[test]
fn auto_stash_operation_failure_refreshes_after_possible_partial_mutation() {
    let mut state = AppState::default();

    let commands = update(
        &mut state,
        Action::GitResult(GitResult::CheckoutCommitDetached {
            commit_id: "abc1234".to_string(),
            auto_stash: true,
            result: Err("stash pop failed".to_string()),
        }),
    );

    assert_eq!(commands, vec![Command::RefreshAll]);
    assert!(state.work.refresh_pending);
    assert!(
        state
            .status
            .last_error
            .as_ref()
            .expect("error should be stored")
            .contains("stash pop failed")
    );
}

#[test]
fn detached_checkout_with_dirty_worktree_uses_auto_stash_confirmation() {
    let mut state = AppState {
        focus: PanelFocus::Commits,
        ..AppState::default()
    };
    state.commits.items = vec![commit_entry("aaa1111", "head")];
    state.files.items = vec![FileEntry {
        path: "dirty.txt".to_string(),
        staged: false,
        untracked: false,
    }];

    let commands = update(
        &mut state,
        Action::Ui(UiAction::CheckoutSelectedCommitDetached),
    );
    assert!(commands.is_empty());
    assert!(matches!(
        state.branches.auto_stash_confirm.operation,
        Some(AutoStashOperation::CheckoutCommitDetached { ref commit_id })
            if commit_id == "aaa1111-full"
    ));

    let commands = update(&mut state, Action::Ui(UiAction::ConfirmAutoStash));
    assert_eq!(
        commands,
        vec![Command::CheckoutCommitDetached {
            commit_id: "aaa1111-full".to_string(),
            auto_stash: true,
        }]
    );
}

#[test]
fn refreshed_snapshot_updates_state_and_clamps_indexes() {
    let mut state = AppState::default();
    state.files.selected = 99;
    let snapshot = RepoSnapshot {
        status_summary: "dirty".to_string(),
        current_branch: "main".to_string(),
        detached_head: false,
        files: vec![FileEntry {
            path: "only.txt".to_string(),
            staged: true,
            untracked: false,
        }],
        commits: vec![],
        branches: vec![],
        stashes: vec![],
    };
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(snapshot)),
    );
    assert_eq!(
        commands,
        vec![Command::RefreshFilesDetailsDiff {
            paths: vec!["only.txt".to_string()]
        }]
    );
    assert_eq!(state.status.summary, "dirty");
    assert_eq!(state.files.selected, 0);
    assert_eq!(state.status.refresh_count, 1);
}

#[test]
fn files_selection_navigation_requests_details_refresh() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: false,
            untracked: false,
        },
    ];

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    assert_details_refresh_for_paths(commands, vec!["b.txt".to_string()]);
}

#[test]
fn non_files_navigation_does_not_request_files_details_refresh() {
    let mut state = AppState {
        focus: PanelFocus::Branches,
        last_left_focus: PanelFocus::Branches,
        ..AppState::default()
    };

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    assert!(commands.is_empty());
}

#[test]
fn files_details_diff_result_updates_state() {
    let mut state = AppState::default();
    state.files.items = vec![FileEntry {
        path: "src/lib.rs".to_string(),
        staged: false,
        untracked: false,
    }];
    state.files.expanded_dirs.insert("src".to_string());
    refresh_tree_projection(&mut state.files);
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::FilesDetailsDiff {
            paths: vec!["src/lib.rs".to_string()],
            result: Ok("### unstaged\ndiff --git a/src/lib.rs b/src/lib.rs".to_string()),
        }),
    );
    assert!(commands.is_empty());
    assert_eq!(state.details.files_targets, vec!["src/lib.rs".to_string()]);
    assert!(state.details.files_error.is_none());
    assert!(state.details.files_diff.contains("diff --git"));
    assert_eq!(state.details.cached_files_diffs.len(), 1);
}

#[test]
fn details_scroll_actions_update_offset_and_clamp_to_content() {
    let mut state = AppState::default();
    state.details.files_diff = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6".to_string();

    assert!(
        update(
            &mut state,
            Action::Ui(UiAction::DetailsScrollDown {
                lines: 2,
                visible_lines: 2,
            })
        )
        .is_empty()
    );
    assert_eq!(state.details.scroll_offset, 2);
    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollDown {
            lines: 2,
            visible_lines: 2,
        }),
    );
    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollDown {
            lines: 2,
            visible_lines: 2,
        }),
    );
    assert_eq!(state.details.scroll_offset, 4);

    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollUp { lines: 2 }),
    );
    assert_eq!(state.details.scroll_offset, 2);
    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollUp { lines: 5 }),
    );
    assert_eq!(state.details.scroll_offset, 0);
}

#[test]
fn details_scroll_down_does_not_advance_past_last_visible_page() {
    let mut state = AppState::default();
    state.details.files_diff = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7".to_string();
    state.details.scroll_offset = 4;

    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollDown {
            lines: 3,
            visible_lines: 3,
        }),
    );
    assert_eq!(state.details.scroll_offset, 4);

    update(
        &mut state,
        Action::Ui(UiAction::DetailsScrollUp { lines: 3 }),
    );
    assert_eq!(state.details.scroll_offset, 1);
}

#[test]
fn details_scroll_resets_when_files_details_target_changes() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    refresh_tree_projection(&mut state.files);
    state.details.files_targets = vec!["a.txt".to_string()];
    state.details.files_diff = "a1\na2".to_string();
    state.details.scroll_offset = 1;

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));

    assert_details_refresh_for_paths(commands, vec!["b.txt".to_string()]);
    assert_eq!(state.details.scroll_offset, 0);
}

#[test]
fn files_details_diff_cache_serves_repeated_selection_without_git_command() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    refresh_tree_projection(&mut state.files);

    update(
        &mut state,
        Action::GitResult(GitResult::FilesDetailsDiff {
            paths: vec!["a.txt".to_string()],
            result: Ok("cached a diff".to_string()),
        }),
    );
    assert_eq!(state.details.cached_files_diffs.len(), 1);

    let commands = update(&mut state, Action::Ui(UiAction::MoveDown));
    assert_details_refresh_for_paths(commands, vec!["b.txt".to_string()]);
    assert!(state.work.details_pending);

    let commands = update(&mut state, Action::Ui(UiAction::MoveUp));
    assert!(commands.is_empty());
    assert_eq!(state.details.files_diff, "cached a diff");
    assert!(!state.work.details_pending);
}

#[test]
fn files_details_diff_cache_is_bounded_and_cleared_by_repo_changes() {
    let mut state = AppState::default();
    state.files.items = (0..18)
        .map(|index| FileEntry {
            path: format!("file-{index:02}.txt"),
            staged: false,
            untracked: false,
        })
        .collect();
    refresh_tree_projection(&mut state.files);

    for index in 0..18 {
        state.files.selected = index;
        let path = format!("file-{index:02}.txt");
        update(
            &mut state,
            Action::GitResult(GitResult::FilesDetailsDiff {
                paths: vec![path.clone()],
                result: Ok(format!("diff {path}")),
            }),
        );
    }

    assert_eq!(state.details.cached_files_diffs.len(), 16);
    assert!(
        !state
            .details
            .cached_files_diffs
            .iter()
            .any(|entry| entry.paths == vec!["file-00.txt".to_string()])
    );

    update(
        &mut state,
        Action::GitResult(GitResult::StageFiles {
            paths: vec!["file-17.txt".to_string()],
            result: Ok(()),
        }),
    );
    assert!(state.details.cached_files_diffs.is_empty());

    state
        .details
        .cached_files_diffs
        .push(ratagit_core::CachedFilesDiff {
            paths: vec!["file-17.txt".to_string()],
            diff: "diff".to_string(),
        });
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(RepoSnapshot {
            status_summary: "clean".to_string(),
            current_branch: "main".to_string(),
            detached_head: false,
            files: Vec::new(),
            commits: Vec::new(),
            branches: Vec::new(),
            stashes: Vec::new(),
        })),
    );
    assert!(state.details.cached_files_diffs.is_empty());
}

#[test]
fn stale_files_details_diff_result_is_ignored() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    refresh_tree_projection(&mut state.files);
    state.files.selected = 1;
    state.details.files_targets = vec!["b.txt".to_string()];
    state.work.details_pending = true;

    let commands = update(
        &mut state,
        Action::GitResult(GitResult::FilesDetailsDiff {
            paths: vec!["a.txt".to_string()],
            result: Ok("stale".to_string()),
        }),
    );

    assert!(commands.is_empty());
    assert!(state.details.files_diff.is_empty());
    assert!(state.work.details_pending);
}

#[test]
fn tree_projection_cache_tracks_rows_and_directory_descendants() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "src/lib.rs".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "src/main.rs".to_string(),
            staged: true,
            untracked: false,
        },
    ];
    state.files.expanded_dirs.insert("src".to_string());
    refresh_tree_projection(&mut state.files);

    assert_eq!(
        state
            .files
            .tree_rows
            .iter()
            .map(|row| row.path.as_str())
            .collect::<Vec<_>>(),
        vec!["src", "src/lib.rs", "src/main.rs"]
    );
    assert_eq!(
        state.files.row_descendants.get("src"),
        Some(&vec!["src/lib.rs".to_string(), "src/main.rs".to_string()])
    );
}

#[test]
fn failed_git_result_is_visible_in_state() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::CreateCommit {
            message: "wip".to_string(),
            result: Err("nothing staged".to_string()),
        }),
    );
    assert!(commands.is_empty());
    assert!(
        state
            .status
            .last_error
            .as_ref()
            .expect("error should be stored")
            .contains("nothing staged")
    );
}

#[test]
fn focus_next_and_prev_cycle_only_left_panels() {
    let mut state = AppState::default();
    assert_eq!(state.focus, PanelFocus::Files);
    assert_eq!(state.last_left_focus, PanelFocus::Files);

    update(&mut state, Action::Ui(UiAction::FocusNext));
    assert_eq!(state.focus, PanelFocus::Branches);
    assert_eq!(state.last_left_focus, PanelFocus::Branches);

    update(&mut state, Action::Ui(UiAction::FocusNext));
    assert_eq!(state.focus, PanelFocus::Commits);
    assert_eq!(state.last_left_focus, PanelFocus::Commits);

    update(&mut state, Action::Ui(UiAction::FocusPrev));
    assert_eq!(state.focus, PanelFocus::Branches);
    assert_eq!(state.last_left_focus, PanelFocus::Branches);
}

#[test]
fn focus_panel_allows_right_focus_and_preserves_last_left() {
    let mut state = AppState::default();
    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Stash,
        }),
    );
    assert_eq!(state.focus, PanelFocus::Stash);
    assert_eq!(state.last_left_focus, PanelFocus::Stash);

    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Details,
        }),
    );
    assert_eq!(state.focus, PanelFocus::Details);
    assert_eq!(state.last_left_focus, PanelFocus::Stash);
}

#[test]
fn move_selection_does_not_change_left_indexes_when_focus_is_right_panel() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    state.files.selected = 1;
    update(
        &mut state,
        Action::Ui(UiAction::FocusPanel {
            panel: PanelFocus::Details,
        }),
    );
    update(&mut state, Action::Ui(UiAction::MoveUp));
    assert_eq!(state.files.selected, 1);
}

#[test]
fn commit_editor_confirms_subject_and_multiline_body() {
    let mut state = AppState::default();
    assert!(update(&mut state, Action::Ui(UiAction::OpenCommitEditor)).is_empty());
    assert_eq!(
        state.editor.kind,
        Some(EditorKind::Commit {
            message: String::new(),
            message_cursor: 0,
            body: String::new(),
            body_cursor: 0,
            active_field: CommitField::Message,
            intent: CommitEditorIntent::Create,
        })
    );

    for ch in "feat: ship".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorNextField));
    for ch in "line one".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorInsertNewline));
    for ch in "line two".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let commands = update(&mut state, Action::Ui(UiAction::EditorConfirm));
    assert_eq!(
        commands,
        vec![Command::CreateCommit {
            message: "feat: ship\n\nline one\nline two".to_string()
        }]
    );
    assert_eq!(state.commits.draft_message, "feat: ship");
    assert!(state.editor.kind.is_none());
}

#[test]
fn commit_editor_blocks_empty_subject_and_keeps_editor_open() {
    let mut state = AppState::default();
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    update(&mut state, Action::Ui(UiAction::EditorNextField));
    update(&mut state, Action::Ui(UiAction::EditorInputChar('x')));

    let commands = update(&mut state, Action::Ui(UiAction::EditorConfirm));
    assert!(commands.is_empty());
    assert!(matches!(
        state.editor.kind,
        Some(EditorKind::Commit {
            active_field: CommitField::Body,
            ..
        })
    ));
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.contains("Commit message cannot be empty"))
    );
}

#[test]
fn stash_editor_confirms_all_scope_outside_multiselect() {
    let mut state = AppState::default();
    update(&mut state, Action::Ui(UiAction::OpenStashEditor));
    assert_eq!(
        state.editor.kind,
        Some(EditorKind::Stash {
            title: String::new(),
            title_cursor: 0,
            scope: StashScope::All,
        })
    );
    for ch in "checkpoint".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let commands = update(&mut state, Action::Ui(UiAction::EditorConfirm));
    assert_eq!(
        commands,
        vec![Command::StashPush {
            message: "checkpoint".to_string()
        }]
    );
    assert!(state.editor.kind.is_none());
}

#[test]
fn stash_editor_confirms_selected_paths_scope_in_multiselect_mode() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: false,
            untracked: false,
        },
    ];
    state.files.mode = FileInputMode::MultiSelect;
    state.files.selected_rows.insert("a.txt".to_string());
    update(&mut state, Action::Ui(UiAction::OpenStashEditor));
    assert_eq!(
        state.editor.kind,
        Some(EditorKind::Stash {
            title: String::new(),
            title_cursor: 0,
            scope: StashScope::SelectedPaths(vec!["a.txt".to_string()]),
        })
    );
    for ch in "pick".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }

    let commands = update(&mut state, Action::Ui(UiAction::EditorConfirm));
    assert_eq!(
        commands,
        vec![Command::StashFiles {
            message: "pick".to_string(),
            paths: vec!["a.txt".to_string()],
        }]
    );
    assert!(state.editor.kind.is_none());
}

#[test]
fn reset_menu_opens_moves_and_cancels() {
    let mut state = AppState::default();
    assert!(update(&mut state, Action::Ui(UiAction::OpenResetMenu)).is_empty());
    assert!(state.reset_menu.active);
    assert_eq!(state.reset_menu.selected, ResetChoice::Mixed);

    update(&mut state, Action::Ui(UiAction::MoveResetMenuDown));
    assert_eq!(state.reset_menu.selected, ResetChoice::Soft);
    update(&mut state, Action::Ui(UiAction::MoveResetMenuDown));
    assert_eq!(state.reset_menu.selected, ResetChoice::Hard);
    update(&mut state, Action::Ui(UiAction::MoveResetMenuDown));
    assert_eq!(state.reset_menu.selected, ResetChoice::Nuke);
    update(&mut state, Action::Ui(UiAction::MoveResetMenuDown));
    assert_eq!(state.reset_menu.selected, ResetChoice::Nuke);
    update(&mut state, Action::Ui(UiAction::MoveResetMenuUp));
    assert_eq!(state.reset_menu.selected, ResetChoice::Hard);

    assert!(update(&mut state, Action::Ui(UiAction::CancelResetMenu)).is_empty());
    assert!(!state.reset_menu.active);
}

#[test]
fn reset_menu_confirm_emits_selected_reset_command() {
    let cases = [
        (
            ResetChoice::Mixed,
            Command::Reset {
                mode: ResetMode::Mixed,
            },
        ),
        (
            ResetChoice::Soft,
            Command::Reset {
                mode: ResetMode::Soft,
            },
        ),
        (
            ResetChoice::Hard,
            Command::Reset {
                mode: ResetMode::Hard,
            },
        ),
        (ResetChoice::Nuke, Command::Nuke),
    ];

    for (choice, expected_command) in cases {
        let mut state = AppState::default();
        update(&mut state, Action::Ui(UiAction::OpenResetMenu));
        state.reset_menu.selected = choice;

        let commands = update(&mut state, Action::Ui(UiAction::ConfirmResetMenu));

        assert_eq!(commands, vec![expected_command]);
        assert!(!state.reset_menu.active);
    }
}

#[test]
fn reset_git_result_reports_success_and_failure() {
    let mut state = AppState::default();
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Reset {
            mode: ResetMode::Mixed,
            result: Ok(()),
        }),
    );
    assert_eq!(commands, vec![Command::RefreshAll]);
    assert_eq!(state.last_operation, Some("reset_mixed".to_string()));
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.contains("Reset mixed to HEAD"))
    );

    let commands = update(
        &mut state,
        Action::GitResult(GitResult::Nuke {
            result: Err("blocked".to_string()),
        }),
    );
    assert!(commands.is_empty());
    assert_eq!(state.last_operation, Some("nuke".to_string()));
    assert!(
        state
            .status
            .last_error
            .as_ref()
            .is_some_and(|error| error.contains("blocked"))
    );
}

#[test]
fn discard_confirm_opens_for_current_file_and_confirms_command() {
    let mut state = AppState::default();
    state.files.items = vec![FileEntry {
        path: "a.txt".to_string(),
        staged: false,
        untracked: false,
    }];

    assert!(update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm)).is_empty());
    assert!(state.discard_confirm.active);
    assert_eq!(state.discard_confirm.paths, vec!["a.txt".to_string()]);

    let commands = update(&mut state, Action::Ui(UiAction::ConfirmDiscard));

    assert_eq!(
        commands,
        vec![Command::DiscardFiles {
            paths: vec!["a.txt".to_string()]
        }]
    );
    assert!(!state.discard_confirm.active);
    assert!(state.discard_confirm.paths.is_empty());
}

#[test]
fn discard_confirm_uses_visual_selected_targets_and_can_cancel() {
    let mut state = AppState::default();
    state.files.items = vec![
        FileEntry {
            path: "a.txt".to_string(),
            staged: false,
            untracked: false,
        },
        FileEntry {
            path: "b.txt".to_string(),
            staged: true,
            untracked: false,
        },
    ];
    state.files.mode = FileInputMode::MultiSelect;
    state.files.selected_rows.insert("a.txt".to_string());
    state.files.selected_rows.insert("b.txt".to_string());

    update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));
    assert_eq!(
        state.discard_confirm.paths,
        vec!["a.txt".to_string(), "b.txt".to_string()]
    );

    assert!(update(&mut state, Action::Ui(UiAction::CancelDiscard)).is_empty());
    assert!(!state.discard_confirm.active);
    assert!(state.discard_confirm.paths.is_empty());
}

#[test]
fn discard_confirm_without_selection_reports_notice() {
    let mut state = AppState::default();

    let commands = update(&mut state, Action::Ui(UiAction::OpenDiscardConfirm));

    assert!(commands.is_empty());
    assert!(!state.discard_confirm.active);
    assert!(
        state
            .notices
            .iter()
            .any(|notice| notice.contains("No file selected"))
    );
}

#[test]
fn commit_editor_edits_subject_at_cursor() {
    let mut state = AppState::default();
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    for ch in "feat ship".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    for _ in 0..5 {
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorLeft));
    }
    update(&mut state, Action::Ui(UiAction::EditorInputChar(':')));
    update(&mut state, Action::Ui(UiAction::EditorMoveCursorHome));
    update(&mut state, Action::Ui(UiAction::EditorBackspace));
    update(&mut state, Action::Ui(UiAction::EditorMoveCursorEnd));
    update(&mut state, Action::Ui(UiAction::EditorInputChar('!')));

    assert_eq!(
        state.editor.kind,
        Some(EditorKind::Commit {
            message: "feat: ship!".to_string(),
            message_cursor: "feat: ship!".len(),
            body: String::new(),
            body_cursor: 0,
            active_field: CommitField::Message,
            intent: CommitEditorIntent::Create,
        })
    );
}

#[test]
fn commit_editor_edits_multiline_body_at_cursor() {
    let mut state = AppState::default();
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    update(&mut state, Action::Ui(UiAction::EditorNextField));
    for ch in "ab".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorMoveCursorLeft));
    update(&mut state, Action::Ui(UiAction::EditorInsertNewline));

    assert!(matches!(
        state.editor.kind,
        Some(EditorKind::Commit {
            body,
            body_cursor: 2,
            active_field: CommitField::Body,
            ..
        }) if body == "a\nb"
    ));
}

#[test]
fn stash_editor_edits_title_at_cursor() {
    let mut state = AppState::default();
    update(&mut state, Action::Ui(UiAction::OpenStashEditor));
    for ch in "save point".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    for _ in 0..5 {
        update(&mut state, Action::Ui(UiAction::EditorMoveCursorLeft));
    }
    update(&mut state, Action::Ui(UiAction::EditorBackspace));
    update(&mut state, Action::Ui(UiAction::EditorInputChar('-')));

    assert_eq!(
        state.editor.kind,
        Some(EditorKind::Stash {
            title: "save-point".to_string(),
            title_cursor: "save-".len(),
            scope: StashScope::All,
        })
    );
}

#[test]
fn editor_cursor_respects_unicode_boundaries() {
    let mut state = AppState::default();
    update(&mut state, Action::Ui(UiAction::OpenCommitEditor));
    for ch in "修复".chars() {
        update(&mut state, Action::Ui(UiAction::EditorInputChar(ch)));
    }
    update(&mut state, Action::Ui(UiAction::EditorMoveCursorLeft));
    update(&mut state, Action::Ui(UiAction::EditorBackspace));
    update(&mut state, Action::Ui(UiAction::EditorInputChar('改')));

    assert_eq!(
        state.editor.kind,
        Some(EditorKind::Commit {
            message: "改复".to_string(),
            message_cursor: "改".len(),
            body: String::new(),
            body_cursor: 0,
            active_field: CommitField::Message,
            intent: CommitEditorIntent::Create,
        })
    );
}
