use ratagit_core::{Action, AppContext, GitResult, ResetChoice, update};
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::{
    MAIN_UI_THEME, RenderContext, TOKYO_NIGHT_THEME, TerminalBuffer, TerminalSize,
    render_terminal_buffer, render_terminal_buffer_with_render_context,
};
use ratatui::style::{Modifier, Style};

fn dirty_state() -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    state
}

fn buffer_text(buffer: &TerminalBuffer) -> String {
    buffer
        .content()
        .chunks(buffer.area.width as usize)
        .map(|cells| cells.iter().map(|cell| cell.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

// Read only the modal interior so wrapping assertions cannot accidentally match
// the repository panels behind it. Ignore layout whitespace, including the
// continuation cells occupied by wide Unicode graphemes.
fn confirmation_text(buffer: &TerminalBuffer) -> String {
    let rows = buffer
        .content()
        .chunks(buffer.area.width as usize)
        .collect::<Vec<_>>();
    let top = rows
        .iter()
        .position(|cells| {
            cells
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
                .contains(" Confirm ")
        })
        .expect("confirmation title should be visible");
    let left = rows[top]
        .iter()
        .position(|cell| cell.symbol() == "╭")
        .expect("modal left border");
    let right = rows[top]
        .iter()
        .rposition(|cell| cell.symbol() == "╮")
        .expect("modal right border");
    rows.iter()
        .skip(top + 1)
        .take_while(|cells| cells[left].symbol() != "╰")
        .flat_map(|cells| cells[left + 1..right].iter())
        .flat_map(|cell| cell.symbol().chars())
        .filter(|ch| !ch.is_whitespace())
        .collect()
}

fn compact(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[test]
fn reset_confirmation_keeps_complete_question_consequence_and_actions() {
    for (choice, question, action, name) in [
        (
            ResetChoice::Hard,
            "Hard reset working tree to HEAD?",
            "Enter reset hard",
            "hard_reset",
        ),
        (
            ResetChoice::Nuke,
            "Nuke working tree?",
            "Enter nuke",
            "nuke",
        ),
    ] {
        for (width, height) in [(80, 24), (100, 30)] {
            let mut state = dirty_state();
            state.ui.reset_menu.danger_confirm = Some(choice);
            let buffer = render_terminal_buffer(&state, TerminalSize { width, height });
            let content = confirmation_text(&buffer);
            for message in [
                question,
                "This can discard uncommitted work that reflog cannot restore.",
                action,
                "Esc cancel",
            ] {
                assert!(
                    content.contains(&compact(message)),
                    "missing {message:?} at {width}x{height}: {content}"
                );
            }
            insta::assert_snapshot!(format!("{name}_{width}x{height}"), buffer_text(&buffer));
        }
    }
}

#[test]
fn long_unicode_branch_question_and_warning_wrap_before_details() {
    let branch = "feature/确认删除/组合字符-e\u{301}/界面交互优化/very-long-unbroken-target-name";
    for (width, height) in [(80, 24), (100, 30)] {
        let mut state = dirty_state();
        let confirm = &mut state.ui.branches.force_delete_confirm;
        confirm.active = true;
        confirm.context.target_branch = branch.to_string();
        confirm.context.reason = "Git refused because the branch is not fully merged.".to_string();
        let buffer = render_terminal_buffer(&state, TerminalSize { width, height });
        let content = confirmation_text(&buffer);
        for message in [
            format!("Force delete branch: {branch}?"),
            "This action cannot be undone.".to_string(),
            "Enter force delete".to_string(),
            "Esc cancel".to_string(),
        ] {
            assert!(
                content.contains(&compact(&message)),
                "missing {message:?} at {width}x{height}: {content}"
            );
        }
        insta::assert_snapshot!(
            format!("unicode_branch_{width}x{height}"),
            buffer_text(&buffer)
        );
    }
}

#[test]
fn long_unicode_path_preview_does_not_truncate_discard_warning() {
    let path =
        "src/界面/确认删除/组合字符-e\u{301}/a-very-long-unbroken-filename-to-check-wrapping.rs";
    for (width, height) in [(80, 24), (100, 30)] {
        let mut state = dirty_state();
        state.ui.discard_confirm.active = true;
        state.ui.discard_confirm.context = vec![path.to_string()];
        let buffer = render_terminal_buffer(&state, TerminalSize { width, height });
        let content = confirmation_text(&buffer);
        for message in [
            "Discard selected file changes?",
            "This action cannot be undone.",
            path,
            "Enter discard",
            "Esc cancel",
        ] {
            assert!(
                content.contains(&compact(message)),
                "missing {message:?} at {width}x{height}: {content}"
            );
        }
        insta::assert_snapshot!(
            format!("unicode_path_{width}x{height}"),
            buffer_text(&buffer)
        );
    }
}

#[test]
fn wrapped_confirmation_preserves_tone_text_and_footer_styles() {
    let mut state = dirty_state();
    state.ui.reset_menu.danger_confirm = Some(ResetChoice::Hard);
    for theme in [&MAIN_UI_THEME, &TOKYO_NIGHT_THEME] {
        let buffer = render_terminal_buffer_with_render_context(
            &state,
            TerminalSize {
                width: 80,
                height: 24,
            },
            RenderContext {
                spinner_frame: 0,
                theme,
            },
        );
        let danger = Style::default()
            .fg(theme.modal.danger)
            .add_modifier(Modifier::BOLD);
        let text = Style::default().fg(theme.modal.text.unwrap_or(theme.foreground));
        let footer = Style::default().fg(theme.modal.dim);
        for (needle, style) in [
            ("Hard reset working tree to HEAD?", danger),
            ("This can discard", text),
            ("cannot restore.", text),
            ("Enter", danger),
            ("Esc", danger),
            (" cancel", footer),
        ] {
            let cells = buffer
                .content()
                .chunks(buffer.area.width as usize)
                .find_map(|cells| {
                    let line = cells.iter().map(|cell| cell.symbol()).collect::<String>();
                    let start = line.find(needle)?;
                    let offset = line[..start].chars().count();
                    Some(&cells[offset..offset + needle.len()])
                })
                .unwrap_or_else(|| panic!("missing styled text {needle:?}"));
            assert!(
                cells.iter().all(|cell| {
                    style.fg.is_none_or(|fg| cell.fg == fg)
                        && cell.bg == theme.modal.background.unwrap_or(theme.background)
                        && cell.modifier.contains(style.add_modifier)
                }),
                "wrong style for {needle:?}"
            );
        }
    }
}

#[test]
fn modal_footer_keeps_complete_execute_and_cancel_actions_in_compact_width() {
    for width in [40, 65, 80, 100] {
        let mut state = dirty_state();
        state.ui.command_palette.active = true;
        let buffer = render_terminal_buffer(&state, TerminalSize { width, height: 24 });
        let text = buffer_text(&buffer);
        let footer = text
            .lines()
            .find(|line| line.contains("Enter run"))
            .expect("execute hint");
        assert!(footer.contains("Esc close"), "{width}: {footer}");
        assert!(!footer.contains("Esc clo │"));
    }
}

#[test]
fn short_palette_shows_resize_guidance_and_cancel() {
    let mut state = dirty_state();
    state.ui.command_palette.active = true;
    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 80,
            height: 12,
        },
    );
    let text = buffer_text(&buffer);

    assert!(text.contains("Window too small"));
    assert!(text.contains("40 columns × 14 rows"));
    assert!(text.contains("Esc cancel"));
    insta::assert_snapshot!(text);
}

#[test]
fn short_confirmation_shows_resize_guidance_instead_of_hidden_action() {
    let mut state = dirty_state();
    state.ui.reset_menu.danger_confirm = Some(ResetChoice::Hard);
    let buffer = render_terminal_buffer(
        &state,
        TerminalSize {
            width: 80,
            height: 12,
        },
    );
    let text = buffer_text(&buffer);

    assert!(text.contains("Window too small"));
    assert!(text.contains("Esc cancel"));
    assert!(!text.contains("Enter reset hard"));
    insta::assert_snapshot!(text);
}
