use ratagit_core::partial::SelectionMode;
use ratagit_core::patch::PatchSource;
use ratagit_core::workflow::{Feature, WorkflowAction};
use ratagit_core::{Action, AppContext, CommitHashStatus, GitResult, PanelFocus, UiAction, update};
use ratagit_testkit::{fixture_commit, fixture_dirty_repo, fixture_file, fixture_patch};
use ratagit_ui::{
    MAIN_UI_THEME, RenderContext, TerminalBuffer, TerminalSize, UiTheme,
    render_terminal_buffer_with_render_context,
};
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Color, Modifier};

const SIZE: TerminalSize = TerminalSize {
    width: 120,
    height: 40,
};

fn act(state: &mut AppContext, action: UiAction) {
    update(state, Action::Ui(action));
}

fn state() -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    state
}

fn custom_theme() -> UiTheme {
    let mut theme = MAIN_UI_THEME;
    for (role, rgb) in [
        ("foreground", (230, 231, 232)),
        ("background", (10, 12, 14)),
        ("accent", (235, 160, 200)),
        ("muted", (120, 125, 130)),
        ("selection.foreground", (10, 20, 30)),
        ("selection.background", (220, 230, 240)),
        ("batch.foreground", (240, 230, 220)),
        ("batch.background", (30, 20, 10)),
        ("badge.foreground", (230, 220, 210)),
        ("badge.background", (20, 30, 40)),
        ("files.modified", (220, 190, 110)),
        ("files.unknown", (90, 200, 220)),
        ("files.conflict", (240, 100, 120)),
        ("files.staged", (120, 220, 170)),
        ("commit.graph", (130, 140, 150)),
        ("commit.hash.merged", (140, 230, 160)),
        ("commit.hash.pushed", (230, 210, 140)),
        ("commit.hash.unpushed", (240, 150, 160)),
        ("commit.author.0", (170, 180, 240)),
        ("modal.active", (130, 210, 230)),
        ("modal.warning", (235, 190, 120)),
        ("modal.danger", (240, 130, 150)),
        ("modal.muted", (140, 145, 155)),
        ("modal.selection.foreground", (15, 25, 35)),
        ("modal.selection.background", (235, 225, 205)),
        ("diff.added.foreground", (200, 250, 220)),
        ("diff.removed.foreground", (255, 210, 220)),
        ("diff.context.foreground", (220, 225, 245)),
        ("diff.context.cursor", (25, 40, 65)),
        ("diff.context.marked", (35, 30, 60)),
        ("diff.added.unstaged.normal", (10, 30, 20)),
        ("diff.added.unstaged.marked", (15, 40, 25)),
        ("diff.added.unstaged.cursor", (20, 50, 30)),
        ("diff.added.staged.normal", (20, 35, 25)),
        ("diff.added.staged.marked", (25, 45, 30)),
        ("diff.added.staged.cursor", (30, 55, 35)),
        ("diff.removed.unstaged.normal", (35, 10, 20)),
        ("diff.removed.unstaged.marked", (45, 15, 25)),
        ("diff.removed.unstaged.cursor", (55, 20, 30)),
        ("diff.removed.staged.normal", (40, 15, 25)),
        ("diff.removed.staged.marked", (50, 20, 30)),
        ("diff.removed.staged.cursor", (60, 25, 35)),
    ] {
        theme
            .set_color(role, Color::Rgb(rgb.0, rgb.1, rgb.2))
            .unwrap();
    }
    theme
        .validate()
        .expect("configured fixture has readable color pairs");
    theme
}

fn render(state: &AppContext, theme: &UiTheme) -> TerminalBuffer {
    render_terminal_buffer_with_render_context(
        state,
        SIZE,
        RenderContext {
            theme,
            ..Default::default()
        },
    )
}

fn position(buffer: &TerminalBuffer, area: Rect, needle: &str) -> (u16, u16) {
    let chars: Vec<_> = needle.chars().collect();
    let matches: Vec<_> = (area.y..area.bottom())
        .flat_map(|y| {
            let chars = &chars;
            (area.x..area.right()).filter_map(move |x| {
                (x as usize + chars.len() <= area.right() as usize
                    && chars.iter().enumerate().all(|(offset, ch)| {
                        buffer[(x + offset as u16, y)].symbol() == ch.to_string()
                    }))
                .then_some((x, y))
            })
        })
        .collect();
    assert_eq!(
        matches.len(),
        1,
        "unique {needle:?} in {area:?}: {matches:?}"
    );
    matches[0]
}

