use ratagit_core::workflow::{Feature, WorkflowAction};
use ratagit_core::{
    Action, AppContext, GitResult, MenuDirection, MenuKind, PanelFocus, UiAction, update,
};
use ratagit_testkit::fixture_dirty_repo;
use ratagit_ui::{TerminalBuffer, TerminalSize, render_terminal_buffer};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier};

const SIZE: TerminalSize = TerminalSize {
    width: 100,
    height: 30,
};
const SELECTED_FG: Color = Color::Rgb(20, 34, 56);
const SELECTED_BG: Color = Color::Rgb(180, 215, 255);

fn act(state: &mut AppContext, action: UiAction) {
    update(state, Action::Ui(action));
}

fn state(panel: PanelFocus) -> AppContext {
    let mut state = AppContext::default();
    update(
        &mut state,
        Action::GitResult(GitResult::Refreshed(fixture_dirty_repo())),
    );
    act(&mut state, UiAction::FocusPanel { panel });
    state
}

// Match every cell of a unique needle within the requested modal rectangle. This
// avoids accidentally checking the same command in the underlying shortcut bar.
fn positions(buffer: &TerminalBuffer, area: Rect, needle: &str) -> Vec<(u16, u16)> {
    assert!(!needle.is_empty());
    let chars: Vec<_> = needle.chars().collect();
    (area.y..area.bottom())
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
        .collect()
}

fn needle_position(buffer: &TerminalBuffer, area: Rect, needle: &str) -> (u16, u16) {
    let found = positions(buffer, area, needle);
    assert_eq!(
        found.len(),
        1,
        "expected unique {needle:?} in {area:?}: {found:?}"
    );
    found[0]
}

fn bordered_rect(buffer: &TerminalBuffer, area: Rect, title: &str) -> Rect {
    let titles: Vec<_> = positions(buffer, area, title)
        .into_iter()
        .filter(|(x, y)| {
            (area.x..*x).any(|column| buffer[(column, *y)].symbol() == "╭")
                && (*x..area.right()).any(|column| buffer[(column, *y)].symbol() == "╮")
        })
        .collect();
    assert_eq!(
        titles.len(),
        1,
        "unique bordered title {title:?}: {titles:?}"
    );
    let (title_x, top) = titles[0];
    let left = (area.x..title_x)
        .rev()
        .find(|x| buffer[(*x, top)].symbol() == "╭")
        .expect("top-left modal/list corner");
    let right = (title_x..area.right())
        .find(|x| buffer[(*x, top)].symbol() == "╮")
        .expect("top-right modal/list corner");
    let bottom = (top + 1..area.bottom())
        .find(|y| buffer[(left, *y)].symbol() == "╰")
        .expect("bottom-left modal/list corner");
    Rect::new(left, top, right - left + 1, bottom - top + 1)
}

fn content_rect(outer: Rect) -> Rect {
    // Modal content has an extra horizontal inset inside its outer border.
    Rect::new(outer.x + 2, outer.y + 1, outer.width - 4, outer.height - 2)
}

fn title_rect(modal: Rect) -> Rect {
    Rect::new(modal.x, modal.y, modal.width, 1)
}

fn footer_rect(modal: Rect) -> Rect {
    Rect::new(modal.x + 2, modal.bottom() - 2, modal.width - 4, 1)
}

fn assert_text_style(buffer: &TerminalBuffer, area: Rect, needle: &str, fg: Color) {
    let (x, y) = needle_position(buffer, area, needle);
    for offset in 0..needle.chars().count() as u16 {
        let cell = &buffer[(x + offset, y)];
        assert_eq!(
            cell.fg,
            fg,
            "{needle:?} foreground at ({}, {y})",
            x + offset
        );
        assert_eq!(cell.bg, Color::Reset, "{needle:?} ordinary background");
    }
}

