use super::*;
use crate::file_tree::FileTreeNodeStatus;
use crate::git::patch::{
    DiffDocument, HunkId, Patch, PatchChange, PatchDirection, PatchFile, PatchHunk, PatchLine,
    PatchLineKind, PatchRequest, PatchSide, PatchSource,
};
use crate::git::{DiffLine, DiffLineKind, FileStatus};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn patch(sources: &[PatchSource]) -> Arc<Patch> {
    let request = if sources.len() > 1 {
        PatchRequest::Directory("src".into())
    } else if sources[0] == PatchSource::Index {
        PatchRequest::Index("src/file.txt".into())
    } else {
        PatchRequest::Worktree("src/file.txt".into())
    };
    Arc::new(Patch {
        request,
        files: sources
            .iter()
            .enumerate()
            .map(|(file, source)| {
                let side = PatchSide {
                    path: Some(format!("src/file{file}.txt").into()),
                    oid: None,
                    mode: 0o100644,
                };
                PatchFile {
                    source: source.clone(),
                    change: PatchChange::Modified,
                    old: side.clone(),
                    new: side,
                    binary: false,
                    hunks: (0..2)
                        .map(|hunk| PatchHunk {
                            old_start: 1 + hunk * 10,
                            old_lines: 1,
                            new_start: 1 + hunk * 10,
                            new_lines: 1,
                            header: format!("@@ hunk {hunk} @@\n").into_bytes(),
                            lines: vec![PatchLine {
                                kind: PatchLineKind::Added,
                                old_line: None,
                                new_line: Some(1 + hunk * 10),
                                content: b"added\n".to_vec(),
                            }],
                        })
                        .collect(),
                    encoded: Some(b"owned patch bytes".to_vec()),
                }
            })
            .collect(),
        truncated: false,
    })
}

fn document(patch: Arc<Patch>) -> DiffDocument {
    DiffDocument {
        lines: patch.display_lines(),
        patch: Some(patch),
    }
}

fn loaded(sources: &[PatchSource]) -> DiffState {
    let mut state = DiffState::default();
    state.set_test_document(document(patch(sources)));
    state
}

fn file_target(name: &str) -> DiffTarget {
    DiffTarget::File {
        path: name.into(),
        status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
    }
}

fn load(state: &mut DiffState, target: DiffTarget, document: DiffDocument) {
    state.schedule_reload(Instant::now());
    let DiffLoad::Request { cache_key, .. } = state.prepare_reload(target.clone(), &[]) else {
        panic!("expected an uncached background request");
    };
    assert!(state.complete_load(cache_key, document, &target, &[]));
}

#[test]
fn test_reload_debounce_uses_the_requested_timestamp() {
    let mut state = DiffState::default();
    let requested_at = Instant::now();
    state.schedule_reload(requested_at);
    assert!(state.is_loading());
    assert!(!state.debounce_elapsed(
        Duration::from_millis(80),
        requested_at + Duration::from_millis(79)
    ));
    assert!(state.debounce_elapsed(
        Duration::from_millis(80),
        requested_at + Duration::from_millis(80)
    ));
}

#[test]
fn test_hunk_navigation_scrolls_and_clamps_without_application_state() {
    let mut state = loaded(&[PatchSource::Worktree]);
    state.update(DiffAction::ToggleFocus, true);
    assert!(state.interaction().focused);
    assert_eq!(state.interaction().side, Some(DiffSide::Unstaged));
    assert!(state
        .view()
        .hunks(true)
        .can_switch_side(&file_target("file.txt")));
    state.update(DiffAction::NextHunk, true);
    let second = HunkId { file: 0, hunk: 1 };
    assert_eq!(state.interaction().cursor, Some(second));
    let span = state
        .document()
        .patch
        .as_ref()
        .unwrap()
        .hunk_spans()
        .into_iter()
        .find(|span| span.id == second)
        .unwrap();
    assert_eq!(state.scroll(), span.range.start);
    state.update(DiffAction::NextHunk, true);
    assert_eq!(state.interaction().cursor, Some(second));
    state.update(DiffAction::PreviousHunk, true);
    state.update(DiffAction::PreviousHunk, true);
    assert_eq!(
        state.interaction().cursor,
        Some(HunkId { file: 0, hunk: 0 })
    );
}

