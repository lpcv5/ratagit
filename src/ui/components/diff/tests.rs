use super::*;
use crate::git::patch::{
    Patch, PatchChange, PatchFile, PatchHunk, PatchLine, PatchLineKind, PatchRequest, PatchSide,
    PatchSource,
};
use crate::ui::theme::UiTheme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::StatefulWidget,
};

mod changes;
mod viewport;

fn patch(source: PatchSource, count: u32) -> Patch {
    let side = PatchSide {
        path: Some("file.txt".into()),
        oid: None,
        mode: 0o100644,
    };
    Patch {
        request: PatchRequest::File("file.txt".into()),
        truncated: false,
        preview: None,
        files: vec![PatchFile {
            source,
            change: PatchChange::Modified,
            old: side.clone(),
            new: side,
            binary: false,
            encoded: None,
            hunks: vec![PatchHunk {
                old_start: 0,
                old_lines: 0,
                new_start: 1,
                new_lines: count,
                header: format!("@@ -0,0 +1,{count} @@\n").into_bytes(),
                lines: (1..=count)
                    .map(|number| PatchLine {
                        kind: PatchLineKind::Added,
                        old_line: None,
                        new_line: Some(number),
                        content: format!("line {number}\n").into_bytes(),
                    })
                    .collect(),
            }],
        }],
    }
}

fn row(buffer: &Buffer, y: u16) -> String {
    (buffer.area.left()..buffer.area.right())
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}

fn draw(model: DiffViewModel<'_>, theme: &UiTheme, state: &mut DiffViewState) -> Buffer {
    let area = Rect::new(0, 0, 80, 10);
    let mut buffer = Buffer::empty(area);
    StatefulWidget::render(DiffView::new(model, theme), area, &mut buffer, state);
    buffer
}

#[test]
fn test_view_accepts_data_theme_and_non_origin_container_without_app_state() {
    let patch = patch(PatchSource::Index, 3);
    let lines = patch.display_lines();
    let theme = UiTheme {
        diff_staged_added_bg: Color::Indexed(237),
        ..UiTheme::default()
    };
    let model = DiffViewModel {
        lines: &lines,
        patch: Some(&patch),
        status: DiffStatus::ReadOnly,
        ..Default::default()
    };
    let area = Rect::new(3, 2, 80, 10);
    let mut buffer = Buffer::empty(Rect::new(0, 0, 90, 14));
    buffer.set_string(0, 0, "outside", Style::default());
    let before = buffer.clone();
    StatefulWidget::render(
        DiffView::new(model, &theme),
        area,
        &mut buffer,
        &mut DiffViewState::default(),
    );
    for y in 0..14 {
        for x in 0..90 {
            if !area.contains((x, y).into()) {
                assert_eq!(buffer[(x, y)], before[(x, y)]);
            }
        }
    }
    assert!(row(&buffer, 2).contains("Diff · file.txt"));
    assert!(row(&buffer, 11).contains("Read only"));
    assert_eq!(buffer[(50, 6)].bg, theme.diff_staged_added_bg);
}

#[test]
fn test_view_state_retains_loading_preview_and_is_isolated_per_container() {
    let patch = patch(PatchSource::Worktree, 40);
    let lines = patch.display_lines();
    let model = DiffViewModel {
        lines: &lines,
        patch: Some(&patch),
        scroll: 20,
        ..Default::default()
    };
    let theme = UiTheme::default();
    let mut first_state = DiffViewState::default();
    let first = draw(model, &theme, &mut first_state);
    assert!(row(&first, 1).contains("line 18"));
    let loading = draw(
        DiffViewModel {
            scroll: 0,
            status: DiffStatus::Loading,
            ..model
        },
        &theme,
        &mut first_state,
    );
    for y in 1..9 {
        for x in 0..80 {
            assert_eq!(first[(x, y)], loading[(x, y)]);
        }
    }
    let other = draw(
        DiffViewModel { scroll: 0, ..model },
        &theme,
        &mut DiffViewState::default(),
    );
    assert!(row(&other, 4).contains("line 1"));
    assert_eq!(
        row(&first, 1),
        row(&draw(model, &theme, &mut first_state), 1)
    );
}

#[test]
fn test_external_callback_updates_controlled_selection_through_the_data_entry() {
    let patch = patch(PatchSource::Worktree, 3);
    let lines = patch.display_lines();
    let ids = patch.line_ids();
    let selection = DiffSelection {
        focused: true,
        mode: DiffSelectionMode::Line,
        hunk_cursor: Some(ids[0]),
        line_cursor: Some(ids[0]),
        position: DiffPosition {
            current: 1,
            total: ids.len(),
        },
        ..Default::default()
    };
    let model = DiffViewModel {
        lines: &lines,
        patch: Some(&patch),
        selection,
        ..Default::default()
    };
    let theme = UiTheme::default();
    let mut state = DiffViewState::default();
    let before = draw(model, &theme, &mut state);
    let bindings = |intent| match intent {
        DiffIntent::NextLine => Some(ids[1]),
        _ => None,
    };
    let next = bindings.resolve_diff_intent(DiffIntent::NextLine).unwrap();
    assert!(bindings
        .resolve_diff_intent(DiffIntent::StageSelection)
        .is_none());
    assert_eq!(model.selection.line_cursor, Some(ids[0]));
    assert_eq!(before[(1, 4)].symbol(), ">");
    let after = draw(
        DiffViewModel {
            selection: DiffSelection {
                line_cursor: Some(next),
                position: DiffPosition {
                    current: 2,
                    total: ids.len(),
                },
                ..selection
            },
            ..model
        },
        &theme,
        &mut state,
    );
    assert_eq!(after[(1, 4)].symbol(), " ");
    assert_eq!(after[(1, 5)].symbol(), ">");
}
