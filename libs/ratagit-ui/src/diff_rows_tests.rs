use super::*;
use crate::theme::{MAIN_UI_THEME, TOKYO_NIGHT_THEME};
use ratagit_core::partial::preview_rows;
use ratatui::style::Color;

#[test]
fn only_selection_gutter_is_added_to_native_diff_without_line_numbers() {
    let native = concat!(
        "diff --git a/a.rs b/a.rs\nindex 1111111..2222222 100644\n",
        "--- a/a.rs\n+++ b/a.rs\n@@ -1,4 +1,4 @@ function\n",
        " context\n-    old  \n+    new  \n \n",
        "\\ No newline at end of file\n",
        "@@ -50 +50,2 @@ another\n-old\n+new\n+@@ this is code\n",
    );
    let rows: Vec<_> = preview_rows(native)
        .into_iter()
        .map(|r| DiffRow::preview(r.row, r.staged))
        .collect();
    let lines = render(&rows, 0, 100, &MAIN_UI_THEME);
    let content = lines
        .iter()
        .map(|line| line.spans[1].text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(format!("{content}\n"), native);
    assert!(
        lines
            .iter()
            .all(|line| line.spans.len() == 2 && line.spans[0].text == "   ")
    );
    assert_eq!(lines[6].text(), "   -    old  ");
    assert!(
        lines
            .iter()
            .any(|line| line.style.bg == Some(Color::Rgb(24, 46, 36)))
    );
    assert_eq!(lines[0].spans[1].style.fg, None);
    assert_eq!(lines[4].spans[1].style.fg, Some(MAIN_UI_THEME.info));
    let code = lines
        .iter()
        .find(|line| line.spans[1].text == "+@@ this is code")
        .unwrap();
    assert_eq!(code.spans[1].style.fg, Some(MAIN_UI_THEME.diff.added_text));
}

fn contrast_ratio(foreground: Color, background: Color) -> f64 {
    fn luminance(color: Color) -> f64 {
        let Color::Rgb(red, green, blue) = color else {
            panic!("contrast requires explicit RGB, got {color:?}");
        };
        let linear = |channel: u8| {
            let value = f64::from(channel) / 255.0;
            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(red) + 0.7152 * linear(green) + 0.0722 * linear(blue)
    }
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

#[test]
fn diff_text_and_gutters_are_readable_for_every_highlight_combination() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Paragraph, Widget};

    // Each background list is ordered as plain, marked, cursor. Cursor wins
    // when a row is both marked and focused; staging keeps its own shade.
    let cases = [
        (
            PatchLineKind::Added,
            MAIN_UI_THEME.diff.added_text,
            [
                [(24, 46, 36), (30, 65, 47), (40, 80, 58)],
                [(33, 67, 48), (44, 85, 60), (52, 100, 70)],
            ],
        ),
        (
            PatchLineKind::Removed,
            MAIN_UI_THEME.diff.removed_text,
            [
                [(51, 30, 37), (72, 38, 46), (88, 44, 54)],
                [(73, 36, 45), (94, 44, 55), (111, 52, 64)],
            ],
        ),
        (
            PatchLineKind::Context,
            MAIN_UI_THEME.diff.context_text,
            [
                [(0, 0, 0), (46, 58, 86), (37, 57, 82)],
                [(0, 0, 0), (46, 58, 86), (37, 57, 82)],
            ],
        ),
    ];
    for (kind, foreground, backgrounds) in cases {
        for staged in [false, true] {
            for cursor in [false, true] {
                for marked in [false, true] {
                    let display = DiffRow {
                        row: PatchRow {
                            text: "content".into(),
                            id: None,
                            kind,
                            old_line: Some(1),
                            new_line: Some(1),
                        },
                        staged,
                        focused: cursor,
                        chosen: marked,
                        marker: cursor,
                    };
                    let line = render_row(&display, &MAIN_UI_THEME);
                    if kind == PatchLineKind::Context && !cursor && !marked {
                        assert_eq!(line.style, Style::default());
                        assert_eq!(line.spans[1].style, Style::default());
                        continue;
                    }
                    let index = if cursor { 2 } else { usize::from(marked) };
                    let (red, green, blue) = backgrounds[usize::from(staged)][index];
                    let background = Color::Rgb(red, green, blue);
                    assert_eq!(line.style, Style::default().fg(foreground).bg(background));
                    assert_eq!(line.spans[1].style, line.style);
                    assert_eq!(
                        line.spans[0].style.fg,
                        Some(MAIN_UI_THEME.diff.context_text)
                    );
                    assert_eq!(line.spans[0].style.bg, Some(background));

                    let area = Rect::new(0, 0, 12, 1);
                    let mut buffer = Buffer::empty(area);
                    let spans = line
                        .spans
                        .iter()
                        .map(|span| Span::styled(span.text.clone(), span.style))
                        .collect::<Vec<_>>();
                    Paragraph::new(Line::from(spans).style(line.style)).render(area, &mut buffer);
                    // All gutter cells and the content prefix/text must retain
                    // explicit readable colors after ratatui composes styles.
                    for x in 0..11 {
                        let cell = &buffer[(x, 0)];
                        assert_eq!(cell.bg, background);
                        assert_eq!(
                            cell.fg,
                            if x < 3 {
                                MAIN_UI_THEME.diff.context_text
                            } else {
                                foreground
                            }
                        );
                        let ratio = contrast_ratio(cell.fg, cell.bg);
                        assert!(
                            ratio > 4.5,
                            "{kind:?} staged={staged} cursor={cursor} marked={marked} x={x}: {ratio:.2}:1"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn modal_scrim_clears_batch_and_diff_backgrounds_without_erasing_text() {
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::widgets::{Block, Widget};

    let underlying = [
        MAIN_UI_THEME.batch_selected_row_style(),
        diff_style(PatchLineKind::Added, false, false, false, &MAIN_UI_THEME),
        diff_style(PatchLineKind::Removed, true, false, false, &MAIN_UI_THEME),
        diff_style(PatchLineKind::Added, true, true, false, &MAIN_UI_THEME),
        diff_style(PatchLineKind::Removed, false, false, true, &MAIN_UI_THEME),
        diff_style(PatchLineKind::Context, false, true, false, &MAIN_UI_THEME),
        diff_style(PatchLineKind::Context, false, false, true, &MAIN_UI_THEME),
    ];
    let scrim = MAIN_UI_THEME.modal_scrim_style();
    let expected_background = MAIN_UI_THEME.modal.scrim.unwrap_or(Color::Reset);
    assert_eq!(scrim.bg, Some(expected_background));
    for style in underlying {
        let area = Rect::new(0, 0, 3, 1);
        let mut buffer = Buffer::empty(area);
        buffer.set_string(0, 0, "row", style);
        Block::default().style(scrim).render(area, &mut buffer);
        for (x, symbol) in ["r", "o", "w"].into_iter().enumerate() {
            let cell = &buffer[(x as u16, 0)];
            assert_eq!(cell.symbol(), symbol);
            assert_eq!(cell.bg, expected_background, "underlying {style:?}");
            assert_eq!(cell.fg, MAIN_UI_THEME.modal.dim);
        }
    }
}

#[test]
fn tokyo_night_batch_selection_has_normal_text_contrast() {
    let theme = crate::theme::TOKYO_NIGHT_THEME;
    let ratio = contrast_ratio(theme.batch_selection_text, theme.batch_selection);
    assert!(ratio > 4.5, "Tokyo Night batch selection: {ratio:.2}:1");
}

#[test]
fn custom_diff_palette_reaches_every_source_and_selection_state() {
    let mut theme = TOKYO_NIGHT_THEME;
    theme.diff.added_text = Color::Rgb(201, 202, 203);
    theme.diff.removed_text = Color::Rgb(204, 205, 206);
    theme.diff.context_text = Color::Rgb(207, 208, 209);
    theme.diff.gutter = Color::Rgb(210, 211, 212);
    theme.diff.context_marked = Color::Rgb(21, 22, 23);
    theme.diff.context_cursor = Color::Rgb(24, 25, 26);
    for source in 0..2 {
        for state in 0..3 {
            theme.diff.added[source][state] = Color::Rgb(30 + source as u8, 40 + state as u8, 50);
            theme.diff.removed[source][state] = Color::Rgb(60 + source as u8, 70 + state as u8, 80);
        }
    }
    for kind in [
        PatchLineKind::Added,
        PatchLineKind::Removed,
        PatchLineKind::Context,
    ] {
        for staged in [false, true] {
            for cursor in [false, true] {
                for marked in [false, true] {
                    let display = DiffRow {
                        row: PatchRow {
                            text: "content".into(),
                            id: None,
                            kind,
                            old_line: Some(1),
                            new_line: Some(1),
                        },
                        staged,
                        focused: cursor,
                        chosen: marked,
                        marker: cursor,
                    };
                    let default_before = render_row(&display, &MAIN_UI_THEME);
                    let line = render_row(&display, &theme);
                    let index = if cursor { 2 } else { usize::from(marked) };
                    let expected = match kind {
                        PatchLineKind::Added => Some((
                            theme.diff.added_text,
                            theme.diff.added[usize::from(staged)][index],
                        )),
                        PatchLineKind::Removed => Some((
                            theme.diff.removed_text,
                            theme.diff.removed[usize::from(staged)][index],
                        )),
                        _ if cursor => Some((theme.diff.context_text, theme.diff.context_cursor)),
                        _ if marked => Some((theme.diff.context_text, theme.diff.context_marked)),
                        _ => None,
                    };
                    if let Some((foreground, background)) = expected {
                        assert_eq!(line.style.fg, Some(foreground));
                        assert_eq!(line.style.bg, Some(background));
                        assert_eq!(line.spans[0].style.fg, Some(theme.diff.context_text));
                        assert_eq!(line.spans[0].style.bg, Some(background));
                    } else {
                        assert_eq!(line.style, Style::default());
                        assert_eq!(line.spans[0].style.fg, Some(theme.diff.gutter));
                    }
                    assert_eq!(line.spans[1].style, line.style);
                    assert_eq!(default_before, render_row(&display, &MAIN_UI_THEME));
                }
            }
        }
    }
}

#[test]
fn tab_stops_are_relative_to_native_diff_content() {
    assert_eq!(expand_tabs("+\tword\tend"), "+       word    end");
    assert_eq!(expand_tabs("-  \tword"), "-       word");
    assert_eq!(expand_tabs(" 你好\tx"), " 你好   x");
}