#[test]
fn test_mixed_direction_selection_starts_a_new_group() {
    let mut state = loaded(&[PatchSource::Worktree, PatchSource::Index]);
    state.update(DiffAction::ToggleFocus, true);
    state.update(DiffAction::ToggleSelection, true);
    state.update(DiffAction::NextHunk, true);
    state.update(DiffAction::ToggleSelection, true);
    assert_eq!(state.interaction().selected.len(), 2);
    state.update(DiffAction::NextHunk, true);
    state.update(DiffAction::ToggleSelection, true);
    let index_hunk = HunkId { file: 1, hunk: 0 };
    assert_eq!(
        state
            .interaction()
            .selected
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        vec![index_hunk]
    );
    assert!(state
        .view()
        .hunks(true)
        .targets(PatchDirection::Stage)
        .is_empty());
    assert_eq!(
        state.view().hunks(true).targets(PatchDirection::Unstage),
        vec![index_hunk]
    );
}

#[test]
fn test_read_only_or_non_files_context_never_emits_hunk_writes() {
    for source in [PatchSource::Worktree, PatchSource::Commit("history".into())] {
        let mut state = loaded(&[source]);
        state.update(DiffAction::ToggleFocus, false);
        state.update(DiffAction::NextHunk, false);
        assert_eq!(state.interaction().cursor.unwrap().hunk, 1);
        assert!(state
            .update(DiffAction::ApplySelected(PatchDirection::Stage), false)
            .write
            .is_none());
    }
    let mut state = loaded(&[PatchSource::Stash(3)]);
    state.update(DiffAction::ToggleFocus, true);
    assert!(state
        .update(DiffAction::ApplySelected(PatchDirection::Stage), true)
        .write
        .is_none());
}

#[test]
fn test_write_identity_blocks_duplicates_and_ignores_stale_completions() {
    let mut state = loaded(&[PatchSource::Worktree]);
    let original = state.document().patch.clone().unwrap();
    state.update(DiffAction::ToggleFocus, true);
    let output = state.update(DiffAction::ApplySelected(PatchDirection::Stage), true);
    let Some(HunkWrite::Selected {
        id,
        patch,
        ids,
        direction,
    }) = output.write
    else {
        panic!("expected an owned hunk write");
    };
    assert!(Arc::ptr_eq(&original, &patch));
    assert_eq!(ids, vec![HunkId { file: 0, hunk: 0 }]);
    assert_eq!(direction, PatchDirection::Stage);
    assert_eq!(state.interaction().pending_write, Some(id));
    assert!(state
        .update(DiffAction::ApplySelected(PatchDirection::Stage), true)
        .write
        .is_none());

    let output = state.update(
        DiffAction::WriteFinished {
            id: id + 1,
            direction,
            result: &Ok(()),
        },
        true,
    );
    assert_eq!(state.interaction().pending_write, Some(id));
    assert!(output.log.is_none());
    assert!(!output.refresh_status);
    assert!(output.reload == DiffReload::None);
    assert!(!output.redraw);

    // Context changes must retain the write identity until its completion.
    state.update(DiffAction::SelectionChanged, false);
    assert_eq!(state.interaction().pending_write, Some(id));
    let output = state.update(
        DiffAction::WriteFinished {
            id,
            direction,
            result: &Err("patch changed".into()),
        },
        false,
    );
    assert!(state.interaction().pending_write.is_none());
    assert!(state.interaction().selected.is_empty());
    assert_eq!(output.log, Some(("patch changed".to_string(), false)));
    assert!(output.refresh_status);
    assert!(output.reload == DiffReload::Deferred);
    assert!(output.redraw);

    state.set_test_document(document(original));
    state.update(DiffAction::ToggleFocus, true);
    let next = state.update(DiffAction::ApplySelected(direction), true);
    assert!(matches!(next.write, Some(HunkWrite::Selected { id: next_id, .. }) if next_id > id));
}