fn bordered(buffer: &TerminalBuffer, title: &str) -> Rect {
    let (x, y) = position(buffer, buffer.area, title);
    let left = (0..x)
        .rev()
        .find(|column| buffer[(*column, y)].symbol() == "╭")
        .unwrap();
    let right = (x..buffer.area.right())
        .find(|column| buffer[(*column, y)].symbol() == "╮")
        .unwrap();
    let bottom = (y + 1..buffer.area.bottom())
        .find(|row| buffer[(left, *row)].symbol() == "╰")
        .unwrap();
    Rect::new(left, y, right - left + 1, bottom - y + 1)
}

fn colors(buffer: &TerminalBuffer, area: Rect, needle: &str, fg: Color, bg: Color) {
    let (x, y) = position(buffer, area, needle);
    for offset in 0..needle.chars().count() as u16 {
        assert_eq!(
            buffer[(x + offset, y)].fg,
            fg,
            "{needle:?} fg at ({}, {y})",
            x + offset
        );
        assert_eq!(
            buffer[(x + offset, y)].bg,
            bg,
            "{needle:?} bg at ({}, {y})",
            x + offset
        );
    }
}

fn row_colors(buffer: &TerminalBuffer, area: Rect, needle: &str, fg: Color, bg: Color) {
    let (_, y) = position(buffer, area, needle);
    assert_eq!(
        buffer[(area.right() - 1, y)].symbol(),
        " ",
        "exercise padding after {needle:?}"
    );
    for x in area.x..area.right() {
        let cell = &buffer[(x, y)];
        assert_eq!((cell.fg, cell.bg), (fg, bg), "{needle:?} at ({x}, {y})");
        assert!(cell.modifier.contains(Modifier::BOLD));
        assert!(!cell.modifier.intersects(Modifier::REVERSED | Modifier::DIM));
    }
}

#[test]
fn render_context_applies_custom_titles_cursor_and_base_padding_without_leaking_theme() {
    let state = state();
    let theme = custom_theme();
    let before = render_terminal_buffer_with_render_context(&state, SIZE, RenderContext::default());
    let custom = render(&state, &theme);
    let after = render_terminal_buffer_with_render_context(&state, SIZE, RenderContext::default());
    assert_eq!(
        before, after,
        "a custom render must not mutate the default theme"
    );
    assert_ne!(before, custom);
    colors(
        &custom,
        custom.area,
        "󰈙 Files",
        theme.accent,
        theme.background,
    );
    colors(
        &custom,
        custom.area,
        " Branches",
        theme.muted,
        theme.background,
    );
    colors(
        &custom,
        custom.area,
        " 1 ",
        theme.selection_text,
        theme.selection,
    );
    colors(
        &custom,
        custom.area,
        " 2 ",
        theme.badge_text,
        theme.badge_background,
    );
    let files = bordered(&custom, "󰈙 Files").inner(Margin::new(1, 1));
    row_colors(
        &custom,
        files,
        "? README.md",
        theme.selection_text,
        theme.selection,
    );
    let padding = &custom[(files.right() - 1, files.bottom() - 1)];
    assert_eq!(padding.symbol(), " ");
    assert_eq!(
        (padding.fg, padding.bg),
        (theme.foreground, theme.background)
    );
}

