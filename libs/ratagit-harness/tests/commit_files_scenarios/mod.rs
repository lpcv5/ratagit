use super::*;
use ratagit_core::workflow::{Dialog, Feature, WorkflowAction};

fn open(feature: Feature) -> UiAction {
    UiAction::Workflow(WorkflowAction::Open(feature))
}
fn confirm() -> UiAction {
    UiAction::Workflow(WorkflowAction::Confirm)
}
fn dispatch(runtime: &mut Runtime<MockGitBackend>, actions: &[UiAction]) {
    for action in actions {
        runtime.dispatch_ui(action.clone());
    }
}
fn runtime() -> Runtime<MockGitBackend> {
    let mut runtime = Runtime::new(
        AppContext::default(),
        MockGitBackend::new(clean_three_commit_fixture()),
        TerminalSize {
            width: 140,
            height: 40,
        },
    );
    dispatch(
        &mut runtime,
        &[
            UiAction::RefreshAll,
            UiAction::FocusPanel {
                panel: PanelFocus::Commits,
            },
            UiAction::OpenCommitFilesPanel,
        ],
    );
    runtime
}

pub(super) fn operations_and_hunks() {
    let mut runtime = runtime();
    let clean = runtime.backend().snapshot().clone();
    runtime.dispatch_ui(open(Feature::CommitFileCheckout));
    assert!(
        runtime
            .render_terminal_text()
            .contains("index is preserved")
    );
    assert_eq!(runtime.backend().snapshot(), &clean);
    runtime.dispatch_ui(UiAction::Workflow(WorkflowAction::Cancel));
    runtime.dispatch_ui(UiAction::OpenCommitFilePatch);
    assert!(runtime.render_terminal_text().contains("old mock content"));
    assert!(runtime.render_terminal_text().contains("Ctrl+P"));
    assert_eq!(runtime.backend().snapshot(), &clean);
    assert_eq!(
        runtime.state().ui.details.patch.mode,
        ratagit_core::partial::SelectionMode::Hunk
    );
    dispatch(
        &mut runtime,
        &[
            UiAction::ApplyPatchSelection,
            UiAction::CustomPatch(ratagit_core::custom_patch::CustomPatchAction::Options),
            UiAction::CustomPatch(ratagit_core::custom_patch::CustomPatchAction::MenuKey('r')),
        ],
    );
    assert!(
        runtime
            .render_terminal_text()
            .contains("index and commit history are")
    );
    assert!(
        runtime
            .render_terminal_text()
            .contains("preserved; conflicting patches are rejected")
    );
    assert_eq!(runtime.backend().snapshot(), &clean);
    runtime.dispatch_ui(confirm());
    assert!(
        runtime
            .backend()
            .operations()
            .iter()
            .any(|op| op == "custom-patch-Reverse:abc1234:1")
    );
    assert!(
        runtime
            .backend()
            .snapshot()
            .files
            .iter()
            .any(|f| f.path == "README.md" && f.unstaged && !f.staged)
    );
    assert_eq!(runtime.backend().snapshot().commits, clean.commits);
    assert_eq!(runtime.state().ui.focus, PanelFocus::Details);
    assert!(runtime.state().ui.commits.files.active);
    dispatch(
        &mut runtime,
        &[
            UiAction::CloseFilePatch,
            UiAction::MoveDown,
            UiAction::EnterCommitFilesMultiSelect,
            UiAction::MoveDown,
        ],
    );
    runtime.dispatch_ui(open(Feature::CommitFileCheckout));
    if let Some(Dialog::Confirm(ratagit_core::workflow::Operation::CommitFileCheckout {
        paths,
        ..
    })) = &runtime.state().ui.workflow.dialog
    {
        assert!(paths.contains(&"src/lib.rs".to_string()));
        assert!(paths.contains(&"src/new_name.rs".to_string()));
        assert!(paths.contains(&"src/old_name.rs".to_string()));
        assert!(!paths.contains(&"README.md".to_string()));
    } else {
        panic!("missing captured file selection");
    }
    runtime.dispatch_ui(confirm());
    assert!(runtime.backend().operations().iter().any(|op| {
        op.contains("checkout-commit-files:abc1234:src/lib.rs,src/new_name.rs,src/old_name.rs")
    }));
    assert_eq!(runtime.backend().snapshot().files.len(), 4);

    let mut clean_runtime = self::runtime();
    dispatch(
        &mut clean_runtime,
        &[
            UiAction::EnterCommitFilesMultiSelect,
            UiAction::MoveDown,
            open(Feature::CommitFileDiscard),
        ],
    );
    assert!(
        clean_runtime
            .render_terminal_text()
            .contains("descendants will be rewritten")
    );
    assert!(clean_runtime.backend().snapshot().files.is_empty());
    clean_runtime.dispatch_ui(confirm());
    assert!(
        clean_runtime
            .backend()
            .operations()
            .iter()
            .any(|op| op.starts_with("discard-commit-files:abc1234:"))
    );
    assert!(clean_runtime.backend().snapshot().files.is_empty());
    assert!(
        clean_runtime
            .backend()
            .snapshot()
            .commits
            .iter()
            .all(|c| c.full_id.starts_with("rewritten-"))
    );
    assert!(!clean_runtime.state().ui.commits.files.active);

    let mut branch = self::runtime();
    let unchanged = branch.backend().snapshot().clone();
    dispatch(
        &mut branch,
        &[
            UiAction::FocusPanel {
                panel: PanelFocus::Branches,
            },
            UiAction::OpenBranchCommitsPanel,
            UiAction::OpenBranchCommitFilesPanel,
            UiAction::OpenCommitFilePatch,
        ],
    );
    assert_eq!(branch.state().ui.last_left_focus, PanelFocus::Branches);
    assert!(branch.render_terminal_text().contains("old mock content"));
    branch.dispatch_ui(UiAction::NextPatchFile { up: false });
    assert!(
        branch
            .backend()
            .operations()
            .iter()
            .any(|op| op == "commit-patch:abc1234:src/lib.rs")
    );
    assert!(branch.render_terminal_text().contains("a/src/lib.rs"));
    branch.dispatch_ui(UiAction::NextPatchFile { up: true });
    branch.dispatch_ui(UiAction::CloseFilePatch);
    assert_eq!(branch.state().ui.focus, PanelFocus::Branches);
    assert!(branch.state().ui.branches.commit_files.active);
    assert_eq!(branch.backend().snapshot(), &unchanged);
}