#[test]
fn test_loading_keeps_preview_and_disables_hunk_selectors() {
    let mut state = loaded(&[PatchSource::Worktree]);
    state.update(DiffAction::ToggleFocus, true);
    state.update(DiffAction::ToggleSelection, true);
    let preview = state.document().lines.len();
    state.schedule_reload(Instant::now());
    let view = state.view();
    assert_eq!(view.document.lines.len(), preview);
    assert!(view.document.patch.is_none());
    assert!(view.loading);
    assert!(!view.show_loading());
    assert!(view.hunks(true).ids().is_empty());
    assert!(!view.hunks(true).can_navigate());
    assert!(!view.hunks(true).can_apply(PatchDirection::Stage));
    assert!(!view.hunks(true).can_switch_side(&file_target("file.txt")));
    assert!(view.hunks(true).targets(PatchDirection::Stage).is_empty());
    assert!(view.interaction.selected.is_empty());
    assert!(view.interaction.cursor.is_none());

    let mut empty = DiffState::default();
    empty.schedule_reload(Instant::now());
    assert!(empty.view().show_loading());
}

#[test]
fn test_side_switch_returns_immediate_reload_intent() {
    let mut state = loaded(&[PatchSource::Index]);
    state.update(DiffAction::ToggleFocus, true);
    let output = state.update(DiffAction::SwitchSide, true);
    assert_eq!(state.interaction().side, Some(DiffSide::Unstaged));
    assert!(output.reload == DiffReload::Immediate);
    assert!(output.write.is_none());
    // Timing and runtime task cancellation are handled by the coordinator.
    assert!(!state.has_pending_reload());
    assert!(matches!(
        file_target("file.txt").with_side(state.interaction().side),
        DiffTarget::File {
            status: FileTreeNodeStatus::Unstaged(_),
            ..
        }
    ));
    state.schedule_reload(Instant::now());
    let output = state.update(DiffAction::SwitchSide, true);
    assert!(output.reload == DiffReload::None);
    assert_eq!(state.interaction().side, Some(DiffSide::Unstaged));
}

#[test]
fn test_cache_reuse_preserves_patch_identity_and_resets_cursor() {
    let mut state = DiffState::default();
    let original = patch(&[PatchSource::Worktree]);
    let target = file_target("file.txt");
    load(&mut state, target.clone(), document(original.clone()));
    state.update(DiffAction::ToggleFocus, true);
    state.update(DiffAction::NextHunk, true);
    state.update(DiffAction::ToggleSelection, true);
    state.schedule_reload(Instant::now());
    assert!(matches!(state.prepare_reload(target, &[]), DiffLoad::Ready));
    assert!(!state.is_loading());
    assert!(Arc::ptr_eq(
        state.document().patch.as_ref().unwrap(),
        &original
    ));
    assert_eq!(
        state.interaction().cursor,
        Some(HunkId { file: 0, hunk: 0 })
    );
    assert!(state.interaction().selected.is_empty());
    assert_eq!(state.scroll(), 0);
}

#[test]
fn test_selection_mismatch_does_not_publish_or_cache_obsolete_content() {
    let mut state = DiffState::default();
    let old_target = file_target("old.txt");
    state.schedule_reload(Instant::now());
    let DiffLoad::Request { cache_key, .. } = state.prepare_reload(old_target.clone(), &[]) else {
        panic!("expected a background load");
    };
    assert!(!state.complete_load(
        cache_key,
        document(patch(&[PatchSource::Worktree])),
        &file_target("new.txt"),
        &[],
    ));
    assert!(state.document().lines.is_empty());
    assert!(state.document().patch.is_none());
    state.schedule_reload(Instant::now());
    assert!(matches!(
        state.prepare_reload(old_target, &[]),
        DiffLoad::Request { .. }
    ));
}

