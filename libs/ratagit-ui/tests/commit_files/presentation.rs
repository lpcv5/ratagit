use super::*;
use ratagit_core::partial::SelectionMode;
use ratagit_ui::{
    MAIN_UI_THEME, RenderContext, TerminalBuffer, render_terminal_buffer_with_render_context,
};
use ratatui::style::{Modifier, Style};

const DIFF: &str = "diff --git a/src/lib.rs b/src/lib.rs\nindex 1111111..2222222 100644\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,3 +1,3 @@\n-old first\n+new first\n unchanged\n-old last\n+new last\n";

fn probe(buffer: &TerminalBuffer, needle: &str) -> (usize, Vec<(Color, Color, Modifier)>) {
    for row in buffer.content().chunks(SIZE.width) {
        let text = row.iter().map(|cell| cell.symbol()).collect::<String>();
        if let Some(start) = text.find(needle) {
            let column = text[..start].chars().count();
            return (
                column,
                row[column..column + needle.len()]
                    .iter()
                    .map(|cell| (cell.fg, cell.bg, cell.modifier))
                    .collect(),
            );
        }
    }
    panic!("missing diff row {needle}");
}

#[test]
fn historical_file_hunk_and_custom_previews_share_themed_columns_and_styles() {
    let mut state = files();
    // Deliberately supply conflicting Git ANSI colors: semantic UI colors win.
    state.repo.details.commit_file_diff = DIFF
        .replace("-old", "\x1b[35m-old")
        .replace("+new", "\x1b[34m+new")
        .replace("\n", "\x1b[m\n");
    let mut theme = MAIN_UI_THEME;
    theme.diff.added_text = Color::Rgb(101, 220, 150);
    theme.diff.removed_text = Color::Rgb(220, 130, 150);
    theme.diff.added[0][0] = Color::Rgb(10, 40, 20);
    theme.diff.removed[0][0] = Color::Rgb(40, 10, 20);
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
    let passive = render(&state);
    insta::assert_snapshot!(
        "historical_passive_diff",
        render_terminal_text(&state, SIZE)
    );
    historical_patch(&mut state);
    // Remove active cursor styling to compare identical read-only content.
    state.ui.focus = PanelFocus::Commits;
    let source = render(&state);
    state.repo.custom_patch.preview = DIFF.as_bytes().to_vec();
    state.ui.custom_patch.preview_return_focus = Some(PanelFocus::Commits);
    let preview = render(&state);
    for needle in [
        "   diff --git",
        "   --- a/src/lib.rs",
        "   @@ -1,3 +1,3 @@",
        "   -old first",
        "   +new first",
        "    unchanged",
    ] {
        assert_eq!(probe(&passive, needle), probe(&source, needle), "{needle}");
        assert_eq!(probe(&source, needle), probe(&preview, needle), "{needle}");
    }
    for (needle, fg, bg) in [
        (
            "-old first",
            theme.diff.removed_text,
            theme.diff.removed[0][0],
        ),
        ("+new first", theme.diff.added_text, theme.diff.added[0][0]),
    ] {
        assert!(
            probe(&passive, needle)
                .1
                .iter()
                .all(|(actual_fg, actual_bg, _)| *actual_fg == fg && *actual_bg == bg)
        );
    }
    let mut branch = files();
    branch.repo.details.commit_file_diff = state.repo.details.commit_file_diff.clone();
    branch.repo.branches.commit_files = branch.repo.commits.files.clone();
    branch.ui.branches.commit_files = branch.ui.commits.files.clone();
    branch.ui.branches.subview = ratagit_core::BranchesSubview::CommitFiles;
    branch.ui.commits.files.active = false;
    branch.ui.focus = PanelFocus::Branches;
    branch.ui.last_left_focus = PanelFocus::Branches;
    let branch = render(&branch);
    assert_eq!(
        probe(&passive, "   -old first"),
        probe(&branch, "   -old first")
    );
}

#[test]
fn historical_mode_caption_tracks_cycle_and_yields_to_preview_metadata() {
    let mut state = files();
    historical_patch(&mut state);
    for (mode, label) in [
        (SelectionMode::Hunk, "hunk"),
        (SelectionMode::Line, "line"),
        (SelectionMode::Block, "block"),
    ] {
        assert_eq!(state.ui.details.patch.mode, mode);
        assert!(render_terminal_text(&state, SIZE).contains(&format!("{label} · src/lib.rs")));
        action(&mut state, UiAction::CyclePatchSelection);
    }
    insta::assert_snapshot!(
        "historical_mode_narrow",
        render_terminal_text(
            &state,
            TerminalSize {
                width: 80,
                height: 24
            }
        )
    );
    let buffer = render_terminal_buffer(&state, SIZE);
    assert!(ratagit_ui::buffer_contains_text_with_style(
        &buffer,
        "hunk · src/lib.rs",
        Style::default().fg(MAIN_UI_THEME.muted)
    ));
    action(&mut state, UiAction::ApplyPatchSelection);
    custom_output(&mut state, C::Preview);
    let text = render_terminal_text(&state, SIZE);
    assert!(text.contains("read-only"));
    assert!(!text.contains("hunk · src/lib.rs"));
    action(&mut state, UiAction::CustomPatch(C::ClosePreview));
    assert!(render_terminal_text(&state, SIZE).contains("hunk · src/lib.rs"));
}
