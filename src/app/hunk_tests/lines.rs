use super::*;
use crate::git::patch::{LineId, PatchLineKind};

#[test]
fn test_line_mode_keyboard_range_stage_and_unstage_preserve_worktree() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('a'));
    assert!(fixture.app.diff.interaction().line_mode);
    let file_selection = fixture.app.files.view().panel.selected();
    let first = fixture.app.diff.interaction().line_cursor.unwrap();
    assert_eq!(
        fixture.app.diff.scroll(),
        fixture
            .app
            .diff
            .document()
            .patch
            .as_ref()
            .unwrap()
            .line_row(first)
            .unwrap()
    );
    press(&mut fixture.app, KeyCode::Char('v'));
    press(&mut fixture.app, KeyCode::Char('j'));
    assert_eq!(fixture.app.hunk_view().marked_lines().len(), 2);
    assert_eq!(fixture.app.files.view().panel.selected(), file_selection);
    let rendered = text(&render(&fixture.app, false));
    assert!(rendered.contains("Diff [Line") && rendered.contains("| 2 selected | Range"));
    assert!(rendered.contains("[x] +first change"));
    assert!(help(&fixture.app, "hunk stage").unwrap().label == "StageLines");
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(
        String::from_utf8(fixture.index()).unwrap(),
        Fixture::base().replace("line 3\n", "first change\n")
    );
    assert!(fixture.app.diff.interaction().line_mode);
    assert!(fixture.app.diff.interaction().selected_lines.is_empty());
    press(&mut fixture.app, KeyCode::Tab);
    refresh(&mut fixture.app);
    assert_eq!(fixture.app.diff.interaction().side, Some(DiffSide::Staged));
    press(&mut fixture.app, KeyCode::Char('v'));
    press(&mut fixture.app, KeyCode::Char('j'));
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(String::from_utf8(fixture.index()).unwrap(), Fixture::base());
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
}

#[test]
fn test_line_cursor_stage_and_local_discard_apply_only_one_changed_line() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('a'));
    // The first selectable row is the removal of line 3; stage only that row.
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(
        String::from_utf8(fixture.index()).unwrap(),
        Fixture::base().replace("line 3\n", "")
    );
    assert!(text(&render(&fixture.app, false)).contains("Diff [Line"));
    // The remaining first row is the added replacement, discarded from worktree.
    press(&mut fixture.app, KeyCode::Char('d'));
    refresh(&mut fixture.app);
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed().replace("first change\n", "")
    );
    assert_eq!(
        String::from_utf8(fixture.index()).unwrap(),
        Fixture::base().replace("line 3\n", "")
    );
}

#[test]
fn test_discard_hunk_and_staged_discard_do_not_destroy_index_or_worktree() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('d'));
    refresh(&mut fixture.app);
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::base().replace("line 30\n", "second change\n")
    );
    assert_eq!(String::from_utf8(fixture.index()).unwrap(), Fixture::base());
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Tab);
    refresh(&mut fixture.app);
    let before = std::fs::read(fixture.dir.path().join("file.txt")).unwrap();
    assert_eq!(
        help(&fixture.app, "diff discard").unwrap().label,
        "UnstageSelection"
    );
    press(&mut fixture.app, KeyCode::Char('d'));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        before
    );
}

#[test]
fn test_explicit_line_selection_overrides_cursor_and_escape_returns_to_list() {
    let mut fixture = Fixture::new();
    fixture.focus();
    palette(&mut fixture.app, "diff line mode");
    palette(&mut fixture.app, "hunk select");
    let selected = fixture.app.diff.interaction().selected_lines.clone();
    press(&mut fixture.app, KeyCode::Char('j'));
    let target = fixture.app.hunk_view().line_targets(PatchDirection::Stage);
    assert_eq!(target, selected.iter().copied().collect::<Vec<_>>());
    let retained = AppStateSnapshotOwned::from_app(&fixture.app.state);
    assert!(fixture
        .app
        .shortcut_hints()
        .contains(&("Esc".into(), "BackToList".into())));
    press(&mut fixture.app, KeyCode::Esc);
    assert!(!fixture.app.diff.interaction().has_selection());
    assert_eq!(retained.diff_interaction.selected_lines, selected);
    assert!(!fixture.app.diff.interaction().focused);
}

#[test]
fn test_line_mode_custom_bindings_and_palette_use_same_actions() {
    let mut keymap = Keymap::default();
    keymap
        .global
        .bindings
        .insert("diff_line_mode".into(), vec!["m".into()]);
    keymap
        .global
        .bindings
        .insert("diff_discard".into(), vec!["x".into()]);
    keymap.rebuild_indexes();
    let mut fixture = Fixture::with_keymap(keymap);
    fixture.focus();
    assert!(map_test_key(
        &fixture.app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)
    )
    .is_none());
    press(&mut fixture.app, KeyCode::Char('m'));
    let expected = fixture
        .app
        .resolve_command_palette_command("diff discard")
        .unwrap();
    assert_eq!(
        format!("{expected:?}"),
        format!(
            "{:?}",
            map_test_key(
                &fixture.app,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)
            )
            .unwrap()
        )
    );
    assert_eq!(help(&fixture.app, "diff discard").unwrap().keys, ["x"]);
    assert_eq!(help(&fixture.app, "diff line mode").unwrap().keys, ["m"]);
}

#[test]
fn test_line_write_failure_and_panel_switch_preserve_flux_completion_identity() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('a'));
    fixture
        .app
        .apply_repository_action(&DomainAction::ApplySelectedHunks(PatchDirection::Discard));
    let request = fixture
        .app
        .take_effects()
        .into_iter()
        .find(|effect| matches!(effect, EffectRequest::ApplyLines { .. }))
        .unwrap();
    assert!(help(&fixture.app, "hunk stage").is_none());
    assert!(help(&fixture.app, "diff discard").is_none());
    std::fs::write(
        fixture.dir.path().join("file.txt"),
        "new external content\n",
    )
    .unwrap();
    press(&mut fixture.app, KeyCode::Char('2'));
    let events = effects::execute(request, &*fixture.app.runtime.repo);
    assert!(matches!(
        &events[0],
        crate::flux::action::Action::Domain(DomainAction::ApplyHunksFinished {
            result: Err(_),
            ..
        })
    ));
    crate::flux::test_runtime::drive_actions(&mut fixture.app, events);
    refresh(&mut fixture.app);
    assert_eq!(fixture.app.ui.active_panel, SidePanel::LocalBranches);
    assert!(!fixture.app.diff.interaction().focused);
    assert!(fixture.app.diff.interaction().pending_write.is_none());
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        b"new external content\n"
    );
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
}

#[test]
fn test_line_navigation_includes_context_but_never_eof_markers() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('a'));
    let mut context_seen = false;
    for _ in 0..30 {
        let id: LineId = fixture.app.diff.interaction().line_cursor.unwrap();
        let patch = fixture.app.diff.document().patch.as_ref().unwrap();
        let kind = patch.files[id.file].hunks[id.hunk].lines[id.line].kind;
        assert_ne!(kind, PatchLineKind::NoNewline);
        if kind == PatchLineKind::Context {
            context_seen = true;
            assert!(!fixture.app.hunk_view().can_apply(PatchDirection::Stage));
        }
        press(&mut fixture.app, KeyCode::Char('j'));
    }
    assert!(context_seen);
    assert_eq!(fixture.app.diff.interaction().line_cursor.unwrap().hunk, 1);
    press(&mut fixture.app, KeyCode::Char('a'));
    assert!(!fixture.app.diff.interaction().line_mode);
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
}
