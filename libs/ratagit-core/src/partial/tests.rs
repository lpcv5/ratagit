use super::*;
use crate::patch::{PatchChange, PatchFile, PatchHunk, PatchLine, PatchSide};

fn patch() -> Box<Patch> {
    let side = PatchSide {
        path: Some("file.txt".into()),
        oid: None,
        mode: 0o100644,
    };
    let file = PatchFile {
        source: PatchSource::Worktree,
        change: PatchChange::Modified,
        old: side.clone(),
        new: side,
        binary: false,
        encoded: Some(Vec::new()),
        hunks: vec![PatchHunk {
            old_start: 1,
            old_lines: 3,
            new_start: 1,
            new_lines: 3,
            header: b"@@ -1,3 +1,3 @@\n".to_vec(),
            lines: [
                PatchLineKind::Removed,
                PatchLineKind::Added,
                PatchLineKind::Context,
                PatchLineKind::Removed,
                PatchLineKind::Added,
            ]
            .into_iter()
            .map(|kind| PatchLine {
                kind,
                old_line: Some(1),
                new_line: Some(1),
                content: b"text\n".to_vec(),
            })
            .collect(),
        }],
    };
    Box::new(Patch {
        preview: None,
        request: PatchRequest::File("file.txt".into()),
        files: vec![file],
        truncated: false,
    })
}

fn state() -> AppContext {
    let mut state = AppContext::default();
    state.ui.details.patch.active = true;
    state.ui.focus = PanelFocus::Details;
    state.repo.details.patch = Some(patch());
    state
}

#[test]
fn test_block_hunk_line_modes_and_range() {
    let patch = patch();
    assert_eq!(
        groups(&patch, SelectionMode::Block, false)
            .iter()
            .map(Vec::len)
            .collect::<Vec<_>>(),
        vec![2, 2]
    );
    assert_eq!(groups(&patch, SelectionMode::Hunk, false).len(), 1);
    assert_eq!(groups(&patch, SelectionMode::Line, false).len(), 4);
    let mut state = state();
    toggle_range(&mut state);
    move_selection(&mut state, false, 8);
    assert_eq!(selected_ids(&state).len(), 2);
    assert!(
        matches!(apply(&mut state, false).as_slice(), [Command::ApplyPatch { ids, .. }] if ids.len() == 2)
    );
}

#[test]
fn test_stale_load_does_not_replace_current_request() {
    let mut state = state();
    let request = PatchRequest::File("file.txt".into());
    let notices = state.notices.clone();
    state.work.details.patch_request = Some((3, request.clone()));
    loaded(&mut state, 2, request.clone(), Err("stale".into()));
    assert!(state.repo.details.patch.is_some());
    assert!(state.work.details.patch_request.is_some());
    assert_eq!(state.notices, notices);
    close(&mut state);
    loaded(&mut state, 3, request, Ok(patch()));
    assert!(state.repo.details.patch.is_none());
}

#[test]
fn test_discard_requires_reviewed_confirmation_and_no_pending_mutation() {
    let mut state = state();
    assert!(apply(&mut state, true).is_empty());
    request_discard(&mut state);
    let ids = state.ui.details.patch.discard.clone().unwrap();
    assert!(
        matches!(apply(&mut state, true).as_slice(), [Command::ApplyPatch { ids: actual, direction: PatchDirection::Discard, .. }] if actual == &ids)
    );
    assert!(apply(&mut state, false).is_empty());
}

#[test]
fn test_source_switch_preserves_independent_selections() {
    let mut state = state();
    let mut staged = state.repo.details.patch.as_ref().unwrap().files[0].clone();
    staged.source = PatchSource::Index;
    state
        .repo
        .details
        .patch
        .as_mut()
        .unwrap()
        .files
        .push(staged);
    move_selection(&mut state, false, 10);
    assert_eq!(state.ui.details.patch.selected, [1, 0]);
    switch_source(&mut state);
    assert_eq!(selected_ids(&state)[0].file, 1);
    switch_source(&mut state);
    assert_eq!(state.ui.details.patch.selected, [1, 0]);
    assert_eq!(selected_ids(&state)[0].line, 3);
}

#[test]
fn test_patch_completion_preserves_viewport_and_refreshes() {
    let mut state = state();
    state.ui.details.scroll_offset = 4;
    assert_eq!(
        applied(&mut state, PatchDirection::Stage, Ok(())),
        vec![Command::RefreshFiles]
    );
    assert_eq!(state.ui.details.scroll_offset, 4);
    crate::details::reset_scroll(&mut state);
    assert_eq!(state.ui.details.scroll_offset, 4);
    assert!(state.notices.last().unwrap().contains("completed"));
}
#[test]
fn test_eof_markers_do_not_split_replacement_blocks() {
    let mut patch = patch();
    patch.files[0].hunks[0].lines.insert(
        1,
        PatchLine {
            kind: PatchLineKind::NoNewline,
            old_line: None,
            new_line: None,
            content: b"\n\\ No newline at end of file\n".to_vec(),
        },
    );
    assert_eq!(groups(&patch, SelectionMode::Block, false)[0].len(), 2);
}

#[test]
fn test_mode_change_keeps_current_block_and_side_viewports() {
    let mut state = state();
    move_selection(&mut state, false, 4);
    let id = selected_ids(&state)[0];
    set_mode(&mut state, SelectionMode::Line);
    assert_eq!(selected_ids(&state)[0], id);
    state.ui.details.scroll_offset = 4;
    switch_source(&mut state);
    state.ui.details.scroll_offset = 1;
    switch_source(&mut state);
    assert_eq!(state.ui.details.scroll_offset, 4);
}
