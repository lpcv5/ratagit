use super::*;
use ratagit_core::extras::RepositoryOperation;
use ratagit_core::workflow::{Dialog, Feature, Operation, WorkflowAction};
use ratagit_core::{Action, Command as CoreCommand, update};
mod branches;
mod commits;
mod files;

fn action(runtime: &mut TracedCase<'_>, action: WorkflowAction) {
    runtime.dispatch_ui(UiAction::Workflow(action));
}
fn open(runtime: &mut TracedCase<'_>, feature: Feature) {
    action(runtime, WorkflowAction::Open(feature));
}
fn confirm(runtime: &mut TracedCase<'_>) {
    action(runtime, WorkflowAction::Confirm);
}
fn down(runtime: &mut TracedCase<'_>, count: usize) {
    for _ in 0..count {
        action(runtime, WorkflowAction::Down);
    }
}
fn input(runtime: &mut TracedCase<'_>, text: &str) {
    for ch in text.chars() {
        action(runtime, WorkflowAction::Input(ch));
    }
}

fn assert_operation(runtime: &mut TracedCase<'_>, operation: Operation) {
    // Assert the exact reducer-to-backend trace, then execute through Runtime
    // and assert real Git state in the scenario. Failure artifacts retain keys.
    let commands = update(
        &mut runtime.state().clone(),
        Action::Ui(UiAction::Workflow(WorkflowAction::Confirm)),
    );
    assert_eq!(
        commands,
        vec![CoreCommand::RepositoryOperation(
            RepositoryOperation::Workflow(operation)
        )]
    );
    confirm(runtime);
}

fn select_branch(runtime: &mut TracedCase<'_>, name: &str) {
    runtime.dispatch_ui(UiAction::FocusPanel {
        panel: PanelFocus::Branches,
    });
    for _ in 0..runtime.state().ui.branches.selected {
        runtime.dispatch_ui(UiAction::MoveUp);
    }
    for _ in 0..runtime.state().repo.branches.items.len() {
        if runtime.state().repo.branches.items[runtime.state().ui.branches.selected].name == name {
            return;
        }
        runtime.dispatch_ui(UiAction::MoveDown);
    }
    panic!("branch {name} not found");
}

fn select_commit(runtime: &mut TracedCase<'_>, id: &str) {
    runtime.dispatch_ui(UiAction::FocusPanel {
        panel: PanelFocus::Commits,
    });
    for _ in 0..runtime.state().ui.commits.selected {
        runtime.dispatch_ui(UiAction::MoveUp);
    }
    for _ in 0..runtime.state().repo.commits.items.len() {
        if ratagit_core::commit_key(
            &runtime.state().repo.commits.items[runtime.state().ui.commits.selected],
        ) == id
        {
            return;
        }
        runtime.dispatch_ui(UiAction::MoveDown);
    }
    panic!("commit {id} not found");
}

fn commit_file(repo: &TmpGitRepo, path: &str, content: &str, message: &str) -> String {
    write(repo.path().join(path), content).unwrap();
    repo.run_git(&["add", "--", path]);
    repo.run_git(&["commit", "-m", message]);
    repo.run_git_capture(&["rev-parse", "HEAD"])
        .trim()
        .to_string()
}