pub(super) fn accumulation() {
    use ratagit_core::custom_patch::{CustomPatchAction as C, Inclusion};
    let mut runtime = runtime();
    let clean = runtime.backend().snapshot().clone();
    dispatch(
        &mut runtime,
        &[
            open(Feature::CommitFilePatch),
            UiAction::MoveDown,
            UiAction::MoveDown,
            UiAction::OpenCommitFilePatch,
            UiAction::CyclePatchSelection,
            UiAction::MovePatchSelection {
                up: false,
                visible_lines: 20,
            },
            UiAction::ApplyPatchSelection,
        ],
    );
    assert_eq!(runtime.state().ui.custom_patch.included.len(), 2);
    assert_eq!(
        runtime.state().ui.custom_patch.included["README.md"],
        Inclusion::Whole
    );
    assert!(matches!(
        runtime.state().ui.custom_patch.included["src/lib.rs"],
        Inclusion::Lines(_)
    ));
    assert!(
        runtime
            .render_terminal_text()
            .contains("│ +new mock content")
    );
    assert_eq!(runtime.backend().snapshot(), &clean);
    runtime.dispatch_ui(UiAction::CloseFilePatch);
    let screen = runtime.render_terminal_text();
    assert!(screen.contains("│M README.md"));
    assert!(screen.contains("╎  A lib.rs"));
    assert!(screen.contains("patch: 2 files"));
    runtime.dispatch_ui(UiAction::CustomPatch(C::Options));
    assert!(
        runtime
            .render_terminal_text()
            .contains("Custom patch options")
    );
    if let Some(Dialog::Menu { choices, .. }) = &runtime.state().ui.workflow.dialog {
        assert!(
            matches!(&choices[5].action,ratagit_core::workflow::ChoiceAction::Copy(text) if text.contains("a/README.md") && text.contains("a/src/lib.rs"))
        );
    } else {
        panic!("missing aggregate options");
    }
    runtime.dispatch_ui(UiAction::CustomPatch(C::MenuKey('p')));
    assert!(
        runtime
            .render_terminal_text()
            .contains("2 files · abc1234 · read-only")
    );
    runtime.dispatch_ui(UiAction::CustomPatch(C::ClosePreview));
    assert_eq!(runtime.state().ui.focus, PanelFocus::Commits);
    dispatch(
        &mut runtime,
        &[
            UiAction::CustomPatch(C::Options),
            UiAction::CustomPatch(C::MenuKey('a')),
            confirm(),
        ],
    );
    assert!(
        runtime
            .backend()
            .operations()
            .iter()
            .any(|op| op == "custom-patch-Apply:abc1234:2")
    );
    assert_eq!(runtime.backend().snapshot().commits, clean.commits);
    assert_eq!(runtime.backend().snapshot().files.len(), 2);
    assert!(
        runtime
            .backend()
            .snapshot()
            .files
            .iter()
            .all(|file| !file.staged && file.unstaged)
    );
    let before_reset = runtime.backend().snapshot().clone();
    dispatch(
        &mut runtime,
        &[
            UiAction::CustomPatch(C::Options),
            UiAction::CustomPatch(C::MenuKey('c')),
        ],
    );
    assert!(runtime.state().ui.custom_patch.included.is_empty());
    assert_eq!(runtime.backend().snapshot(), &before_reset);
    assert!(!runtime.render_terminal_text().contains("[~]"));
}
