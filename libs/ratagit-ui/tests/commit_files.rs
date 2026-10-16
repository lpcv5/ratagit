use ratagit_core::custom_patch::CustomPatchAction as C;
use ratagit_core::patch::PatchSource;
use ratagit_core::workflow::{Feature, WorkflowAction};
use ratagit_core::{
    Action, AppContext, Command, CommitFileEntry, CommitFileStatus, GitResult, PanelFocus,
    UiAction, update,
};
use ratagit_ui::{TerminalSize, render_terminal_buffer, render_terminal_text};
use ratatui::style::Color;

#[path = "commit_files/presentation.rs"]
mod presentation;

const SIZE: TerminalSize = TerminalSize {
    width: 120,
    height: 36,
};
fn action(state: &mut AppContext, action: UiAction) -> Vec<Command> {
    update(state, Action::Ui(action))
}
fn details(state: &mut AppContext, commands: Vec<Command>) {
    for command in commands {
        if let Command::RefreshCommitFileDiff { request_id, target } = command {
            update(
                state,
                Action::GitResult(GitResult::CommitFileDiff {
                    request_id,
                    target,
                    result: Ok(
                        "diff --git a/src/lib.rs b/src/lib.rs\n@@ -1 +1 @@\n-old first\n+new first"
                            .into(),
                    ),
                }),
            );
        }
    }
}
fn files() -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(ratagit_testkit::fixture_dirty_repo())),
    );
    action(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        },
    );
    let commands = action(&mut state, UiAction::OpenCommitFilesPanel);
    let [Command::RefreshCommitFiles { commit_id }] = commands.as_slice() else {
        panic!("missing request");
    };
    let commands = update(
        &mut state,
        Action::GitResult(GitResult::CommitFiles {
            commit_id: commit_id.clone(),
            result: Ok(vec![CommitFileEntry {
                path: "src/lib.rs".into(),
                old_path: None,
                status: CommitFileStatus::Modified,
            }]),
        }),
    );
    details(&mut state, commands);
    let commands = action(&mut state, UiAction::MoveDown); // select file under its directory
    details(&mut state, commands);
    state
}
fn historical_patch(state: &mut AppContext) {
    let commands = action(state, UiAction::OpenCommitFilePatch);
    let [
        Command::LoadPatch {
            request_id,
            request,
        },
    ] = commands.as_slice()
    else {
        panic!("missing historical patch request: {commands:?}");
    };
    let mut patch = ratagit_testkit::fixture_patch();
    patch.files.retain(|f| f.source == PatchSource::Worktree);
    patch.request = request.clone();
    for file in &mut patch.files {
        file.source = PatchSource::Commit("abc1234".into());
    }
    update(
        state,
        Action::GitResult(GitResult::LoadedPatch {
            request_id: *request_id,
            request: request.clone(),
            result: Ok(patch),
        }),
    );
}

