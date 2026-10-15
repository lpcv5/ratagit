use ratagit_core::{Action, AppContext, PanelFocus, UiAction, patch::PatchSource, update};
use ratagit_ui::{
    TerminalSize, details_content_lines_for_terminal_size, render_terminal_buffer,
    render_terminal_text,
};
use ratatui::style::Color;

fn state() -> AppContext {
    let mut state = AppContext::default();
    state.ui.focus = PanelFocus::Details;
    state.ui.last_left_focus = PanelFocus::Files;
    state.ui.details.patch.active = true;
    state.repo.details.patch = Some(ratagit_testkit::fixture_patch());
    state
        .repo
        .details
        .patch
        .as_mut()
        .unwrap()
        .files
        .retain(|f| f.source == PatchSource::Worktree);
    state
}

fn action(state: &mut AppContext, action: UiAction) {
    update(state, Action::Ui(action));
}

fn movement(state: &mut AppContext) {
    action(
        state,
        UiAction::MovePatchSelection {
            up: false,
            visible_lines: 18,
        },
    );
}

const SIZE: TerminalSize = TerminalSize {
    width: 120,
    height: 35,
};

#[test]
fn test_pr8_details_marks_line_mode_and_staged_palette() {
    let mut state = state();
    action(&mut state, UiAction::TogglePatchMark);
    movement(&mut state);
    let buffer = render_terminal_buffer(&state, SIZE);
    let text = render_terminal_text(&state, SIZE);
    assert!(text.contains("Hunk 2/2 | 1 selected"));
    assert!(text.contains("┃ 1   │ - old first"));
    let backgrounds: Vec<_> = buffer.content().iter().map(|cell| cell.bg).collect();
    assert!(backgrounds.contains(&Color::Rgb(72, 38, 46))); // Marked removal.
    assert!(backgrounds.contains(&Color::Rgb(40, 80, 58))); // Cursor addition.
    insta::assert_snapshot!("pr8_details_marks", text);
    action(&mut state, UiAction::CyclePatchSelection);
    action(&mut state, UiAction::TogglePatchRange);
    movement(&mut state);
    insta::assert_snapshot!("pr8_details_line_range", render_terminal_text(&state, SIZE));
    // Staged rows retain polarity and use PR #8's brighter staged backgrounds.
    let patch = state.repo.details.patch.as_mut().unwrap();
    patch.files[0].source = PatchSource::Index;
    action(&mut state, UiAction::SwitchPatchSource);
    let buffer = render_terminal_buffer(&state, SIZE);
    assert!(
        buffer
            .content()
            .iter()
            .any(|cell| cell.bg == Color::Rgb(111, 52, 64))
    );
    insta::assert_snapshot!("pr8_details_staged", render_terminal_text(&state, SIZE));
}

#[test]
fn test_pr8_details_hunk_range_freeze_and_narrow_gutter() {
    let mut state = state();
    action(&mut state, UiAction::TogglePatchRange);
    movement(&mut state);
    let text = render_terminal_text(&state, SIZE);
    assert!(text.contains("2 selected | Range"));
    insta::assert_snapshot!("pr8_details_hunk_range", text);
    action(&mut state, UiAction::TogglePatchRange);
    assert!(!render_terminal_text(&state, SIZE).contains("selected | Range"));
    let text = render_terminal_text(
        &state,
        TerminalSize {
            width: 35,
            height: 20,
        },
    );
    // There is room to read changes without the dual-number gutter.
    assert!(!text.contains("│ - old"));
    insta::assert_snapshot!("pr8_details_narrow", text);
}

#[test]
fn test_pr8_details_loading_suppresses_stale_selection_colors() {
    let mut state = state();
    action(&mut state, UiAction::TogglePatchMark);
    state.work.details.patch_request = Some((
        1,
        state.repo.details.patch.as_ref().unwrap().request.clone(),
    ));
    let buffer = render_terminal_buffer(&state, SIZE);
    assert!(
        !buffer
            .content()
            .iter()
            .any(|cell| cell.bg == Color::Rgb(40, 80, 58))
    );
    insta::assert_snapshot!("pr8_details_loading", render_terminal_text(&state, SIZE));
}

#[test]
fn test_selected_replacement_fits_actual_viewport_across_terminal_heights() {
    for height in 14..40 {
        let size = TerminalSize { width: 100, height };
        let mut state = state();
        movement(&mut state);
        action(
            &mut state,
            UiAction::DetailsViewport {
                visible_lines: details_content_lines_for_terminal_size(size),
            },
        );
        let text = render_terminal_text(&state, size);
        assert!(text.contains("old last"), "height {height}: {text}");
        assert!(text.contains("new last"), "height {height}: {text}");
    }
}
