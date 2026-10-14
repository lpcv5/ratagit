//! Exact, reviewable terminal snapshots without a real terminal or snapshot dependency.
use ratatui::{backend::TestBackend, buffer::Buffer, layout::Position, Frame, Terminal};
use std::fmt::Write;
use std::path::Path;

pub struct RenderedUi {
    pub buffer: Buffer,
    pub cursor: Option<Position>,
}

pub fn draw(terminal: &mut Terminal<TestBackend>, render: impl FnOnce(&mut Frame)) -> RenderedUi {
    let frame = terminal.draw(render).expect("render UI into TestBackend");
    let buffer = frame.buffer.clone();
    let backend = terminal.backend();
    let cursor = backend.cursor_visible().then(|| backend.cursor_position());
    RenderedUi { buffer, cursor }
}

pub fn render(width: u16, height: u16, render: impl FnOnce(&mut Frame)) -> RenderedUi {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("create test terminal");
    draw(&mut terminal, render)
}

/// Preserve every cell's symbol (including wide-character continuation cells),
/// style and diff flags. Styles are indexed in first-seen order for compact diffs.
/// Multi-codepoint/empty symbols have explicit boundaries below the readable rows.
#[allow(deprecated)]
pub fn serialize(ui: &RenderedUi) -> String {
    let area = ui.buffer.area;
    let mut output = format!(
        "UI buffer v1\narea: {area:?}\ncursor: {:?}\n\ntext:\n",
        ui.cursor
    );
    let mut palette = Vec::new();
    let mut style_rows = Vec::new();
    let mut exceptional_symbols = Vec::new();
    for y in area.top()..area.bottom() {
        let mut text = String::new();
        let mut runs: Vec<(u16, u16, usize)> = Vec::new();
        for x in area.left()..area.right() {
            let cell = &ui.buffer[(x, y)];
            text.push_str(cell.symbol());
            if cell.symbol().chars().count() != 1 {
                exceptional_symbols.push((x, y, cell.symbol()));
            }
            let style = format!(
                "{:?}; diff={:?}; skip={}",
                cell.style(),
                cell.diff_option,
                cell.skip
            );
            let index = palette
                .iter()
                .position(|entry| entry == &style)
                .unwrap_or_else(|| {
                    palette.push(style);
                    palette.len() - 1
                });
            if let Some(last) = runs.last_mut().filter(|last| last.2 == index) {
                last.1 = x + 1;
            } else {
                runs.push((x, x + 1, index));
            }
        }
        writeln!(output, "{y:02}: {}", serde_json::to_string(&text).unwrap()).unwrap();
        style_rows.push((y, runs));
    }
    writeln!(
        output,
        "\nsymbol boundaries: {}",
        serde_json::to_string(&exceptional_symbols).unwrap()
    )
    .unwrap();
    output.push_str("\nstyles:\n");
    for (index, style) in palette.iter().enumerate() {
        writeln!(output, "{index}: {style}").unwrap();
    }
    output.push_str("\nstyle runs (x start..end exclusive = style):\n");
    for (y, runs) in style_rows {
        write!(output, "{y:02}:").unwrap();
        for (start, end, style) in runs {
            write!(output, " {start}..{end}={style}").unwrap();
        }
        output.push('\n');
    }
    output
}

pub fn assert_golden(name: &str, ui: &RenderedUi) {
    assert!(name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_'));
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/ui/goldens")
        .join(format!("{name}.golden"));
    let actual = serialize(ui);
    if std::env::var("RATAGIT_UPDATE_GOLDEN").as_deref() == Ok("1") {
        assert!(
            std::env::var_os("CI").is_none(),
            "golden updates are disabled in CI"
        );
        std::fs::create_dir_all(path.parent().unwrap()).expect("create golden directory");
        std::fs::write(&path, actual).expect("write golden buffer");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("Cannot read {}: {error}. Run with RATAGIT_UPDATE_GOLDEN=1 to create baselines, then review the diff.", path.display())
    }).replace("\r\n", "\n");
    pretty_assertions::assert_eq!(
        expected,
        actual,
        "UI golden changed: {}. Review the change before updating with RATAGIT_UPDATE_GOLDEN=1.",
        path.display()
    );
}

#[test]
fn test_serialization_detects_cell_and_cursor_changes() {
    let mut ui = RenderedUi {
        buffer: Buffer::empty(ratatui::layout::Rect::new(0, 0, 3, 1)),
        cursor: None,
    };
    let baseline = serialize(&ui);
    ui.buffer[(1, 0)].set_fg(ratatui::style::Color::Red);
    assert_ne!(baseline, serialize(&ui));
    ui.buffer[(1, 0)].reset();
    assert_eq!(baseline, serialize(&ui));
    ui.buffer[(1, 0)].set_symbol("e\u{301}");
    let grapheme = serialize(&ui);
    ui.buffer[(1, 0)].set_symbol("e");
    ui.buffer[(2, 0)].set_symbol("\u{301} ");
    assert_ne!(grapheme, serialize(&ui));
    ui.buffer.reset();
    ui.cursor = Some(Position::new(1, 0));
    assert_ne!(baseline, serialize(&ui));
}