#[test]
fn commit_files_hunks_actions_and_confirmations_have_reviewable_snapshots_and_styles() {
    let mut state = files();
    action(
        &mut state,
        UiAction::Workflow(WorkflowAction::Open(Feature::CommitFileCheckout)),
    );
    insta::assert_snapshot!("commit_files_checkout", render_terminal_text(&state, SIZE));
    action(&mut state, UiAction::Workflow(WorkflowAction::Cancel));
    action(&mut state, UiAction::OpenCommandPalette);
    insta::assert_snapshot!("commit_files_commands", render_terminal_text(&state, SIZE));
    action(&mut state, UiAction::CloseCommandPalette);
    historical_patch(&mut state);
    let buffer = render_terminal_buffer(&state, SIZE);
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.bg == Color::Rgb(40, 80, 58))
    );
    insta::assert_snapshot!("commit_files_hunks", render_terminal_text(&state, SIZE));
    action(&mut state, UiAction::CyclePatchSelection);
    action(&mut state, UiAction::TogglePatchRange);
    action(
        &mut state,
        UiAction::MovePatchSelection {
            up: false,
            visible_lines: 20,
        },
    );
    insta::assert_snapshot!(
        "commit_files_line_range",
        render_terminal_text(&state, SIZE)
    );
    action(&mut state, UiAction::ApplyPatchSelection);
    assert_eq!(state.ui.custom_patch.included.len(), 1);
    action(&mut state, UiAction::ClearPatchMarks);
    insta::assert_snapshot!(
        "custom_patch_included_lines",
        render_terminal_text(&state, SIZE)
    );
    assert!(
        render_terminal_buffer(&state, SIZE)
            .content()
            .iter()
            .any(|cell| cell.bg == Color::Rgb(72, 38, 46))
    );
    let buffer = render_terminal_buffer(&state, SIZE);
    let row = buffer
        .content()
        .chunks(SIZE.width)
        .find(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("│ -old first")
        })
        .unwrap();
    let rail = row
        .iter()
        .find(|cell| cell.symbol() == "│" && cell.bg == Color::Rgb(72, 38, 46))
        .unwrap();
    assert!(rail.modifier.contains(ratatui::style::Modifier::BOLD));
    custom_output(&mut state, C::Options);
    insta::assert_snapshot!(
        "custom_patch_options_narrow",
        render_terminal_text(
            &state,
            TerminalSize {
                width: 80,
                height: 24
            }
        )
    );
    action(&mut state, UiAction::Workflow(WorkflowAction::Down));
    let buffer = render_terminal_buffer(&state, SIZE);
    assert!(ratagit_ui::buffer_contains_text_with_style(
        &buffer,
        "a        Apply patch to working tree",
        ratatui::style::Style::default()
            .fg(Color::Rgb(20, 34, 56))
            .bg(Color::Rgb(180, 215, 255))
            .add_modifier(ratatui::style::Modifier::BOLD)
    ));
    insta::assert_snapshot!(
        "commit_files_patch_actions",
        render_terminal_text(&state, SIZE)
    );
    action(&mut state, UiAction::Workflow(WorkflowAction::Confirm));
    insta::assert_snapshot!(
        "commit_files_patch_confirm",
        render_terminal_text(&state, SIZE)
    );
    action(&mut state, UiAction::Workflow(WorkflowAction::Cancel));
    action(&mut state, UiAction::RequestPatchDiscard);
    insta::assert_snapshot!(
        "commit_files_remove_hunk",
        render_terminal_text(&state, SIZE)
    );
    action(&mut state, UiAction::Workflow(WorkflowAction::Cancel));
    action(&mut state, UiAction::ClearPatchMarks);
    action(&mut state, UiAction::OpenCommandPalette);
    insta::assert_snapshot!(
        "commit_files_hunk_commands",
        render_terminal_text(&state, SIZE)
    );
    action(&mut state, UiAction::CloseCommandPalette);
    custom_output(&mut state, C::Preview);
    insta::assert_snapshot!("custom_patch_preview", render_terminal_text(&state, SIZE));
    insta::assert_snapshot!(
        "custom_patch_preview_narrow",
        render_terminal_text(
            &state,
            TerminalSize {
                width: 80,
                height: 24
            }
        )
    );
    action(&mut state, UiAction::CustomPatch(C::ClosePreview));
    let patch = state.repo.details.patch.as_mut().unwrap();
    patch.request = ratagit_core::patch::PatchRequest::CommitFiles {
        oid: "another-commit".into(),
        paths: vec!["src/lib.rs".into()],
    };
    patch.files[0].source = PatchSource::Commit("another-commit".into());
    action(&mut state, UiAction::ApplyPatchSelection);
    insta::assert_snapshot!(
        "custom_patch_replace_source",
        render_terminal_text(&state, SIZE)
    );
}

