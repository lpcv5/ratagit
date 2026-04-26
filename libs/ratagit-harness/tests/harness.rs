use ratagit_core::UiAction;
use ratagit_harness::run_mock_scenario;
use ratagit_testkit::{fixture_dirty_repo, fixture_empty_repo, fixture_many_files};

#[test]
fn harness_status_refresh() {
    let result = run_mock_scenario(
        "mvp_status_refresh",
        fixture_empty_repo(),
        &[UiAction::RefreshAll],
        &["[Files]", "[Details]"],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_stage_and_unstage() {
    let result = run_mock_scenario(
        "mvp_files_stage_unstage",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::MoveDown,
            UiAction::StageSelectedFile,
            UiAction::UnstageSelectedFile,
        ],
        &["[Files]", "src/lib.rs"],
        &["stage-files:src/lib.rs", "unstage-files:src/lib.rs"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_tree_expand_collapse() {
    let result = run_mock_scenario(
        "files_tree_expand_collapse",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::MoveDown,
            UiAction::ToggleSelectedDirectory,
        ],
        &["[+]", "src/"],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_space_toggles_directory_stage() {
    let result = run_mock_scenario(
        "files_space_toggles_directory_stage",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::MoveDown,
            UiAction::ToggleSelectedFileStage,
        ],
        &["Staged src/lib.rs"],
        &["stage-files:src/lib.rs"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_multi_select_stashes_selected_targets() {
    let result = run_mock_scenario(
        "files_multi_select_stash",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::ToggleFilesMultiSelect,
            UiAction::MoveDown,
            UiAction::ToggleCurrentFileSelection,
            UiAction::StashSelectedFiles,
        ],
        &["Stashed 3 files"],
        &["stash-files:savepoint:README.md,src/lib.rs,src/main.rs"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_search_jumps_and_clears() {
    let result = run_mock_scenario(
        "files_search_jumps_and_clears",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::StartFileSearch,
            UiAction::InputFileSearchChar('l'),
            UiAction::InputFileSearchChar('i'),
            UiAction::ConfirmFileSearch,
            UiAction::NextFileSearchMatch,
            UiAction::PrevFileSearchMatch,
            UiAction::CancelFileSearch,
        ],
        &["src/lib.rs", "keys(files):"],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_scroll_keeps_selection_visible() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 20))
        .collect::<Vec<_>>();
    let result = run_mock_scenario(
        "files_scroll_keeps_selection_visible",
        fixture_many_files(),
        &inputs,
        &[
            "    [S] file-16.txt",
            "    [ ] file-17.txt",
            ">   [S] file-20.txt",
            "    [ ] file-23.txt",
        ],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_files_scroll_up_uses_top_reserve() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 25))
        .chain(std::iter::repeat_n(UiAction::MoveUp, 5))
        .collect::<Vec<_>>();
    let result = run_mock_scenario(
        "files_scroll_up_uses_top_reserve",
        fixture_many_files(),
        &inputs,
        &[
            "    [ ] file-17.txt",
            ">   [S] file-20.txt",
            "    [S] file-24.txt",
        ],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_commits_create_and_refresh() {
    let result = run_mock_scenario(
        "mvp_commits_create_refresh",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::FocusNext,
            UiAction::FocusNext,
            UiAction::CreateCommit {
                message: "mvp commit".to_string(),
            },
        ],
        &["mvp commit"],
        &["commit:mvp commit", "refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_branches_create_and_checkout() {
    let result = run_mock_scenario(
        "mvp_branches_create_checkout",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::FocusNext,
            UiAction::CreateBranch {
                name: "feature/new".to_string(),
            },
            UiAction::MoveDown,
            UiAction::MoveDown,
            UiAction::CheckoutSelectedBranch,
        ],
        &["feature/new"],
        &["create-branch:feature/new", "checkout-branch:feature/new"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_stash_push_and_pop() {
    let result = run_mock_scenario(
        "mvp_stash_push_pop",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::FocusPrev,
            UiAction::StashPush {
                message: "savepoint".to_string(),
            },
            UiAction::StashPopSelected,
        ],
        &["[Stash]"],
        &["stash-push:savepoint", "stash-pop:stash@{0}"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_error_visible_without_crash() {
    let result = run_mock_scenario(
        "mvp_error_visible_non_crash",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::FocusNext,
            UiAction::FocusNext,
            UiAction::CreateCommit {
                message: String::new(),
            },
        ],
        &["error=Failed to create commit"],
        &["commit:"],
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_focus_panel_shortcuts_follow_focus() {
    let result = run_mock_scenario(
        "mvp_focus_panel_shortcuts_follow_focus",
        fixture_dirty_repo(),
        &[
            UiAction::RefreshAll,
            UiAction::FocusPanel {
                panel: ratagit_core::PanelFocus::Details,
            },
            UiAction::FocusPanel {
                panel: ratagit_core::PanelFocus::Log,
            },
            UiAction::FocusPanel {
                panel: ratagit_core::PanelFocus::Branches,
            },
        ],
        &["[Details]", "[Log]", "keys(branches):", "o checkout"],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}
