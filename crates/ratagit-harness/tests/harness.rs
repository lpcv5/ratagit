use ratagit_core::UiAction;
use ratagit_harness::run_mock_scenario;
use ratagit_testkit::{fixture_dirty_repo, fixture_empty_repo};

#[test]
fn harness_status_refresh() {
    let result = run_mock_scenario(
        "mvp_status_refresh",
        fixture_empty_repo(),
        &[UiAction::RefreshAll],
        &["summary=staged: 0, unstaged: 0", "[Details]"],
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
        &["stage:src/lib.rs", "unstage:src/lib.rs"],
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
        &["[Details]", "[Log]", "keys(branches):", "focus=Branches"],
        &["refresh"],
    );
    assert!(result.is_ok(), "{result:?}");
}
