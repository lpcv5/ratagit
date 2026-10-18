use ratagit_core::{Action, AppContext, GitResult, PanelFocus, UiAction, update};
use ratagit_testkit::{fixture_dirty_repo, fixture_patch};
use ratagit_ui::{
    MAIN_UI_THEME, RenderContext, TOKYO_NIGHT_THEME, TerminalBuffer, TerminalSize,
    render_terminal_buffer_with_render_context, render_terminal_text,
};
use ratatui::style::{Modifier, Style};
use unicode_width::UnicodeWidthStr;

const WIDTHS: [usize; 4] = [35, 65, 80, 100];

fn fixture() -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    state.work.details.details_pending = false;
    state.repo.details.files_diff =
        "diff --git a/README.md b/README.md\n@@ -1 +1 @@\n-old\n+new".into();
    state
}

fn size(width: usize) -> TerminalSize {
    TerminalSize { width, height: 24 }
}

fn bottom(screen: &str) -> &str {
    screen.lines().last().unwrap().trim_end()
}

fn search(state: &mut AppContext, query: &str, confirmed: bool) {
    update(state, Action::Ui(UiAction::StartSearch));
    for ch in query.chars() {
        update(state, Action::Ui(UiAction::InputSearchChar(ch)));
    }
    if confirmed {
        update(state, Action::Ui(UiAction::ConfirmSearch));
    }
    state.work.details.details_pending = false;
}

fn assert_exact_text_style(buffer: &TerminalBuffer, row: u16, needle: &str, style: Style) {
    let cells = (0..buffer.area.width)
        .map(|x| &buffer[(x, row)])
        .collect::<Vec<_>>();
    let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
    let byte = line
        .find(needle)
        .unwrap_or_else(|| panic!("missing {needle:?} in {line:?}"));
    let start = line[..byte].width();
    for cell in &cells[start..start + needle.width()] {
        assert_eq!(Some(cell.fg), style.fg, "{needle:?} foreground");
        if let Some(bg) = style.bg {
            assert_eq!(cell.bg, bg, "{needle:?} background");
        }
        assert!(
            cell.modifier.contains(style.add_modifier),
            "{needle:?} modifiers"
        );
    }
}

#[test]
fn shortcuts_keep_whole_actions_and_help_at_all_widths() {
    let state = fixture();
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        let bar = bottom(&screen);
        assert!(bar.ends_with("?  commands"), "{width}: {bar}");
        // Every displayed item must be a complete key badge + action label.
        let mut rest = bar;
        for segment in [
            " space  stage/unstage",
            " d  discard",
            " A  amend",
            " c  commit",
            " s  stash",
            " D  reset",
            " enter  expand",
            " ?  commands",
        ] {
            if let Some(tail) = rest.strip_prefix(segment) {
                rest = tail.strip_prefix("  ").unwrap_or(tail);
            }
        }
        assert!(
            rest.is_empty(),
            "incomplete action at {width}: {bar:?}, remainder {rest:?}"
        );
        insta::assert_snapshot!(format!("shortcuts_{width}"), screen);
    }
}

#[test]
fn loading_reserves_help_and_a_local_action() {
    let mut state = fixture();
    state.work.refresh.refresh_pending = true;
    state.work.mutation.operation_pending = Some("push changes to remote".into());
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        let bar = bottom(&screen);
        assert!(bar.contains("?  commands"), "{width}: {bar}");
        assert!(bar.contains("stage/unstage"), "{width}: {bar}");
        assert!(!bar.ends_with("loading:"));
        insta::assert_snapshot!(format!("loading_{width}"), screen);
    }
}

#[test]
fn details_patch_preserves_back_and_help_including_history() {
    let mut state = fixture();
    state.ui.focus = PanelFocus::Details;
    state.ui.last_left_focus = PanelFocus::Files;
    state.ui.details.patch.active = true;
    state.repo.details.patch = Some(fixture_patch());
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        let bar = bottom(&screen);
        assert!(bar.contains("Esc  clear/back"), "{width}: {bar}");
        assert!(bar.contains("?  commands"), "{width}: {bar}");
        insta::assert_snapshot!(format!("patch_{width}"), screen);
    }
    state.ui.last_left_focus = PanelFocus::Commits;
    state.ui.commits.files.active = true;
    state.ui.commits.files.commit_id = Some("abc1234".into());
    state.repo.details.patch.as_mut().unwrap().files[0].source =
        ratagit_core::patch::PatchSource::Commit("abc1234".into());
    state.repo.details.patch.as_mut().unwrap().request =
        ratagit_core::patch::PatchRequest::CommitFiles {
            oid: "abc1234".into(),
            paths: vec!["src/lib.rs".into()],
        };
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        assert!(bottom(&screen).contains("Esc  clear/back"));
        assert!(
            screen.contains("block · src/lib.rs"),
            "patch caption lost at {width}: {screen}"
        );
        insta::assert_snapshot!(format!("historical_patch_{width}"), screen);
    }
}

