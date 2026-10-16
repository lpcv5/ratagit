use ratagit_core::workflow::{Dialog, Feature, Operation, WorkflowAction};
use ratagit_core::{Action, AppContext, GitResult, PanelFocus, UiAction, update};
use ratagit_testkit::{fixture_conflict, fixture_dirty_repo};
use ratagit_ui::{TerminalSize, render_terminal_buffer_with_cursor, render_terminal_text};

const SIZE: TerminalSize = TerminalSize {
    width: 100,
    height: 30,
};
fn state(panel: PanelFocus) -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    update(&mut state, Action::Ui(UiAction::FocusPanel { panel }));
    state
}
fn open(state: &mut AppContext, feature: Feature) {
    update(
        state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Open(feature))),
    );
}

#[test]
fn files_workflow_dialogs_have_deterministic_snapshots_and_selected_style() {
    let mut state = state(PanelFocus::Files);
    open(&mut state, Feature::Stash);
    insta::assert_snapshot!("files_stash_options", render_terminal_text(&state, SIZE));
    let (buffer, _) = render_terminal_buffer_with_cursor(&state, SIZE);
    assert!(ratagit_ui::buffer_contains_text_with_style(
        &buffer,
        "All changes (including untracked)",
        ratatui::style::Style::default()
            .fg(ratatui::style::Color::White)
            .bg(ratatui::style::Color::Blue)
            .add_modifier(ratatui::style::Modifier::BOLD)
    ));
    open(&mut state, Feature::Filter);
    insta::assert_snapshot!("files_status_filter", render_terminal_text(&state, SIZE));
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_conflict())),
    );
    open(&mut state, Feature::Conflict);
    insta::assert_snapshot!("files_conflict_options", render_terminal_text(&state, SIZE));
    update(
        &mut state,
        Action::Ui(UiAction::Workflow(WorkflowAction::Confirm)),
    );
    insta::assert_snapshot!("files_conflict_confirm", render_terminal_text(&state, SIZE));
}

#[test]
fn branch_workflow_dialogs_and_unicode_input_cursor() {
    let mut state = state(PanelFocus::Branches);
    open(&mut state, Feature::Merge);
    insta::assert_snapshot!("branches_merge", render_terminal_text(&state, SIZE));
    open(&mut state, Feature::Upstream);
    insta::assert_snapshot!("branches_upstream", render_terminal_text(&state, SIZE));
    open(&mut state, Feature::CheckoutByName);
    for ch in "功能/新分支".chars() {
        update(
            &mut state,
            Action::Ui(UiAction::Workflow(WorkflowAction::Input(ch))),
        );
    }
    let (buffer, cursor) = render_terminal_buffer_with_cursor(&state, SIZE);
    assert!(cursor.is_some());
    let cursor = cursor.unwrap();
    assert!(cursor.x < buffer.area.width && cursor.y < buffer.area.height);
    insta::assert_snapshot!(
        "branches_checkout_prompt",
        render_terminal_text(&state, SIZE)
    );
}

#[test]
fn commit_workflow_dialogs_and_remote_list_scroll() {
    let mut state = state(PanelFocus::Commits);
    open(&mut state, Feature::Reset);
    insta::assert_snapshot!("commits_reset", render_terminal_text(&state, SIZE));
    open(&mut state, Feature::CopyAttributes);
    insta::assert_snapshot!("commits_copy_attribute", render_terminal_text(&state, SIZE));
    state.ui.workflow.dialog = Some(Dialog::Confirm(Operation::Autosquash("abc1234".into())));
    insta::assert_snapshot!(
        "commits_rewrite_confirm",
        render_terminal_text(&state, SIZE)
    );
    state.ui.focus = PanelFocus::Branches;
    state.ui.workflow.dialog = None;
    open(&mut state, Feature::RemoteBranches);
    let request = state.ui.workflow.remote_request.unwrap();
    update(
        &mut state,
        Action::GitResult(GitResult::RemoteBranches {
            request_id: request,
            result: Ok((0..20).map(|i| format!("origin/branch-{i:02}")).collect()),
        }),
    );
    for _ in 0..18 {
        update(
            &mut state,
            Action::Ui(UiAction::Workflow(WorkflowAction::Down)),
        );
    }
    insta::assert_snapshot!(
        "branches_remote_scrolled",
        render_terminal_text(&state, SIZE)
    );
}

#[test]
fn many_remote_branches_do_not_overflow_modal_height() {
    let mut ctx = state(PanelFocus::Branches);
    ctx.ui.workflow.dialog = Some(Dialog::Menu {
        title: "Remote branches",
        choices: (0..65_535)
            .map(|i| ratagit_core::workflow::Choice {
                label: format!("origin/branch-{i}"),
                action: ratagit_core::workflow::ChoiceAction::Filter(
                    ratagit_core::workflow::FileFilter::All,
                ),
            })
            .collect(),
        selected: 65_534,
    });
    let text = render_terminal_text(&ctx, SIZE);
    assert!(text.contains("origin/branch-65534"));
    assert!(text.contains("Esc cancel"));
}

#[test]
fn batch_conflict_confirmation_keeps_consequences_visible() {
    let mut ctx = state(PanelFocus::Files);
    ctx.ui.workflow.dialog = Some(Dialog::Confirm(Operation::Resolve {
        paths: (0..100)
            .map(|i| format!("src/large-directory/file-{i:03}.rs"))
            .collect(),
        side: ratagit_core::workflow::ConflictSide::Ours,
    }));
    let text = render_terminal_text(&ctx, SIZE);
    assert!(text.contains("manual"));
    assert!(text.contains("conflict edits will be lost"));
    assert!(text.contains("file-000.rs"));
    insta::assert_snapshot!("files_batch_conflict_confirm", text);
}
