use super::*;

#[test]
fn test_line_range_can_reverse_direction_and_freeze_then_clear() {
    let mut state = loaded(&[PatchSource::Worktree, PatchSource::Worktree]);
    state.update(DiffAction::ToggleFocus, true);
    state.update(DiffAction::ToggleLineMode, true);
    for _ in 0..3 {
        state.update(DiffAction::NextLine, true);
    }
    state.update(DiffAction::ToggleRange, true);
    state.update(DiffAction::PreviousLine, true);
    state.update(DiffAction::PreviousLine, true);
    let range = state.view().hunks(true).marked_lines();
    assert_eq!(range.len(), 3);
    assert_eq!(
        state.view().hunks(true).line_targets(PatchDirection::Stage),
        range
    );
    state.update(DiffAction::ToggleRange, true);
    assert!(state.interaction().range_anchor.is_none());
    state.update(DiffAction::PreviousLine, true);
    assert_eq!(state.view().hunks(true).marked_lines(), range);
    state.update(DiffAction::ClearSelection, true);
    assert!(!state.interaction().has_selection());
    assert_eq!(
        state
            .view()
            .hunks(true)
            .line_targets(PatchDirection::Stage)
            .len(),
        1
    );
}

#[test]
fn test_line_mixed_sources_disable_range_writes_and_toggle_starts_new_group() {
    let mut state = loaded(&[PatchSource::Worktree, PatchSource::Index]);
    state.update(DiffAction::ToggleFocus, true);
    state.update(DiffAction::ToggleLineMode, true);
    state.update(DiffAction::ToggleRange, true);
    for _ in 0..2 {
        state.update(DiffAction::NextLine, true);
    }
    for direction in [
        PatchDirection::Stage,
        PatchDirection::Unstage,
        PatchDirection::Discard,
    ] {
        assert!(!state.view().hunks(true).can_apply(direction));
        assert!(state
            .update(DiffAction::ApplySelected(direction), true)
            .write
            .is_none());
    }
    state.update(DiffAction::ClearSelection, true);
    state.update(DiffAction::ToggleSelection, true);
    assert_eq!(state.interaction().selected_lines.len(), 1);
    assert!(state.view().hunks(true).can_apply(PatchDirection::Unstage));
    state.update(DiffAction::PreviousLine, true);
    state.update(DiffAction::ToggleSelection, true);
    assert_eq!(state.interaction().selected_lines.len(), 1);
    assert!(state.view().hunks(true).can_apply(PatchDirection::Stage));
    assert!(!state.view().hunks(true).can_apply(PatchDirection::Unstage));
}

#[test]
fn test_line_writes_are_disabled_for_history_truncated_binary_modes_and_other_panels() {
    for source in [
        PatchSource::Worktree,
        PatchSource::Commit("history".into()),
        PatchSource::Stash(0),
    ] {
        let mut state = loaded(std::slice::from_ref(&source));
        state.update(DiffAction::ToggleFocus, false);
        state.update(DiffAction::ToggleLineMode, false);
        for direction in [
            PatchDirection::Stage,
            PatchDirection::Unstage,
            PatchDirection::Discard,
        ] {
            assert!(state
                .update(DiffAction::ApplySelected(direction), false)
                .write
                .is_none());
            if source != PatchSource::Worktree {
                assert!(state
                    .update(DiffAction::ApplySelected(direction), true)
                    .write
                    .is_none());
            }
        }
    }
    for kind in 0..3 {
        let mut patch = (*patch(&[PatchSource::Worktree])).clone();
        match kind {
            0 => patch.truncated = true,
            1 => patch.files[0].binary = true,
            _ => patch.files[0].new.mode = 0o100755,
        }
        let mut state = DiffState::default();
        state.set_test_document(document(Arc::new(patch)));
        state.update(DiffAction::ToggleFocus, true);
        state.update(DiffAction::ToggleLineMode, true);
        assert!(state
            .update(DiffAction::ApplySelected(PatchDirection::Stage), true)
            .write
            .is_none());
        assert!(state
            .update(DiffAction::ApplySelected(PatchDirection::Discard), true)
            .write
            .is_none());
    }
}

#[test]
fn test_line_pending_write_blocks_duplicates_and_old_completion_and_loading_clears_ranges() {
    let mut state = loaded(&[PatchSource::Worktree]);
    state.update(DiffAction::ToggleFocus, true);
    state.update(DiffAction::ToggleLineMode, true);
    state.update(DiffAction::ToggleRange, true);
    let original = state.document().patch.clone().unwrap();
    let output = state.update(DiffAction::ApplySelected(PatchDirection::Discard), true);
    let Some(HunkWrite::Lines {
        id,
        patch,
        ids,
        direction,
    }) = output.write
    else {
        panic!("expected line effect");
    };
    assert!(Arc::ptr_eq(&patch, &original));
    assert_eq!(ids.len(), 1);
    assert_eq!(direction, PatchDirection::Discard);
    assert!(state
        .update(DiffAction::ApplySelected(direction), true)
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
    assert!(!output.refresh_status);
    assert_eq!(state.interaction().pending_write, Some(id));
    let output = state.update(
        DiffAction::WriteFinished {
            id,
            direction,
            result: &Err("stale line".into()),
        },
        true,
    );
    assert!(output.refresh_status);
    assert!(output.reload == DiffReload::Deferred);
    assert!(!state.interaction().has_selection());
    assert!(state.interaction().pending_write.is_none());
    state.schedule_reload(Instant::now());
    assert!(state.interaction().line_cursor.is_none());
    assert!(state.view().hunks(true).line_targets(direction).is_empty());
}
