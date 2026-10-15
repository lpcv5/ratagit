use super::{card, render_input, render_modal, shortcut_hint, EmptyState, InputField};
use crate::ui::{
    components::atoms::{badge, Tone},
    test_support::{assert_golden, render},
    theme::UiTheme,
};
use ratatui::{layout::Rect, text::Line, widgets::Paragraph};

#[test]
fn test_component_library_gallery_golden() {
    let theme = UiTheme::default();
    let ui = render(92, 18, |frame| {
        frame.render_widget(card(&theme, "Component library", false), frame.area());
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                badge("Neutral", Tone::Neutral, &theme),
                badge("Accent", Tone::Accent, &theme),
                badge("Success", Tone::Success, &theme),
                badge("Warning", Tone::Warning, &theme),
                badge("Danger", Tone::Danger, &theme),
            ])),
            Rect::new(2, 2, 88, 1),
        );
        frame.render_widget(
            Paragraph::new(Line::from(
                [
                    shortcut_hint("Space", "Stage  ", &theme),
                    shortcut_hint("Esc", "Cancel", &theme),
                ]
                .concat(),
            )),
            Rect::new(2, 4, 88, 1),
        );
        for (x, title, focused) in [(2, "Focused card", true), (47, "Inactive card", false)] {
            let area = Rect::new(x, 6, 43, 10);
            let block = card(&theme, title, focused);
            let inner = block.inner(area);
            frame.render_widget(block, area);
            frame.render_widget(EmptyState::new("No changes", &theme), inner);
        }
    });
    assert_golden("component_library", &ui);
}

#[test]
fn test_modal_erases_underlying_content_and_bounds_inner_area() {
    let theme = UiTheme::default();
    for area in [
        Rect::new(2, 2, 14, 5),
        Rect::new(0, 0, 1, 1),
        Rect::new(0, 0, 0, 0),
    ] {
        let ui = render(20, 9, |frame| {
            frame.render_widget(
                Paragraph::new(vec![Line::from("X".repeat(20)); 9]),
                frame.area(),
            );
            let inner = render_modal(frame, area, "Modal", &theme);
            assert_eq!(inner.intersection(area), inner);
            frame.render_widget(EmptyState::new("Empty", &theme), inner);
        });
        if area.width > 2 {
            assert_eq!(ui.buffer[(area.x + 1, area.y + 2)].symbol(), " ");
            assert_eq!(ui.buffer[(area.x + 1, area.y + 2)].bg, theme.surface);
            assert_eq!(ui.buffer[(0, 8)].symbol(), "X");
        }
    }
}

#[test]
fn test_input_cursor_uses_display_cells_and_scrolls_to_editing_position() {
    let theme = UiTheme::default();
    for (value, expected_x, expected_y, suffix) in [
        ("界e\u{301}", 6, 3, "界"),
        ("01234567890123456789END", 12, 3, "END"),
        ("one\ntwo\nthree\nfour\nEND", 6, 5, "END"),
    ] {
        let ui = render(18, 9, |frame| {
            render_input(
                frame,
                Rect::new(2, 2, 12, 5),
                InputField {
                    title: "Input",
                    value,
                    placeholder: "Type here",
                    focused: true,
                },
                &theme,
            )
        });
        assert_eq!(
            ui.cursor,
            Some(ratatui::layout::Position::new(expected_x, expected_y))
        );
        let text: String = ui.buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains(suffix));
    }
}