#[test]
fn patch_markers_and_shortcut_badges_follow_theme_and_selection_precedence() {
    use ratagit_ui::{MAIN_UI_THEME, RenderContext, render_terminal_buffer_with_render_context};
    use ratatui::style::Modifier;
    let mut state = files();
    historical_patch(&mut state);
    action(&mut state, UiAction::CyclePatchSelection);
    action(&mut state, UiAction::ApplyPatchSelection);
    action(&mut state, UiAction::CloseFilePatch);
    let mut theme = MAIN_UI_THEME;
    theme.info = Color::Rgb(10, 130, 220);
    theme.selection = Color::Rgb(20, 40, 60);
    theme.selection_text = Color::Rgb(230, 220, 210);
    let render = |state: &AppContext| {
        render_terminal_buffer_with_render_context(
            state,
            SIZE,
            RenderContext {
                spinner_frame: 0,
                theme: &theme,
            },
        )
    };
    let buffer = render(&state);
    let row = buffer
        .content()
        .chunks(SIZE.width)
        .find(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("╎  M lib.rs")
        })
        .unwrap();
    let marker = row.iter().find(|cell| cell.symbol() == "╎").unwrap();
    assert_eq!(marker.fg, theme.selection_text);
    assert_eq!(marker.bg, theme.selection);
    let footer = buffer.content().chunks(SIZE.width).last().unwrap();
    let badge = footer.iter().find(|cell| cell.symbol() == "s").unwrap();
    assert_eq!(badge.bg, theme.selection);
    assert_eq!(badge.fg, theme.selection_text);
    assert!(badge.modifier.contains(Modifier::BOLD));
    action(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Files,
        },
    );
    let buffer = render(&state);
    let marker = buffer
        .content()
        .iter()
        .find(|cell| cell.symbol() == "╎")
        .unwrap();
    assert_eq!(marker.fg, theme.info);
    assert!(marker.modifier.contains(Modifier::BOLD));
    action(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Commits,
        },
    );
    action(&mut state, UiAction::StartSearch);
    for ch in "lib".chars() {
        action(&mut state, UiAction::InputSearchChar(ch));
    }
    action(&mut state, UiAction::ConfirmSearch);
    action(&mut state, UiAction::EnterCommitFilesMultiSelect);
    let text = render_terminal_text(&state, SIZE);
    assert!(text.contains("✓╎  M lib.rs"), "{text}");
    let buffer = render(&state);
    let row = buffer
        .content()
        .chunks(SIZE.width)
        .find(|row| {
            row.iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains("✓╎  M lib.rs")
        })
        .unwrap();
    let marker = row.iter().find(|cell| cell.symbol() == "╎").unwrap();
    assert_eq!(marker.fg, theme.batch_selection_text);
    assert_eq!(marker.bg, theme.batch_selection);
    insta::assert_snapshot!("custom_patch_range_and_search", text);
}

fn custom_output(state: &mut AppContext, action_kind: C) {
    let commands = action(state, UiAction::CustomPatch(action_kind));
    let [Command::RenderCustomPatch { request_id, patch }] = commands.as_slice() else {
        panic!("missing captured custom output: {commands:?}");
    };
    update(state,Action::GitResult(GitResult::RenderedCustomPatch { request_id: *request_id, patch: patch.clone(), result: Ok(b"diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,2 +1,2 @@\n-old first\n+new first\n unchanged\n".to_vec()) }));
}

