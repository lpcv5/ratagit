use ratagit_core::{Action, AppContext, Command, GitResult, PanelFocus, UiAction, update};
use ratagit_testkit::{fixture_many_files, fixture_patch};
use ratagit_ui::{TerminalSize, render_terminal_buffer, render_terminal_text};

const SIZE: TerminalSize = TerminalSize {
    width: 120,
    height: 35,
};

fn preview_state() -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_many_files())),
    );
    state.repo.details.files_diff = concat!(
        "diff --git a/file-00.txt b/file-00.txt\nindex 1111111..2222222 100644\n",
        "--- a/file-00.txt\n+++ b/file-00.txt\n@@ -1,3 +1,3 @@\n",
        "-old first\n+new first\n unchanged\n-old last\n+new last\n",
    )
    .into();
    state
}

fn position(state: &AppContext, needle: &str) -> (usize, usize) {
    let buffer = render_terminal_buffer(state, SIZE);
    buffer
        .content()
        .chunks(SIZE.width as usize)
        .enumerate()
        .find_map(|(y, row)| {
            let text = row.iter().map(|cell| cell.symbol()).collect::<String>();
            text.find(needle)
                .map(|byte| (text[..byte].chars().count(), y))
        })
        .unwrap_or_else(|| panic!("missing {needle}: {}", render_terminal_text(state, SIZE)))
}

#[test]
fn entry_keeps_expanded_files_and_previous_preview_until_patch_arrives() {
    let mut state = preview_state();
    let branches = position(&state, "Branches");
    let old_line = position(&state, "-old first");
    let before = render_terminal_buffer(&state, SIZE);
    insta::assert_snapshot!("details_entry_before", render_terminal_text(&state, SIZE));
    let commands = update(&mut state, Action::Ui(UiAction::OpenFilePatch));
    assert!(matches!(commands.as_slice(), [Command::LoadPatch { .. }]));
    assert!(state.repo.details.patch.is_none());
    assert!(state.work.details.patch_request.is_some());
    assert_eq!(position(&state, "Branches"), branches);
    assert_eq!(position(&state, "-old first"), old_line);
    let pending = render_terminal_buffer(&state, SIZE);
    // Compare the actual gutter, code and background cells, excluding borders.
    for y in 1..=10 {
        for x in old_line.0 - 3..SIZE.width - 1 {
            assert_eq!(before[(x as u16, y)], pending[(x as u16, y)]);
        }
    }
    let screen = render_terminal_text(&state, SIZE);
    assert!(!screen.contains("Loading reviewed patch"));
    insta::assert_snapshot!("details_entry_pending", screen);

    let (request_id, request) = state.work.details.patch_request.clone().unwrap();
    let mut patch = fixture_patch();
    patch.request = request.clone();
    patch
        .files
        .retain(|file| file.source == ratagit_core::patch::PatchSource::Worktree);
    patch.files[0].header = format!(
        "{}\n",
        state
            .repo
            .details
            .files_diff
            .lines()
            .take(4)
            .collect::<Vec<_>>()
            .join("\n")
    )
    .into_bytes();
    update(
        &mut state,
        Action::GitResult(GitResult::LoadedPatch {
            request_id,
            request,
            result: Ok(patch),
        }),
    );
    assert_eq!(state.ui.focus, PanelFocus::Details);
    assert_eq!(position(&state, "Branches"), branches);
    assert_eq!(position(&state, "-old first"), old_line);
    insta::assert_snapshot!("details_entry_ready", render_terminal_text(&state, SIZE));
}

#[test]
fn entry_retains_scrolled_preview_and_still_shows_empty_loading_and_errors() {
    let mut state = preview_state();
    state.repo.details.files_diff = state
        .repo
        .details
        .files_diff
        .replace("@@ -1,3 +1,3 @@", "@@ -1,60 +1,60 @@");
    for line in 4..=60 {
        state
            .repo
            .details
            .files_diff
            .push_str(&format!(" context {line}\n"));
    }
    state.ui.details.scroll_offset = 3;
    let previous = position(&state, "-old first");
    update(&mut state, Action::Ui(UiAction::OpenFilePatch));
    assert_eq!(state.ui.details.scroll_offset, 3);
    assert_eq!(position(&state, "-old first"), previous);
    let (request_id, request) = state.work.details.patch_request.clone().unwrap();
    update(
        &mut state,
        Action::GitResult(GitResult::LoadedPatch {
            request_id,
            request,
            result: Err("patch read failed".into()),
        }),
    );
    assert!(render_terminal_text(&state, SIZE).contains("patch read failed"));
    assert!(!render_terminal_text(&state, SIZE).contains("-old first"));

    let mut state = preview_state();
    state.repo.details.files_diff.clear();
    update(&mut state, Action::Ui(UiAction::OpenFilePatch));
    assert!(render_terminal_text(&state, SIZE).contains("Loading reviewed patch"));
}
