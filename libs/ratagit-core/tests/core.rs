use ratagit_core::{
    Action, AppState, BranchEntry, Command, FileEntry, FileInputMode, GitResult, PanelFocus,
    RepoSnapshot, UiAction, update,
};

#[test]
fn refresh_action_emits_refresh_command() {
    let mut state = AppState::default();
    let commands = update(&mut state, Action::Ui(UiAction::RefreshAll));
    assert_eq!(commands, vec![Command::RefreshAll]);
}

fn assert_details_refresh_for_paths(commands: Vec<Command>, expected_paths: Vec<String>) {
    assert_eq!(
        commands,
        vec![Command::RefreshFilesDetailsDiff {
            paths: expected_paths
        }]
    );
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
