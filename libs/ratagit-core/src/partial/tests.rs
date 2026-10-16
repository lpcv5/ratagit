use super::*;
use crate::patch::{PatchChange, PatchFile, PatchHunk, PatchLine, PatchSide};

fn patch() -> Box<Patch> {
    let side = PatchSide {
        path: Some("file.txt".into()),
        oid: None,
        mode: 0o100644,
    };
    let file = PatchFile {
        header: b"diff --git a/file.txt b/file.txt\nindex 1111111..2222222 100644\n--- a/file.txt\n+++ b/file.txt\n".to_vec(),
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
fn historical_patch_load_owns_target_and_accumulation_cannot_fall_through_to_staging() {
    let mut state = state();
    let request = PatchRequest::CommitFiles {
        oid: "old-commit".into(),
        paths: vec!["file.txt".into()],
    };
    let mut reviewed = patch();
    reviewed.request = request.clone();
    reviewed.files[0].source = PatchSource::Commit("old-commit".into());
    state.ui.details.patch.mode = SelectionMode::Hunk;
    state.work.details.patch_request = Some((4, request.clone()));
    loaded(&mut state, 3, request.clone(), Ok(reviewed.clone()));
    assert!(state.ui.workflow.dialog.is_none());
    loaded(
        &mut state,
        4,
        PatchRequest::Commit {
            oid: "other".into(),
            path: None,
        },
        Ok(reviewed.clone()),
    );
    assert!(state.ui.workflow.dialog.is_none());
    loaded(&mut state, 4, request, Ok(reviewed));
    assert!(state.ui.workflow.dialog.is_none());
    assert!(apply(&mut state, false).is_empty());
    assert_eq!(state.ui.custom_patch.included.len(), 1);
    assert!(state.ui.workflow.dialog.is_none());
    request_discard(&mut state);
    assert!(matches!(
        state.ui.workflow.dialog,
        Some(crate::workflow::Dialog::Confirm(
            crate::workflow::Operation::CommitPatch {
                action: crate::workflow::commit_files::PatchAction::Discard,
                ..
            }
        ))
    ));
    assert!(state.ui.details.patch.discard.is_none());
    close(&mut state);
    assert!(state.work.details.patch_request.is_none());
}

#[test]
fn historical_file_range_rejects_incomplete_bytes_without_partial_inclusion() {
    let mut state = state();
    let patch = state.repo.details.patch.as_mut().unwrap();
    patch.request = PatchRequest::CommitFiles {
        oid: "commit".into(),
        paths: vec!["file.txt".into(), "binary.dat".into()],
    };
    patch.files[0].source = PatchSource::Commit("commit".into());
    let mut binary = patch.files[0].clone();
    binary.binary = true;
    binary.encoded = None;
    binary.hunks.clear();
    binary.old.path = Some("binary.dat".into());
    binary.new.path = Some("binary.dat".into());
    patch.files.push(binary);
    let reviewed = patch.clone();
    state.ui.custom_patch.source = Some("commit".into());
    state.work.custom_patch.pending = Some(crate::custom_patch::Pending::Load {
        id: 1,
        request: reviewed.request.clone(),
    });
    crate::custom_patch::loaded(&mut state, 1, reviewed.request.clone(), Ok(reviewed));
    assert!(state.ui.custom_patch.included.is_empty());
    assert!(state.ui.workflow.dialog.is_none());
    assert!(
        state
            .notices
            .last()
            .unwrap()
            .contains("complete native file bytes")
    );
    assert!(apply(&mut state, false).is_empty());
    assert_eq!(state.ui.custom_patch.included.len(), 1);
    cycle_mode(&mut state);
    cycle_mode(&mut state);
    cycle_mode(&mut state);
    assert_eq!(state.ui.details.patch.mode, SelectionMode::Block);
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
    assert_eq!(groups(&patch, SelectionMode::Line, false).len(), 5);
    let mut state = state();
    toggle_range(&mut state);
    move_selection(&mut state, false, 8);
    assert_eq!(selected_ids(&state).len(), 4);
    assert!(
        matches!(apply(&mut state, false).as_slice(), [Command::ApplyPatch { ids, .. }] if ids.len() == 4)
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

#[test]
fn test_visual_hunk_range_freezes_without_forcing_line_mode() {
    let mut state = state();
    toggle_range(&mut state);
    move_selection(&mut state, false, 8);
    assert_eq!(state.ui.details.patch.mode, SelectionMode::Block);
    assert_eq!(selected_ids(&state).len(), 4);
    toggle_range(&mut state);
    assert!(state.ui.details.patch.anchor.is_none());
    move_selection(&mut state, true, 8);
    assert_eq!(selected_ids(&state).len(), 4);
    clear_marks(&mut state);
    assert_eq!(selected_ids(&state).len(), 2);
}

#[test]
fn test_independent_marks_exclude_unmarked_cursor() {
    let mut state = state();
    toggle_mark(&mut state);
    move_selection(&mut state, false, 8);
    assert_eq!(selected_ids(&state)[0].line, 0);
    toggle_mark(&mut state);
    assert_eq!(selected_ids(&state).len(), 4);
    toggle_mark(&mut state);
    assert_eq!(selected_ids(&state).len(), 2);
}

#[test]
fn test_line_range_navigates_context_and_filters_it_from_writes() {
    let mut state = state();
    cycle_mode(&mut state);
    assert_eq!(state.ui.details.patch.mode, SelectionMode::Line);
    move_selection(&mut state, false, 8);
    toggle_range(&mut state);
    move_selection(&mut state, false, 8);
    assert_eq!(cursor_ids(&state)[0].line, 2);
    assert_eq!(selected_ids(&state).len(), 1);
    clear_marks(&mut state);
    assert!(selected_ids(&state).is_empty());
    assert!(apply(&mut state, false).is_empty());
}

#[test]
fn test_resize_follows_live_range_cursor_instead_of_anchor() {
    let mut state = state();
    state.ui.details.viewport_lines = 4;
    toggle_range(&mut state);
    move_selection(&mut state, false, 4);
    let scroll = state.ui.details.scroll_offset;
    assert!(scroll > 0);
    ensure_visible(&mut state);
    assert_eq!(state.ui.details.scroll_offset, scroll);
}

#[test]
fn test_clamped_movement_preserves_manual_scroll() {
    let mut state = state();
    state.ui.details.scroll_offset = 4;
    move_selection(&mut state, true, 5);
    assert_eq!(state.ui.details.scroll_offset, 4);
}

#[test]
fn test_write_cursor_follows_reviewed_bytes_to_destination_side() {
    let mut state = state();
    move_selection(&mut state, false, 10);
    assert!(!apply(&mut state, false).is_empty());
    let mut next = patch();
    next.files[0].source = PatchSource::Index;
    applied(&mut state, PatchDirection::Stage, Ok(()));
    let request = next.request.clone();
    state.work.details.patch_request = Some((42, request.clone()));
    loaded(&mut state, 42, request, Ok(next));
    assert!(state.ui.details.patch.staged);
    assert_eq!(state.ui.details.patch.selected[1], 1);
    assert!(state.ui.details.patch.write_position.is_none());
}

#[test]
fn test_unmarked_cursor_survives_removal_of_an_earlier_hunk() {
    let mut state = state();
    let first = state.repo.details.patch.as_ref().unwrap().files[0].hunks[0].clone();
    let file = &mut state.repo.details.patch.as_mut().unwrap().files[0];
    file.hunks = (0..4)
        .map(|index| {
            let mut hunk = first.clone();
            hunk.lines.truncate(2);
            hunk.old_start = index * 20 + 1;
            hunk.new_start = index * 20 + 1;
            for line in &mut hunk.lines {
                line.content = format!("change-{index}\n").into_bytes();
            }
            hunk
        })
        .collect();
    toggle_mark(&mut state);
    move_selection(&mut state, false, 10);
    move_selection(&mut state, false, 10);
    assert!(!apply(&mut state, false).is_empty());
    let mut next = state.repo.details.patch.clone().unwrap();
    next.files[0].hunks.remove(0);
    applied(&mut state, PatchDirection::Stage, Ok(()));
    let request = next.request.clone();
    state.work.details.patch_request = Some((42, request.clone()));
    loaded(&mut state, 42, request, Ok(next));
    assert!(!state.ui.details.patch.staged);
    assert_eq!(state.ui.details.patch.selected[0], 1);
}

#[test]
fn test_patch_viewport_uses_rows_previously_reserved_for_counter_and_help() {
    let mut state = state();
    // Four native file-header rows, one hunk header and five patch lines.
    move_selection(&mut state, false, 10);
    assert_eq!(state.ui.details.scroll_offset, 0);
    crate::details::scroll_down(&mut state, 100, 5);
    assert_eq!(state.ui.details.scroll_offset, 5);
}

#[test]
fn test_patch_rows_preserve_native_headers_hunks_and_content_indentation() {
    let mut patch = patch();
    let file = &mut patch.files[0];
    file.hunks[0].header = b"@@ -1,3 +1,3 @@ section  \n".to_vec();
    file.hunks[0].lines[0].content = b"  indented\ttext  \n".to_vec();
    file.hunks[0].lines.insert(
        1,
        PatchLine {
            kind: PatchLineKind::NoNewline,
            old_line: None,
            new_line: None,
            content: b"\n\\ No newline at end of file\n".to_vec(),
        },
    );
    let rows = patch_rows(&patch);
    assert_eq!(rows[0].text, "diff --git a/file.txt b/file.txt");
    assert_eq!(rows[1].text, "index 1111111..2222222 100644");
    assert_eq!(rows[2].text, "--- a/file.txt");
    assert_eq!(rows[3].text, "+++ b/file.txt");
    assert_eq!(rows[4].text, "@@ -1,3 +1,3 @@ section  ");
    assert_eq!(rows[5].text, "  indented\ttext  ");
    assert_eq!(
        rows[5].id,
        Some(LineId {
            file: 0,
            hunk: 0,
            line: 0
        })
    );
    assert_eq!(rows[6].text, "\\ No newline at end of file");
    assert_eq!(rows[6].id, None);
}

#[test]
fn test_open_retains_preview_scroll_but_requires_new_reviewed_patch_for_writes() {
    let mut state = AppContext::default();
    state.repo.files.items = vec![crate::FileEntry {
        path: "file.txt".into(),
        staged: false,
        unstaged: true,
        untracked: false,
        status: crate::CommitFileStatus::Modified,
        conflicted: false,
    }];
    crate::initialize_tree_if_needed(&state.repo.files.items, &mut state.ui.files);
    state.repo.details.files_diff = "@@ -1 +1 @@\n-old\n+new".into();
    state.ui.details.scroll_offset = 7;
    let commands = open(&mut state);
    assert!(matches!(commands.as_slice(), [Command::LoadPatch { .. }]));
    assert_eq!(state.ui.details.scroll_offset, 7);
    assert_eq!(state.ui.focus, PanelFocus::Details);
    assert!(state.repo.details.patch.is_none());
    assert!(apply(&mut state, false).is_empty());
    request_discard(&mut state);
    assert!(state.ui.details.patch.discard.is_none());
}