#[test]
fn search_feedback_shows_results_while_editing_and_after_confirming() {
    let mut state = fixture();
    search(&mut state, "rs", false);
    assert_eq!(state.ui.search.matches.len(), 2);
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        assert!(bottom(&screen).contains("search: rs  2 matches"));
        assert!(!bottom(&screen).contains("1/2"));
        insta::assert_snapshot!(format!("search_editing_{width}"), screen);
    }
    update(&mut state, Action::Ui(UiAction::ConfirmSearch));
    // The core's current_match tracks the last jump, not arbitrary cursor moves.
    // Keep truthful totals even when the selected row is no longer that match.
    state.ui.files.selected = 0;
    state.work.details.details_pending = false;
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        let bar = bottom(&screen);
        assert!(bar.contains("search: rs  2 matches"));
        assert!(!bar.contains("1/2"));
        if width >= 80 {
            assert!(bar.contains("n/N  next/prev"));
            assert!(bar.contains("Esc  clear"));
            assert!(bar.contains("?  commands"));
        }
        insta::assert_snapshot!(format!("search_confirmed_{width}"), screen);
    }
}

#[test]
fn search_no_matches_and_unicode_query_remain_visible_during_loading() {
    let mut state = fixture();
    search(&mut state, "修复界面e\u{301}与搜索反馈", false);
    state.work.refresh.refresh_pending = true;
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        let bar = bottom(&screen);
        assert!(bar.contains("search: 修 复"), "{width}: {bar}");
        assert!(bar.contains("no matches"), "{width}: {bar}");
        assert!(!bar.contains("loading:"));
        // The buffer text includes placeholder cells following double-width
        // glyphs; inspect the actual line layout with buffer-based unit coverage.
        insta::assert_snapshot!(format!("search_no_matches_unicode_{width}"), screen);
    }
    update(&mut state, Action::Ui(UiAction::ConfirmSearch));
    assert!(bottom(&render_terminal_text(&state, size(65))).contains("no matches"));
}

#[test]
fn emoji_query_truncation_keeps_result_feedback_and_complete_graphemes() {
    for query in ["❤️".repeat(30), "👨‍👩‍👧‍👦".repeat(30)] {
        let mut state = fixture();
        search(&mut state, &query, false);
        for width in WIDTHS {
            let buffer = render_terminal_buffer_with_render_context(
                &state,
                size(width),
                RenderContext::default(),
            );
            let bar = (0..width as u16)
                .map(|x| buffer[(x, 23)].symbol())
                .collect::<String>();
            assert!(bar.contains("no matches"), "{width}: {bar}");
            if width >= 65 {
                assert!(bar.contains("Esc  cancel"), "{width}: {bar}");
            }
            for x in 0..width as u16 {
                let symbol = buffer[(x, 23)].symbol();
                assert!(
                    !symbol.ends_with('\u{200d}'),
                    "split ZWJ sequence: {symbol}"
                );
            }
        }
    }
}

#[test]
fn errors_and_current_work_precede_repo_notices_in_short_log() {
    let mut state = fixture();
    state.repo.status.last_error = Some("push rejected".into());
    state.work.mutation.operation_pending = Some("fetch".into());
    state.work.refresh.refresh_pending = true;
    state.notices = vec!["older event".into(), "latest event".into()];
    state.repo.status.large_repo_mode = true;
    state.repo.status.status_scan_skipped = true;
    state.repo.status.status_truncated = true;
    for width in WIDTHS {
        let screen = render_terminal_text(&state, size(width));
        let error = screen.find("error=push").unwrap();
        let work = screen.find("work=running").unwrap();
        assert!(error < work);
        assert!(screen.contains("status=huge"), "{width}: {screen}");
        assert!(screen.contains("status=truncated"), "{width}: {screen}");
        assert!(!screen.contains("latest event"));
        insta::assert_snapshot!(format!("log_priority_{width}"), screen);
    }
}

#[test]
fn badges_errors_and_search_feedback_use_theme_styles() {
    for mut theme in [MAIN_UI_THEME, TOKYO_NIGHT_THEME] {
        theme.selection = ratatui::style::Color::Rgb(100, 150, 200);
        theme.selection_text = ratatui::style::Color::Rgb(10, 20, 30);
        theme.danger = ratatui::style::Color::Rgb(230, 80, 90);
        let mut state = fixture();
        state.repo.status.last_error = Some("push rejected".into());
        let context = RenderContext {
            spinner_frame: 0,
            theme: &theme,
        };
        let buffer = render_terminal_buffer_with_render_context(&state, size(65), context);
        assert_exact_text_style(
            &buffer,
            23,
            " ? ",
            Style::default()
                .fg(theme.selection_text)
                .bg(theme.selection)
                .add_modifier(Modifier::BOLD),
        );
        let error_row = (0..23)
            .find(|&y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .contains("error=push rejected")
            })
            .expect("error must be visible");
        assert_exact_text_style(
            &buffer,
            error_row,
            "error=push rejected",
            Style::default().fg(theme.danger),
        );
        search(&mut state, "absent", false);
        let buffer = render_terminal_buffer_with_render_context(&state, size(65), context);
        assert_exact_text_style(
            &buffer,
            23,
            "no matches",
            Style::default().fg(theme.warning),
        );
    }
}