#[test]
fn configured_file_and_commit_semantics_survive_until_batch_selection_overrides_entire_rows() {
    let mut snapshot = fixture_dirty_repo();
    let mut conflict = fixture_file("c-conflict.txt", false, false);
    conflict.conflicted = true;
    snapshot.files = vec![
        fixture_file("a-staged.txt", true, false),
        fixture_file("b-untracked.txt", false, true),
        conflict,
        fixture_file("d-cursor.txt", false, false),
    ];
    snapshot.commits = [
        ("aaa1111", CommitHashStatus::MergedToMain),
        ("bbb2222", CommitHashStatus::Pushed),
        ("ccc3333", CommitHashStatus::Unpushed),
    ]
    .into_iter()
    .map(|(id, status)| {
        let mut commit = fixture_commit(id, "theme commit");
        commit.author_name = "HH".into(); // deterministic author bucket zero
        commit.graph = "|".into();
        commit.hash_status = status;
        commit
    })
    .collect();
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(snapshot)),
    );
    act(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Log,
        },
    );
    let theme = custom_theme();
    let ordinary = render(&state, &theme);
    let files = bordered(&ordinary, "󰈙 Files").inner(Margin::new(1, 1));
    let (x, y) = position(&ordinary, files, "M a-staged.txt");
    assert_eq!(ordinary[(x, y)].fg, theme.files.modified);
    colors(
        &ordinary,
        files,
        "a-staged.txt",
        theme.files.staged,
        theme.background,
    );
    let (x, y) = position(&ordinary, files, "? b-untracked.txt");
    assert_eq!(ordinary[(x, y)].fg, theme.files.unknown);
    let (x, y) = position(&ordinary, files, "MU c-conflict.txt");
    assert_eq!(ordinary[(x + 1, y)].fg, theme.files.conflict);
    let commits = bordered(&ordinary, " Commits").inner(Margin::new(1, 1));
    for (hash, fg) in [
        ("aaa1111", theme.commit.hash_merged),
        ("bbb2222", theme.commit.hash_pushed),
        ("ccc3333", theme.commit.hash_unpushed),
    ] {
        colors(&ordinary, commits, hash, fg, theme.background);
        let (x, y) = position(&ordinary, commits, hash);
        assert_eq!(ordinary[(x - 3, y)].symbol(), "|");
        assert_eq!(ordinary[(x - 3, y)].fg, theme.commit.graph);
        colors(
            &ordinary,
            Rect::new(x, y, commits.right() - x, 1),
            "HH",
            theme.commit.authors[0],
            theme.background,
        );
    }
    act(
        &mut state,
        UiAction::FocusPanel {
            panel: PanelFocus::Files,
        },
    );
    act(&mut state, UiAction::EnterFilesMultiSelect);
    for _ in 0..3 {
        act(&mut state, UiAction::MoveDown);
    }
    let batch = render(&state, &theme);
    let files = bordered(&batch, "󰈙 Files").inner(Margin::new(1, 1));
    for label in [
        "a-staged.txt",
        "b-untracked.txt",
        "c-conflict.txt",
        "d-cursor.txt",
    ] {
        row_colors(
            &batch,
            files,
            label,
            theme.batch_selection_text,
            theme.batch_selection,
        );
    }
    let commits = bordered(&batch, " Commits").inner(Margin::new(1, 1));
    colors(
        &batch,
        commits,
        "ccc3333",
        theme.commit.hash_unpushed,
        theme.background,
    );
}

#[test]
fn custom_diff_colors_reach_passive_and_interactive_normal_marked_and_cursor_rows() {
    let theme = custom_theme();
    let mut state = state();
    state.repo.details.files_diff =
        "### unstaged\ndiff --git a/theme.txt b/theme.txt\n@@ -1 +1 @@\n-old theme\n+new theme"
            .into();
    state.work.details.details_pending = false;
    let passive = render(&state, &theme);
    let details = bordered(&passive, " Details").inner(Margin::new(1, 1));
    colors(
        &passive,
        details,
        "-old theme",
        theme.diff.removed_text,
        theme.diff.removed[0][0],
    );
    colors(
        &passive,
        details,
        "+new theme",
        theme.diff.added_text,
        theme.diff.added[0][0],
    );

    for staged in [false, true] {
        state.repo.details.patch = Some(fixture_patch());
        state
            .repo
            .details
            .patch
            .as_mut()
            .unwrap()
            .files
            .retain(|file| (file.source == PatchSource::Index) == staged);
        state.ui.focus = PanelFocus::Details;
        state.ui.last_left_focus = PanelFocus::Files;
        state.ui.details.patch.active = true;
        state.ui.details.patch.mode = SelectionMode::Line;
        state.ui.details.patch.staged = staged;
        state.ui.details.patch.selected[usize::from(staged)] = 1;
        state.ui.details.patch.marked = [3].into_iter().collect();
        state.work.details.patch_request = None;
        let interactive = render(&state, &theme);
        let details = bordered(&interactive, " Details").inner(Margin::new(1, 1));
        let source = usize::from(staged);
        for (label, fg, bg) in [
            (
                "-old first",
                theme.diff.removed_text,
                theme.diff.removed[source][0],
            ),
            (
                "+new first",
                theme.diff.added_text,
                theme.diff.added[source][2],
            ),
            (
                "-old last",
                theme.diff.removed_text,
                theme.diff.removed[source][1],
            ),
            (
                "+new last",
                theme.diff.added_text,
                theme.diff.added[source][0],
            ),
        ] {
            colors(&interactive, details, label, fg, bg);
            let (_, y) = position(&interactive, details, label);
            assert_eq!(
                interactive[(details.right() - 1, y)].bg,
                bg,
                "diff row padding for {label}"
            );
        }
        state.ui.details.patch.selected[source] = 2;
        let context_cursor = render(&state, &theme);
        colors(
            &context_cursor,
            details,
            " unchanged",
            theme.diff.context_text,
            theme.diff.context_cursor,
        );
    }
}