#[test]
fn custom_patch_file_inclusion_marks_full_partial_and_background_rows() {
    use ratagit_core::custom_patch::Inclusion;
    let mut state = files();
    historical_patch(&mut state);
    state.ui.details.patch.mode = ratagit_core::partial::SelectionMode::Line;
    action(&mut state, UiAction::ApplyPatchSelection);
    let mut second = state.repo.custom_patch.files["src/lib.rs"].clone();
    second.old.path = Some("src/main.rs".into());
    second.new.path = Some("src/main.rs".into());
    state
        .repo
        .custom_patch
        .files
        .insert("src/main.rs".into(), second);
    state
        .ui
        .custom_patch
        .included
        .insert("src/main.rs".into(), Inclusion::Whole);
    state.repo.commits.files.items.push(CommitFileEntry {
        path: "src/main.rs".into(),
        old_path: None,
        status: CommitFileStatus::Modified,
    });
    ratagit_core::mark_commit_file_items_changed(
        &state.repo.commits.files.items,
        &mut state.ui.commits.files,
    );
    ratagit_core::initialize_commit_files_tree(
        &state.repo.commits.files.items,
        &mut state.ui.commits.files,
    );
    action(&mut state, UiAction::CloseFilePatch);
    ratagit_core::select_commit_file_tree_path(
        &state.repo.commits.files.items,
        &mut state.ui.commits.files,
        "src/lib.rs",
    );
    let text = render_terminal_text(&state, SIZE);
    assert!(text.contains("╎  M lib.rs"));
    assert!(text.contains("│  M main.rs"));
    assert!(ratagit_ui::buffer_contains_text_with_style(
        &render_terminal_buffer(&state, SIZE),
        "│  M",
        ratatui::style::Style::default()
            .fg(ratagit_ui::MAIN_UI_THEME.info)
            .add_modifier(ratatui::style::Modifier::BOLD)
    ));
    insta::assert_snapshot!("custom_patch_file_inclusion", text);
    insta::assert_snapshot!(
        "custom_patch_file_inclusion_narrow",
        render_terminal_text(
            &state,
            TerminalSize {
                width: 80,
                height: 24
            }
        )
    );
    let mut without_patch = state.clone();
    without_patch.ui.custom_patch = Default::default();
    let before = render_terminal_text(&without_patch, SIZE);
    let after = render_terminal_text(&state, SIZE);
    for name in ["lib.rs", "main.rs", "src/"] {
        let column = |text: &str| {
            let line = text
                .lines()
                .filter(|line| line.contains(name))
                .last()
                .unwrap();
            line[..line.find(name).unwrap()].chars().count()
        };
        assert_eq!(
            column(&before),
            column(&after),
            "{name} moved after patch inclusion"
        );
    }
    assert_eq!(before.lines().last(), after.lines().last());
    let mut branch = state.clone();
    branch.repo.branches.commit_files = state.repo.commits.files.clone();
    branch.ui.branches.commit_files = state.ui.commits.files.clone();
    branch.ui.commits.files.active = false;
    branch.ui.branches.subview = ratagit_core::BranchesSubview::CommitFiles;
    branch.ui.focus = PanelFocus::Branches;
    branch.ui.last_left_focus = PanelFocus::Branches;
    let branch_text = render_terminal_text(&branch, SIZE);
    assert!(branch_text.contains("Branch Commit Files"));
    assert!(branch_text.contains("patch: 2 files"));
    assert!(branch_text.contains("╎  M lib.rs"));
    assert!(branch_text.contains("│  M main.rs"));
    insta::assert_snapshot!("custom_patch_branch_file_inclusion", branch_text);
    action(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Files,
        },
    );
    assert!(render_terminal_text(&state, SIZE).contains("│  M main.rs"));
    let mut large = b"diff --git a/large b/large\n".to_vec();
    large.extend(
        "+content\n"
            .repeat(ratagit_core::custom_patch::PREVIEW_LINES + 10)
            .as_bytes(),
    );
    large.extend_from_slice(b"+sentinel after preview cap\n");
    state.repo.custom_patch.preview = large;
    state.ui.custom_patch.preview_return_focus = Some(PanelFocus::Files);
    let text = render_terminal_text(&state, SIZE);
    assert!(text.contains("first 5000 lines"));
    insta::assert_snapshot!(
        "custom_patch_limited_preview_narrow",
        render_terminal_text(
            &state,
            TerminalSize {
                width: 80,
                height: 24
            }
        )
    );
    assert!(!text.contains("sentinel after preview cap"));
    assert!(
        state
            .repo
            .custom_patch
            .preview
            .ends_with(b"+sentinel after preview cap\n")
    );
}
