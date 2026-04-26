use ratagit_core::UiAction;
use ratagit_harness::{MockScenario, ScenarioExpectations, run_mock_scenario};
use ratagit_testkit::{fixture_dirty_repo, fixture_empty_repo, fixture_many_files};

fn assert_scenario(scenario: MockScenario<'_>) {
    let result = run_mock_scenario(scenario);
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn harness_status_refresh() {
    let inputs = [UiAction::RefreshAll];
    assert_scenario(MockScenario::new(
        "mvp_status_refresh",
        fixture_empty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Files", "Details", "keys(files):"],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_files_stage_and_unstage() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::StageSelectedFile,
        UiAction::UnstageSelectedFile,
    ];
    assert_scenario(MockScenario::new(
        "mvp_files_stage_unstage",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Files", "src/lib.rs", "staged=no"],
            git_ops_contains: &["stage-files:src/lib.rs", "unstage-files:src/lib.rs"],
            git_state_contains: &["path: \"src/lib.rs\"", "staged: false"],
        },
    ));
}

#[test]
fn harness_files_tree_expand_collapse() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::ToggleSelectedDirectory,
    ];
    assert_scenario(MockScenario::new(
        "files_tree_expand_collapse",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["[+]", "src/"],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"src/main.rs\""],
        },
    ));
}

#[test]
fn harness_files_space_toggles_directory_stage() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::MoveDown,
        UiAction::ToggleSelectedFileStage,
    ];
    assert_scenario(MockScenario::new(
        "files_space_toggles_directory_stage",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Staged src/lib.rs", "staged=yes"],
            git_ops_contains: &["stage-files:src/lib.rs"],
            git_state_contains: &["path: \"src/lib.rs\"", "staged: true"],
        },
    ));
}

#[test]
fn harness_files_multi_select_stashes_selected_targets() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::ToggleFilesMultiSelect,
        UiAction::MoveDown,
        UiAction::ToggleCurrentFileSelection,
        UiAction::StashSelectedFiles,
    ];
    assert_scenario(MockScenario::new(
        "files_multi_select_stash",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Stashed 3 files", "savepoint"],
            git_ops_contains: &["stash-files:savepoint:README.md,src/lib.rs,src/main.rs"],
            git_state_contains: &["summary: \"savepoint\""],
        },
    ));
}

#[test]
fn harness_files_search_jumps_and_clears() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::StartFileSearch,
        UiAction::InputFileSearchChar('l'),
        UiAction::InputFileSearchChar('i'),
        UiAction::ConfirmFileSearch,
        UiAction::NextFileSearchMatch,
        UiAction::PrevFileSearchMatch,
        UiAction::CancelFileSearch,
    ];
    assert_scenario(MockScenario::new(
        "files_search_jumps_and_clears",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["src/lib.rs", "keys(files):"],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_files_scroll_keeps_selection_visible() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 20))
        .collect::<Vec<_>>();
    assert_scenario(MockScenario::new(
        "files_scroll_keeps_selection_visible",
        fixture_many_files(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "    [S] file-18.txt",
                "    [ ] file-19.txt",
                ">   [S] file-20.txt",
                "    [ ] file-23.txt",
            ],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"file-20.txt\""],
        },
    ));
}

#[test]
fn harness_files_scroll_up_uses_top_reserve() {
    let inputs = std::iter::once(UiAction::RefreshAll)
        .chain(std::iter::repeat_n(UiAction::MoveDown, 25))
        .chain(std::iter::repeat_n(UiAction::MoveUp, 5))
        .collect::<Vec<_>>();
    assert_scenario(MockScenario::new(
        "files_scroll_up_uses_top_reserve",
        fixture_many_files(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &[
                "    [ ] file-17.txt",
                ">   [S] file-20.txt",
                "    [S] file-22.txt",
            ],
            git_ops_contains: &["refresh"],
            git_state_contains: &["path: \"file-24.txt\""],
        },
    ));
}

#[test]
fn harness_commits_create_and_refresh() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::CreateCommit {
            message: "mvp commit".to_string(),
        },
    ];
    assert_scenario(MockScenario::new(
        "mvp_commits_create_refresh",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["mvp commit", "Commits *"],
            git_ops_contains: &["commit:mvp commit", "refresh"],
            git_state_contains: &["summary: \"mvp commit\""],
        },
    ));
}

#[test]
fn harness_branches_create_and_checkout() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::CreateBranch {
            name: "feature/new".to_string(),
        },
        UiAction::MoveDown,
        UiAction::MoveDown,
        UiAction::CheckoutSelectedBranch,
    ];
    assert_scenario(MockScenario::new(
        "mvp_branches_create_checkout",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["feature/new", "is_current=yes"],
            git_ops_contains: &["create-branch:feature/new", "checkout-branch:feature/new"],
            git_state_contains: &["current_branch: \"feature/new\""],
        },
    ));
}

#[test]
fn harness_stash_push_and_pop() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusPrev,
        UiAction::StashPush {
            message: "savepoint".to_string(),
        },
        UiAction::StashPopSelected,
    ];
    assert_scenario(MockScenario::new(
        "mvp_stash_push_pop",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Stash", "WIP on main: local test"],
            git_ops_contains: &["stash-push:savepoint", "stash-pop:stash@{0}"],
            git_state_contains: &["summary: \"WIP on main: local test\""],
        },
    ));
}

#[test]
fn harness_error_visible_without_crash() {
    let inputs = [
        UiAction::RefreshAll,
        UiAction::FocusNext,
        UiAction::FocusNext,
        UiAction::CreateCommit {
            message: String::new(),
        },
    ];
    assert_scenario(MockScenario::new(
        "mvp_error_visible_non_crash",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["error=Failed to create commit"],
            git_ops_contains: &["commit:"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}

#[test]
fn harness_focus_panel_shortcuts_follow_focus() {
    let inputs = [
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
    ];
    assert_scenario(MockScenario::new(
        "mvp_focus_panel_shortcuts_follow_focus",
        fixture_dirty_repo(),
        &inputs,
        ScenarioExpectations {
            screen_contains: &["Details", "Log", "keys(branches):", "o checkout"],
            git_ops_contains: &["refresh"],
            git_state_contains: &["current_branch: \"main\""],
        },
    ));
}