#[test]
fn test_refresh_invalidates_worktree_and_refs_but_keeps_commit_cache() {
    let mut state = DiffState::default();
    let file = file_target("file.txt");
    let branch = DiffTarget::Branch {
        name: "main".into(),
    };
    let commit = DiffTarget::Commit {
        oid: "abc".into(),
        path: None,
    };
    let old_patch = patch(&[PatchSource::Commit("abc".into())]);
    load(
        &mut state,
        file.clone(),
        document(patch(&[PatchSource::Worktree])),
    );
    load(&mut state, branch.clone(), Vec::new().into());
    load(&mut state, commit.clone(), document(old_patch.clone()));
    state.repository_refreshed(false);

    state.schedule_reload(Instant::now());
    assert!(matches!(
        state.prepare_reload(branch.clone(), &[]),
        DiffLoad::Ready
    ));
    state.repository_refreshed(true);
    state.schedule_reload(Instant::now());
    assert!(matches!(
        state.prepare_reload(branch, &[]),
        DiffLoad::Request { .. }
    ));
    state.schedule_reload(Instant::now());
    assert!(matches!(
        state.prepare_reload(file, &[]),
        DiffLoad::Request { .. }
    ));
    state.schedule_reload(Instant::now());
    assert!(matches!(state.prepare_reload(commit, &[]), DiffLoad::Ready));
    assert!(Arc::ptr_eq(
        state.document().patch.as_ref().unwrap(),
        &old_patch
    ));
}

#[test]
fn test_same_pending_target_reuses_in_flight_read() {
    let mut state = DiffState::default();
    let target = file_target("file.txt");
    state.schedule_reload(Instant::now());
    assert!(matches!(
        state.prepare_reload(target.clone(), &[]),
        DiffLoad::Request { .. }
    ));
    state.schedule_reload(Instant::now());
    assert!(matches!(
        state.prepare_reload(target, &[]),
        DiffLoad::Unchanged
    ));
    assert!(state.is_loading());
    assert!(!state.has_pending_reload());
    state.finish_load();
    assert!(!state.is_loading());
}

#[test]
fn test_scroll_bounds_for_textual_diff_without_patch() {
    let mut state = DiffState::default();
    state.update(DiffAction::ScrollDown, false);
    assert_eq!(state.scroll(), 0);
    state.set_test_document(
        vec![
            DiffLine {
                kind: DiffLineKind::Context,
                content: "log entry".into(),
            };
            3
        ]
        .into(),
    );
    state.update(DiffAction::ScrollDown, false);
    assert_eq!(state.scroll(), 2);
    state.update(DiffAction::ScrollUp, false);
    assert_eq!(state.scroll(), 0);
    state.update(DiffAction::ToggleFocus, false);
    assert!(!state.interaction().focused);
}

#[test]
fn test_untracked_and_revision_targets_ignore_worktree_side_choice() {
    let target = DiffTarget::File {
        path: "new.txt".into(),
        status: FileTreeNodeStatus::Untracked,
    };
    assert!(matches!(
        target.with_side(Some(DiffSide::Staged)),
        DiffTarget::File {
            status: FileTreeNodeStatus::Untracked,
            ..
        }
    ));
    let target = DiffTarget::Commit {
        oid: "abc".into(),
        path: Some("file.txt".into()),
    };
    let original_key = target.cache_key(&[]);
    let target = target.with_side(Some(DiffSide::Staged));
    assert_eq!(target.cache_key(&[]), original_key);
    assert!(target.affected_by(DiffRefreshSource::Commits));
    assert!(!target.affected_by(DiffRefreshSource::Status));
}