fn luminance(color: Color) -> f64 {
    let Color::Rgb(r, g, b) = color else {
        panic!("selection contrast must use explicit RGB colors, got {color:?}")
    };
    let linear = |channel: u8| {
        let srgb = f64::from(channel) / 255.0;
        if srgb <= 0.04045 {
            srgb / 12.92
        } else {
            ((srgb + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn assert_selected_row(buffer: &TerminalBuffer, area: Rect, needle: &str, indicator: bool) {
    let (x, y) = needle_position(buffer, area, needle);
    if indicator {
        assert_eq!(x, area.x + 2, "label follows the highlight indicator");
        assert_eq!(buffer[(area.x, y)].symbol(), "▌");
        assert_eq!(buffer[(area.x + 1, y)].symbol(), " ");
    } else {
        assert_eq!(x, area.x, "selected text starts at the content inset");
    }
    assert_eq!(
        buffer[(area.right() - 1, y)].symbol(),
        " ",
        "exercise trailing padding"
    );
    for column in area.x..area.right() {
        let cell = &buffer[(column, y)];
        assert_eq!(
            cell.fg, SELECTED_FG,
            "{needle:?} foreground at ({column}, {y})"
        );
        assert_eq!(
            cell.bg, SELECTED_BG,
            "{needle:?} background at ({column}, {y})"
        );
        assert!(
            cell.modifier.contains(Modifier::BOLD),
            "{needle:?} bold at ({column}, {y})"
        );
        assert!(
            !cell.modifier.intersects(Modifier::REVERSED | Modifier::DIM),
            "{needle:?} must not reverse or dim at ({column}, {y}): {:?}",
            cell.modifier
        );
        let fg = luminance(cell.fg);
        let bg = luminance(cell.bg);
        let contrast = (fg.max(bg) + 0.05) / (fg.min(bg) + 0.05);
        assert!(
            contrast >= 7.0,
            "{needle:?} WCAG contrast {contrast:.3}:1 is below 7:1"
        );
    }
}

#[test]
fn reset_selection_overrides_muted_and_danger_labels_including_indicator_and_padding() {
    let mut state = state(PanelFocus::Files);
    act(&mut state, UiAction::OpenResetMenu);
    let labels = ["mixed", "soft", "hard", "Nuke"];
    for (selected, label) in labels.iter().enumerate() {
        let buffer = render_terminal_buffer(&state, SIZE);
        let modal = bordered_rect(&buffer, buffer.area, "Reset");
        let list = bordered_rect(&buffer, modal, "Mode").inner(ratatui::layout::Margin::new(1, 1));
        assert_selected_row(&buffer, list, label, true);
        for (index, other) in labels.iter().enumerate() {
            if index != selected {
                assert_text_style(
                    &buffer,
                    list,
                    other,
                    if *other == "Nuke" {
                        Color::Red
                    } else {
                        Color::DarkGray
                    },
                );
            }
        }
        assert_text_style(&buffer, title_rect(modal), "Reset", Color::Yellow);
        assert_text_style(&buffer, modal, "Description", Color::DarkGray);
        assert_text_style(&buffer, footer_rect(modal), "select", Color::DarkGray);
        act(
            &mut state,
            UiAction::MoveMenuSelection {
                menu: MenuKind::Reset,
                direction: MenuDirection::Down,
            },
        );
    }
}

#[test]
fn branch_choice_menus_highlight_each_selection_without_recoloring_ordinary_rows_or_tones() {
    for (open, menu, title, tone, labels) in [
        (
            UiAction::OpenBranchRebaseMenu,
            MenuKind::BranchRebase,
            "Rebase",
            Color::Yellow,
            [
                "simple rebase",
                "interactive rebase",
                "rebase onto origin/main",
            ],
        ),
        (
            UiAction::OpenBranchDeleteMenu,
            MenuKind::BranchDelete,
            "Delete Branch",
            Color::Red,
            ["delete local", "delete remote", "delete local and remote"],
        ),
    ] {
        let mut state = state(PanelFocus::Branches);
        act(&mut state, UiAction::MoveDown);
        act(&mut state, open);
        for (selected, label) in labels.iter().enumerate() {
            let buffer = render_terminal_buffer(&state, SIZE);
            let modal = bordered_rect(&buffer, buffer.area, title);
            let list =
                bordered_rect(&buffer, modal, "Choice").inner(ratatui::layout::Margin::new(1, 1));
            // Include the indicator in the search to distinguish "delete local"
            // from the longer "delete local and remote" choice.
            let (label_x, y) = needle_position(&buffer, list, &format!("▌ {label}"));
            assert_eq!(label_x, list.x);
            let row = Rect::new(list.x, y, list.width, 1);
            assert_selected_row(&buffer, row, label, true);
            for (index, other) in labels.iter().enumerate() {
                if index != selected {
                    let ordinary_row = Rect::new(list.x, list.y + index as u16, list.width, 1);
                    assert_text_style(&buffer, ordinary_row, other, Color::DarkGray);
                }
            }
            assert_text_style(&buffer, title_rect(modal), title, tone);
            assert_text_style(&buffer, modal, "Description", Color::DarkGray);
            assert_text_style(&buffer, footer_rect(modal), "cancel", Color::DarkGray);
            act(
                &mut state,
                UiAction::MoveMenuSelection {
                    menu,
                    direction: MenuDirection::Down,
                },
            );
        }
    }
}

#[test]
fn command_palette_selection_covers_keys_labels_and_padding_after_navigation_filtering_and_scroll()
{
    for scenario in ["first", "next", "filtered", "scrolled"] {
        let mut state = state(PanelFocus::Files);
        act(&mut state, UiAction::OpenCommandPalette);
        let first = state.command_palette_entries()[0];
        match scenario {
            "next" => act(&mut state, UiAction::MoveCommandPaletteDown),
            "filtered" => {
                for ch in "fetch".chars() {
                    act(&mut state, UiAction::PaletteInput(ch));
                }
                assert_eq!(state.command_palette_entries().len(), 1);
            }
            "scrolled" => {
                for _ in 1..state.command_palette_entries().len() {
                    act(&mut state, UiAction::MoveCommandPaletteDown);
                }
            }
            _ => {}
        }
        let selected = state
            .selected_command_palette_entry()
            .expect("selected command");
        let buffer = render_terminal_buffer(&state, SIZE);
        let modal = bordered_rect(&buffer, buffer.area, "Commands");
        let content = content_rect(modal);
        let label = format!("  {:<8} {}", selected.key, selected.label);
        assert_selected_row(&buffer, content, &label, false);
        assert_text_style(&buffer, title_rect(modal), "Commands", Color::Cyan);
        assert_text_style(&buffer, footer_rect(modal), "select", Color::DarkGray);
        match scenario {
            "filtered" => {
                assert_text_style(&buffer, content, "Filter: fetch", Color::Reset);
                assert_text_style(&buffer, content, "Global commands", Color::DarkGray);
            }
            "scrolled" => {
                assert!(
                    positions(&buffer, content, first.label).is_empty(),
                    "scroll must move the first entry out of view"
                );
                assert_text_style(&buffer, content, "quit", Color::Reset);
            }
            _ => {
                let ordinary = state.command_palette_entries()[usize::from(scenario == "first")];
                assert_text_style(
                    &buffer,
                    content,
                    &format!("  {:<8} {}", ordinary.key, ordinary.label),
                    Color::Reset,
                );
                assert_text_style(&buffer, content, "Local commands", Color::DarkGray);
            }
        }
    }
}

#[test]
fn workflow_menu_selection_covers_labels_and_padding_and_keeps_footer_muted() {
    let mut state = state(PanelFocus::Files);
    act(
        &mut state,
        UiAction::Workflow(WorkflowAction::Open(Feature::Stash)),
    );
    let labels = [
        "All changes (including untracked)",
        "Staged changes only",
        "Unstaged changes (keep index)",
    ];
    for (selected, label) in labels.iter().enumerate() {
        let buffer = render_terminal_buffer(&state, SIZE);
        let modal = bordered_rect(&buffer, buffer.area, "Stash options");
        let content = content_rect(modal);
        assert_selected_row(&buffer, content, label, false);
        for (index, other) in labels.iter().enumerate() {
            if index != selected {
                assert_text_style(&buffer, content, other, Color::Reset);
            }
        }
        assert_text_style(&buffer, title_rect(modal), "Stash options", Color::Cyan);
        assert_text_style(&buffer, footer_rect(modal), "confirm", Color::DarkGray);
        assert_text_style(&buffer, footer_rect(modal), "cancel", Color::DarkGray);
        act(&mut state, UiAction::Workflow(WorkflowAction::Down));
    }
}