#[test]
fn configured_modal_selections_fill_rows_and_modal_background_can_inherit_or_override_base() {
    for explicit_background in [false, true] {
        let mut theme = custom_theme();
        if explicit_background {
            theme
                .set_color("modal.foreground", Color::Rgb(230, 235, 245))
                .unwrap();
            theme
                .set_color("modal.background", Color::Rgb(20, 25, 35))
                .unwrap();
        }
        for kind in ["reset", "rebase", "palette", "workflow"] {
            let mut state = state();
            let (title, selected, list_title) = match kind {
                "reset" => {
                    act(&mut state, UiAction::OpenResetMenu);
                    ("⚠ Reset", "mixed", Some("Mode"))
                }
                "rebase" => {
                    act(
                        &mut state,
                        UiAction::FocusPanel {
                            panel: PanelFocus::Branches,
                        },
                    );
                    act(&mut state, UiAction::MoveDown);
                    act(&mut state, UiAction::OpenBranchRebaseMenu);
                    ("⚠ Rebase", "simple rebase", Some("Choice"))
                }
                "palette" => {
                    act(&mut state, UiAction::OpenCommandPalette);
                    ("? Commands", "  space    stage/unstage", None)
                }
                _ => {
                    act(
                        &mut state,
                        UiAction::Workflow(WorkflowAction::Open(Feature::Stash)),
                    );
                    ("ℹ Stash options", "All changes (including untracked)", None)
                }
            };
            let buffer = render(&state, &theme);
            let modal = bordered(&buffer, title);
            let content = Rect::new(modal.x + 2, modal.y + 1, modal.width - 4, modal.height - 2);
            let selected_area = if let Some(list_title) = list_title {
                bordered(&buffer, list_title).inner(Margin::new(1, 1))
            } else {
                content
            };
            row_colors(
                &buffer,
                selected_area,
                selected,
                theme.modal.surface_text,
                theme.modal.surface,
            );
            let title_color = if matches!(kind, "reset" | "rebase") {
                theme.modal.warning
            } else {
                theme.modal.active
            };
            let bg = theme.modal.background.unwrap_or(theme.background);
            colors(
                &buffer,
                Rect::new(modal.x, modal.y, modal.width, 1),
                title,
                title_color,
                bg,
            );
            let footer = Rect::new(content.x, modal.bottom() - 2, content.width, 1);
            colors(
                &buffer,
                footer,
                if kind == "palette" { "close" } else { "cancel" },
                theme.modal.dim,
                bg,
            );
            let padding = &buffer[(content.right() - 1, modal.bottom() - 4)];
            assert_eq!(padding.symbol(), " ");
            assert_eq!(
                (padding.fg, padding.bg),
                (theme.modal.text.unwrap_or(theme.foreground), bg),
                "{kind} modal padding"
            );
            if kind == "reset" {
                colors(&buffer, selected_area, "Nuke", theme.modal.danger, bg);
            }
        }
    }
}
