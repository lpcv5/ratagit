use super::*;
use crate::git::DiffLine;

fn hunks(counts: &[u32]) -> Patch {
    let mut document = patch(PatchSource::Worktree, 1);
    document.files[0].hunks = counts
        .iter()
        .enumerate()
        .map(|(index, count)| {
            let mut hunk = patch(PatchSource::Worktree, *count)
                .files
                .remove(0)
                .hunks
                .remove(0);
            hunk.header = format!("@@ hunk {index} @@\n").into_bytes();
            for (line, data) in hunk.lines.iter_mut().enumerate() {
                data.content = format!("hunk {index} line {line}\n").into_bytes();
            }
            hunk
        })
        .collect();
    document
}

fn model<'a>(patch: &'a Patch, lines: &'a [DiffLine], index: usize) -> DiffViewModel<'a> {
    let span = patch.change_blocks().remove(index);
    DiffViewModel {
        lines,
        patch: Some(patch),
        scroll: span.range.start,
        selection: DiffSelection {
            focused: true,
            hunk_cursor: Some(span.id),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn test_navigation_fits_change_blocks_and_uses_full_height_for_oversized_changes() {
    let patch = hunks(&[2, 6, 2, 7, 10, 3]);
    let lines = patch.display_lines();
    let spans = patch.change_blocks();
    let theme = UiTheme::default();
    let mut state = DiffViewState::default();
    for index in [0, 1, 2, 3, 4, 5, 4, 3, 2, 1, 0] {
        let data = model(&patch, &lines, index);
        let buffer = draw(data, &theme, &mut state);
        let visible: Vec<_> = (1..9).map(|y| row(&buffer, y)).collect();
        let span = &spans[index].range;
        if span.len() <= 8 {
            for line in &lines[span.clone()] {
                assert!(
                    visible
                        .iter()
                        .any(|row| row.contains(line.content.trim_end())),
                    "hunk {index} must show {}",
                    line.content
                );
            }
        } else {
            assert!(visible[0].contains(&format!("hunk {index} line 0")));
            assert!(visible[7].contains(&format!("hunk {index} line 7")));
        }
        assert_eq!(buffer, draw(data, &theme, &mut state));
    }
    // Resize follows the whole range again, including nearly full-height hunks.
    let data = model(&patch, &lines, 3);
    for height in [1, 2, 3, 5, 8, 10, 20] {
        let area = Rect::new(0, 0, 80, height);
        let mut buffer = Buffer::empty(area);
        StatefulWidget::render(DiffView::new(data, &theme), area, &mut buffer, &mut state);
        if height >= 10 {
            assert!((1..height - 1).any(|y| row(&buffer, y).contains("hunk 3 line 6")));
        }
    }
}

#[test]
fn test_anchor_keeps_manual_position_through_loading_request_change_and_repeated_frames() {
    let mut before = patch(PatchSource::Untracked, 40);
    before.request = PatchRequest::Untracked("file.txt".into());
    let lines = before.display_lines();
    let theme = UiTheme::default();
    let mut state = DiffViewState::default();
    let data = model(&before, &lines, 0);
    draw(data, &theme, &mut state);
    let scrolled = DiffViewModel { scroll: 20, ..data };
    let visible = draw(scrolled, &theme, &mut state);
    for _ in 0..2 {
        let loading = draw(
            DiffViewModel {
                status: DiffStatus::Loading,
                scroll: 0,
                selection: DiffSelection {
                    focused: true,
                    ..Default::default()
                },
                ..data
            },
            &theme,
            &mut state,
        );
        for y in 1..9 {
            assert_eq!(row(&visible, y), row(&loading, y));
        }
    }
    let mut after = before.clone();
    after.request = PatchRequest::File("file.txt".into());
    after.files[0].source = PatchSource::Index;
    let lines = after.display_lines();
    let data = DiffViewModel {
        viewport_anchor: Some(DiffViewportAnchor {
            revision: 1,
            previous_row: 3,
            row: 3,
        }),
        ..model(&after, &lines, 0)
    };
    for _ in 0..3 {
        let anchored = draw(data, &theme, &mut state);
        for y in 1..9 {
            assert_eq!(row(&visible, y), row(&anchored, y));
        }
    }
    // Manual scrolling still works after the anchor has been consumed.
    let manual = draw(DiffViewModel { scroll: 21, ..data }, &theme, &mut state);
    assert!(row(&manual, 1).contains("line 19"));
}

#[test]
fn test_anchor_moves_with_content_and_does_not_pull_up_a_shorter_final_page() {
    for remove_before in [false, true] {
        let before = hunks(&[3, 3, 3]);
        let lines = before.display_lines();
        let theme = UiTheme::default();
        let mut state = DiffViewState::default();
        let old_index = if remove_before { 2 } else { 1 };
        let old = draw(model(&before, &lines, old_index), &theme, &mut state);
        let needle = format!("hunk {old_index} line 0");
        let screen_row = (1..9).find(|y| row(&old, *y).contains(&needle)).unwrap();
        let mut after = before.clone();
        after.files[0]
            .hunks
            .remove(if remove_before { 0 } else { 2 });
        let new_index = 1;
        let after_lines = after.display_lines();
        let data = DiffViewModel {
            viewport_anchor: Some(DiffViewportAnchor {
                revision: 1,
                previous_row: before.hunk_spans()[old_index].range.start + 1,
                row: after.hunk_spans()[new_index].range.start + 1,
            }),
            ..model(&after, &after_lines, new_index)
        };
        for _ in 0..3 {
            let buffer = draw(data, &theme, &mut state);
            assert!(row(&buffer, screen_row).contains(&needle));
        }
    }
}
