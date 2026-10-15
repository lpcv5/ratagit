use super::*;
use crate::config::keymap::Keymap;
use crate::flux::action::DomainAction;
use crate::flux::action::RuntimeEvent;
use crate::flux::effects::{self, EffectRequest};
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::git::patch::{HunkId, LineId, PatchDirection, PatchRequest, PatchSource};
use crate::git::{Git2Repository, PatchRepository};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;
use tempfile::TempDir;
mod blocks;
mod lazygit;
mod lines;
mod viewport;

#[test]
fn test_diff_behavior_binding_resolves_actions_without_executing_them() {
    use crate::ui::components::diff::{DiffBehaviorBindings, DiffIntent};
    let mut fixture = Fixture::new();
    fixture.focus();
    let before = fixture.app.diff.interaction().cursor;
    let action = AppStateSnapshotOwned::from_app(&fixture.app)
        .as_snapshot()
        .command_env()
        .resolve_diff_intent(DiffIntent::NextHunk)
        .unwrap();
    assert_eq!(fixture.app.diff.interaction().cursor, before);
    dispatch_test_action(&mut fixture.app, action);
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
    let index = fixture.index();
    let action = AppStateSnapshotOwned::from_app(&fixture.app)
        .as_snapshot()
        .command_env()
        .resolve_diff_intent(DiffIntent::StageSelection)
        .unwrap();
    assert_eq!(fixture.index(), index);
    dispatch_test_action(&mut fixture.app, action);
    refresh(&mut fixture.app);
    assert_eq!(
        String::from_utf8(fixture.index()).unwrap(),
        Fixture::base().replace("line 30\n", "second change\n")
    );
    fixture.app.schedule_diff_reload();
    let snapshot = AppStateSnapshotOwned::from_app(&fixture.app);
    let view = snapshot.as_snapshot();
    assert!(view
        .command_env()
        .resolve_diff_intent(DiffIntent::StageSelection)
        .is_none());
    assert!(view
        .command_env()
        .resolve_diff_intent(DiffIntent::NextHunk)
        .is_none());
}

struct Fixture {
    dir: TempDir,
    app: App,
}

impl Fixture {
    fn new() -> Self {
        Self::with_keymap(Keymap::default())
    }
    fn with_keymap(keymap: Keymap) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        config.set_bool("core.autocrlf", false).unwrap();
        std::fs::write(dir.path().join("file.txt"), Self::base()).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let oid = index.write_tree().unwrap();
        let tree = repo.find_tree(oid).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), Self::changed()).unwrap();
        let app =
            App::from_repo_with_keymap(Box::new(Git2Repository::open(dir.path()).unwrap()), keymap)
                .unwrap();
        Self { dir, app }
    }
    fn base() -> String {
        (1..=40).map(|n| format!("line {n}\n")).collect()
    }
    fn changed() -> String {
        Self::base()
            .replace("line 3\n", "first change\n")
            .replace("line 30\n", "second change\n")
    }
    fn index(&self) -> Vec<u8> {
        let repo = git2::Repository::open(self.dir.path()).unwrap();
        let index = repo.index().unwrap();
        let entry = index.get_path(Path::new("file.txt"), 0).unwrap();
        let blob = repo.find_blob(entry.id).unwrap();
        blob.content().to_vec()
    }
    fn focus(&mut self) {
        let directory = self
            .app
            .file_selection()
            .selected_tree_node()
            .is_some_and(|node| node.is_dir);
        press(
            &mut self.app,
            if directory {
                KeyCode::Char('0')
            } else {
                KeyCode::Enter
            },
        );
        if self.app.diff.interaction().line_mode {
            press(&mut self.app, KeyCode::Char('a'));
        }
        assert!(self.app.diff.interaction().focused);
    }
}

fn press(app: &mut App, code: KeyCode) {
    dispatch_test_key(app, KeyEvent::new(code, KeyModifiers::NONE));
}
fn palette(app: &mut App, text: &str) {
    dispatch_test_action(app, DomainAction::StartCommandPalette);
    app.input.buffer = text.into();
    dispatch_test_action(app, DomainAction::InputEnter);
}
fn refresh(app: &mut App) {
    for _ in 0..3 {
        dispatch_test_action(
            app,
            DomainAction::Runtime(RuntimeEvent::ProcessBackgroundLoads),
        );
    }
    dispatch_test_action(
        app,
        DomainAction::Runtime(RuntimeEvent::FlushPendingDiffReload),
    );
    assert!(!app.diff.is_loading());
}
fn help(app: &App, name: &str) -> Option<super::commands::CommandHelp> {
    app.available_commands()
        .into_iter()
        .find(|command| command.name == name)
}
fn render(app: &App, palette: bool) -> ratatui::buffer::Buffer {
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(150, 40)).unwrap();
    terminal
        .draw(|frame| {
            if palette {
                crate::ui::panels::render_command_palette(frame, &snapshot.as_snapshot());
            } else {
                crate::ui::layout::render_layout(
                    frame,
                    &snapshot.as_snapshot(),
                    &mut Default::default(),
                );
            }
        })
        .unwrap();
    terminal.backend().buffer().clone()
}
fn text(buffer: &ratatui::buffer::Buffer) -> String {
    buffer.content.iter().map(|cell| cell.symbol()).collect()
}

#[test]
fn test_hunk_focus_navigation_keeps_file_selection_and_scrolls_to_change() {
    let mut fixture = Fixture::new();
    let file = fixture.app.files.view().panel.selected();
    press(&mut fixture.app, KeyCode::Enter);
    assert!(!fixture.app.diff.interaction().line_mode);
    assert_eq!(
        help(&fixture.app, "diff next line").unwrap().label,
        "NextHunk"
    );
    press(&mut fixture.app, KeyCode::Char('j'));
    assert_eq!(
        fixture.app.diff.interaction().cursor.map(LineId::hunk),
        Some(HunkId { file: 0, hunk: 1 })
    );
    assert_eq!(fixture.app.files.view().panel.selected(), file);
    assert!(matches!(
        fixture.app.diff.document().lines[fixture.app.diff.scroll()].kind,
        crate::git::DiffLineKind::Added | crate::git::DiffLineKind::Removed
    ));
    press(&mut fixture.app, KeyCode::Down);
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
    press(&mut fixture.app, KeyCode::Char('h'));
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 0);
    press(&mut fixture.app, KeyCode::Up);
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 0);
    press(&mut fixture.app, KeyCode::Char('l'));
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
    press(&mut fixture.app, KeyCode::Char('k'));
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 0);
}

#[test]
fn test_auto_refresh_keeps_diff_position_during_repeated_down_navigation() {
    for line_mode in [false, true] {
        let mut fixture = Fixture::new();
        fixture.focus();
        if line_mode {
            press(&mut fixture.app, KeyCode::Char('a'));
        }
        for _ in 0..4 {
            press(&mut fixture.app, KeyCode::Char('j'));
        }
        for _ in 0..3 {
            let cursor = fixture.app.diff.interaction().cursor;
            let line_cursor = fixture.app.diff.interaction().line_cursor;
            let scroll = fixture.app.diff.scroll();
            crate::flux::test_runtime::drive_actions(
                &mut fixture.app,
                [crate::flux::action::Action::System(
                    crate::flux::action::SystemAction::AutoRefresh,
                )],
            );
            refresh(&mut fixture.app);
            assert_eq!(
                fixture.app.diff.interaction().cursor,
                cursor,
                "line_mode={line_mode}"
            );
            assert_eq!(
                fixture.app.diff.interaction().line_cursor,
                line_cursor,
                "line_mode={line_mode}"
            );
            assert_eq!(fixture.app.diff.scroll(), scroll, "line_mode={line_mode}");
            press(&mut fixture.app, KeyCode::Char('j'));
        }
    }
}

#[test]
fn test_multi_hunk_selection_escape_returns_to_list_and_clears() {
    let mut fixture = Fixture::new();
    fixture.focus();
    palette(&mut fixture.app, "hunk select");
    press(&mut fixture.app, KeyCode::Char('j'));
    palette(&mut fixture.app, "hunk select");
    assert_eq!(fixture.app.diff.interaction().selected.len(), 2);
    assert!(fixture
        .app
        .shortcut_hints()
        .iter()
        .any(|(key, label)| key == "Esc" && label == "BackToList"));
    press(&mut fixture.app, KeyCode::Esc);
    assert!(fixture.app.diff.interaction().selected.is_empty());
    assert!(!fixture.app.diff.interaction().focused);
}

#[test]
fn test_keyboard_stages_current_hunk_and_keeps_both_sources_in_one_file() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    let index = String::from_utf8(fixture.index()).unwrap();
    assert!(index.contains("first change"));
    assert!(!index.contains("second change"));
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
    assert_eq!(fixture.app.files.status().staged.len(), 1);
    assert_eq!(fixture.app.files.status().unstaged.len(), 1);
    let patch = fixture.app.diff.document().patch.as_ref().unwrap();
    assert_eq!(patch.display_file_count(), 1);
    assert_eq!(patch.files.len(), 2);
    assert!(patch
        .files
        .iter()
        .any(|file| file.source == PatchSource::Index && file.hunks.len() == 1));
    assert!(patch
        .files
        .iter()
        .any(|file| file.source == PatchSource::Worktree && file.hunks.len() == 1));
    assert_eq!(
        fixture
            .app
            .files
            .view()
            .tree_nodes
            .iter()
            .filter(|node| node.path == Path::new("file.txt"))
            .count(),
        1
    );
    assert_eq!(
        fixture
            .app
            .diff
            .document()
            .lines
            .iter()
            .filter(|line| line.content.starts_with("--- "))
            .count(),
        1
    );
    let buffer = render(&fixture.app, false);
    assert!(!text(&buffer).contains("Staged"));
    assert!(!text(&buffer).contains("Unstaged"));
    assert!(buffer
        .content
        .iter()
        .any(|cell| cell.bg == crate::ui::theme::UiTheme::default().diff_staged_added_cursor_bg));
    assert!(fixture.app.diff.interaction().focused);
    assert!(fixture.app.diff.interaction().selected.is_empty());
    assert!(help(&fixture.app, "hunk unstage").is_some());
}

#[test]
fn test_combined_view_keeps_hunk_order_and_space_unstages_the_staged_cursor() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('j'));
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    let patch = fixture.app.diff.document().patch.as_ref().unwrap();
    let ids = patch.hunk_ids();
    assert_eq!(patch.display_file_count(), 1);
    assert_eq!(ids.len(), 2);
    assert_eq!(patch.files[ids[0].file].source, PatchSource::Worktree);
    assert_eq!(patch.files[ids[1].file].source, PatchSource::Index);
    assert_eq!(
        fixture.app.diff.interaction().cursor.map(LineId::hunk),
        Some(ids[1])
    );
    assert_eq!(fixture.app.diff.interaction().side, Some(DiffSide::Staged));
    assert_eq!(help(&fixture.app, "diff stage").unwrap().label, "Unstage");
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
    assert_eq!(fixture.app.files.view().tree_nodes.len(), 1);
    assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(
        String::from_utf8(fixture.index()).unwrap(),
        Fixture::base().replace("line 30\n", "second change\n")
    );
}

#[test]
fn test_space_keeps_the_second_hunk_when_another_hunk_is_already_staged() {
    let mut fixture = Fixture::new();
    press(&mut fixture.app, KeyCode::Enter);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Char('j'));
    for staged in [true, false, true, false] {
        press(&mut fixture.app, KeyCode::Char(' '));
        refresh(&mut fixture.app);
        let patch = fixture.app.diff.document().patch.as_ref().unwrap();
        let cursor = fixture.app.diff.interaction().cursor.unwrap();
        assert_eq!(patch.change_ids()[1], cursor);
        assert_eq!(
            fixture.app.diff.interaction().side,
            Some(if staged {
                DiffSide::Staged
            } else {
                DiffSide::Unstaged
            })
        );
        let expected = Fixture::base().replace("line 3\n", "first change\n");
        let expected = if staged {
            expected.replace("line 30\n", "second change\n")
        } else {
            expected
        };
        assert_eq!(String::from_utf8(fixture.index()).unwrap(), expected);
    }
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
}

#[test]
fn test_space_distinguishes_identical_hunks_after_unstaged_insertions_shift_lines() {
    let mut fixture = Fixture::new();
    let path = fixture.dir.path().join("file.txt");
    let base: String = (1..=100)
        .map(|n| {
            if n == 30 || n == 60 {
                "marker\n".into()
            } else {
                format!("line {n}\n")
            }
        })
        .collect();
    let repo = Git2Repository::open(fixture.dir.path()).unwrap();
    std::fs::write(&path, &base).unwrap();
    crate::git::WorktreeRepository::stage(&repo, Path::new("file.txt")).unwrap();
    crate::git::CommitRepository::commit(&repo, "identical hunk fixture").unwrap();
    let mut staged = base;
    let last_marker = staged.rfind("marker\n").unwrap();
    staged.replace_range(
        last_marker..last_marker + "marker\n".len(),
        "changed marker\n",
    );
    std::fs::write(&path, &staged).unwrap();
    crate::git::WorktreeRepository::stage(&repo, Path::new("file.txt")).unwrap();
    let insertion: String = (0..50).map(|n| format!("inserted {n}\n")).collect();
    let changed = staged
        .replacen("marker\n", "changed marker\n", 1)
        .replace("line 3\n", &insertion);
    std::fs::write(&path, &changed).unwrap();
    load_file(&mut fixture, "file.txt");
    press(&mut fixture.app, KeyCode::Enter);
    press(&mut fixture.app, KeyCode::Char('j'));
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    let patch = fixture.app.diff.document().patch.as_ref().unwrap();
    assert_eq!(patch.hunk_ids().len(), 3);
    assert_eq!(
        fixture.app.diff.interaction().cursor,
        Some(patch.change_ids()[1])
    );
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(String::from_utf8(fixture.index()).unwrap(), staged);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), changed);
}

#[test]
fn test_switch_side_and_unstage_current_hunk_preserve_worktree() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Tab);
    assert_eq!(
        fixture.app.diff.interaction().side,
        Some(DiffSide::Unstaged)
    );
    press(&mut fixture.app, KeyCode::Tab);
    assert_eq!(
        fixture.app.diff.document().patch.as_ref().unwrap().files[0].source,
        PatchSource::Index
    );
    assert!(help(&fixture.app, "hunk stage").is_none());
    assert!(help(&fixture.app, "hunk unstage").is_some());
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert!(!fixture.app.diff.document().lines.is_empty());
    assert!(fixture
        .app
        .diff
        .document()
        .patch
        .as_ref()
        .unwrap()
        .files
        .iter()
        .all(|file| file.source == PatchSource::Worktree));
    press(&mut fixture.app, KeyCode::Tab);
    assert_eq!(
        fixture.app.diff.document().patch.as_ref().unwrap().files[0]
            .hunks
            .len(),
        2
    );
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
}

#[test]
fn test_selected_hunks_override_cursor_and_can_stage_all() {
    let mut fixture = Fixture::new();
    fixture.focus();
    palette(&mut fixture.app, "hunk select");
    press(&mut fixture.app, KeyCode::Char('j'));
    palette(&mut fixture.app, "hunk stage");
    refresh(&mut fixture.app);
    let index = String::from_utf8(fixture.index()).unwrap();
    assert!(index.contains("first change"));
    assert!(!index.contains("second change"));
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::changed().as_bytes());
    assert!(!fixture.app.diff.document().lines.is_empty());
    assert!(fixture
        .app
        .diff
        .document()
        .patch
        .as_ref()
        .unwrap()
        .files
        .iter()
        .all(|file| file.source == PatchSource::Index));
}

#[test]
fn test_palette_can_select_stage_and_unstage_multiple_hunks() {
    let mut fixture = Fixture::new();
    palette(&mut fixture.app, "diff focus");
    palette(&mut fixture.app, "hunk select");
    palette(&mut fixture.app, "hunk next");
    palette(&mut fixture.app, "hunk select");
    palette(&mut fixture.app, "hunk stage");
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::changed().as_bytes());
    palette(&mut fixture.app, "diff side");
    palette(&mut fixture.app, "hunk previous");
    palette(&mut fixture.app, "hunk select");
    palette(&mut fixture.app, "hunk next");
    palette(&mut fixture.app, "hunk select");
    palette(&mut fixture.app, "hunk unstage");
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
}

#[test]
fn test_custom_bindings_palette_and_footer_use_same_hunk_definitions() {
    let mut map = Keymap::default();
    map.global
        .bindings
        .insert("focus_diff".into(), vec!["f".into()]);
    map.global
        .bindings
        .insert("hunk_stage".into(), vec!["x".into()]);
    map.rebuild_indexes();
    let mut fixture = Fixture::with_keymap(map);
    assert!(!matches!(
        map_test_key(
            &fixture.app,
            KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
        ),
        Some(DomainAction::ToggleDiffFocus)
    ));
    press(&mut fixture.app, KeyCode::Char('f'));
    assert_eq!(help(&fixture.app, "hunk stage").unwrap().keys, vec!["x"]);
    assert!(fixture
        .app
        .shortcut_hints()
        .iter()
        .any(|(key, label)| key == "x" && label == "StageHunks"));
    fixture.app.start_command_palette();
    fixture.app.input.buffer = "hunk stage".into();
    let rendered = text(&render(&fixture.app, true));
    assert!(rendered.contains("x  hunk stage  (StageHunks)"));
    press(&mut fixture.app, KeyCode::Esc);
    press(&mut fixture.app, KeyCode::Char('x'));
    refresh(&mut fixture.app);
    assert!(String::from_utf8(fixture.index())
        .unwrap()
        .contains("first change"));
}

#[test]
fn test_shadowed_and_unbound_hunk_keys_are_not_advertised() {
    for keys in [vec!["l".into()], vec![]] {
        let mut map = Keymap::default();
        map.global.bindings.insert("hunk_stage".into(), keys);
        map.rebuild_indexes();
        let mut fixture = Fixture::with_keymap(map);
        fixture.focus();
        assert!(help(&fixture.app, "hunk stage").unwrap().keys.is_empty());
        assert!(!fixture
            .app
            .shortcut_hints()
            .iter()
            .any(|(_, label)| label == "StageHunks"));
        palette(&mut fixture.app, "hunk stage");
        refresh(&mut fixture.app);
        assert!(String::from_utf8(fixture.index())
            .unwrap()
            .contains("first change"));
    }
}

#[test]
fn test_hunk_focus_disables_file_writes_and_preserves_modal_text_input() {
    let mut fixture = Fixture::new();
    fixture.focus();
    assert!(fixture
        .app
        .resolve_command_palette_command("stage")
        .is_none());
    assert!(fixture
        .app
        .resolve_command_palette_command("discard")
        .is_none());
    fixture.app.start_command_palette();
    press(&mut fixture.app, KeyCode::Char('s'));
    assert_eq!(fixture.app.input.buffer, "s");
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    press(&mut fixture.app, KeyCode::Esc);
    assert!(fixture.app.diff.interaction().focused);
}

#[test]
fn test_pending_write_blocks_duplicate_hunk_commands_until_completion() {
    let mut fixture = Fixture::new();
    fixture.focus();
    fixture
        .app
        .apply_repository_action(&DomainAction::ApplySelectedHunks(PatchDirection::Stage));
    let request = fixture
        .app
        .take_effects()
        .into_iter()
        .find(|e| matches!(e, EffectRequest::ApplyHunks { .. }))
        .unwrap();
    let pending = fixture.app.diff.interaction().pending_write;
    assert!(help(&fixture.app, "hunk stage").is_none());
    fixture
        .app
        .apply_repository_action(&DomainAction::ApplySelectedHunks(PatchDirection::Stage));
    assert!(!fixture
        .app
        .take_effects()
        .iter()
        .any(|e| matches!(e, EffectRequest::ApplyHunks { .. })));
    fixture
        .app
        .apply_repository_action(&DomainAction::ApplyHunksFinished {
            id: pending.unwrap() + 1,
            direction: PatchDirection::Stage,
            result: Ok(()),
        });
    assert_eq!(fixture.app.diff.interaction().pending_write, pending);
    let events = effects::execute(request, &*fixture.app.runtime.repo);
    crate::flux::test_runtime::drive_actions(&mut fixture.app, events);
    refresh(&mut fixture.app);
    assert!(fixture.app.diff.interaction().pending_write.is_none());
    assert!(help(&fixture.app, "hunk unstage").is_some());
}

#[test]
fn test_stale_hunk_write_logs_failure_and_reloads_patch() {
    let mut fixture = Fixture::new();
    fixture.focus();
    std::fs::write(fixture.dir.path().join("file.txt"), "external change\n").unwrap();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert!(fixture.app.command_log.iter().any(|entry| !entry.success
        && entry.command.contains("patch changed")
        && entry
            .command
            .contains("Reload the diff and select hunks again")));
    assert!(fixture
        .app
        .diff
        .document()
        .lines
        .iter()
        .any(|line| line.content == "external change"));
    assert!(fixture.app.diff.interaction().pending_write.is_none());
}

#[test]
fn test_refresh_and_panel_switch_reset_hunk_selection() {
    let mut fixture = Fixture::new();
    fixture.focus();
    palette(&mut fixture.app, "hunk select");
    fixture.app.request_refresh(RefreshKind::StatusOnly);
    assert!(fixture.app.diff.interaction().selected.is_empty());
    assert!(help(&fixture.app, "hunk stage").is_none());
    refresh(&mut fixture.app);
    assert!(fixture.app.diff.interaction().focused);
    press(&mut fixture.app, KeyCode::Char('2'));
    assert!(!fixture.app.diff.interaction().focused);
    assert!(fixture.app.diff.interaction().side.is_none());
    assert!(fixture.app.diff.interaction().selected.is_empty());
}

#[test]
fn test_render_shows_hunk_cursor_selection_without_badges_and_snapshot_parity() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char('j'));
    palette(&mut fixture.app, "hunk select");
    let snapshot = AppStateSnapshotOwned::from_app(&fixture.app.state);
    assert!(snapshot.context.diff_focused);
    assert_eq!(
        snapshot.diff_interaction.cursor,
        fixture.app.diff.interaction().cursor
    );
    assert_eq!(
        snapshot
            .as_snapshot()
            .command_env()
            .diff
            .targets(PatchDirection::Stage),
        fixture.app.hunk_view().targets(PatchDirection::Stage)
    );
    let buffer = render(&fixture.app, false);
    let rendered = text(&buffer);
    assert!(rendered.contains("Hunk 2/2 | 1 selected"));
    assert!(buffer.content.iter().any(|cell| cell.symbol() == ">"));
    assert!(!rendered.contains(" Unstaged "));
    assert!(!rendered.contains(" Staged "));
    assert!(buffer
        .content
        .iter()
        .any(|cell| cell.bg == crate::ui::theme::UiTheme::default().diff_added_cursor_bg));
}

#[test]
fn test_historical_and_truncated_patches_allow_navigation_without_writes() {
    for historical in [true, false] {
        let mut fixture = Fixture::new();
        let mut patch = (*fixture.app.diff.document().patch.clone().unwrap()).clone();
        if historical {
            patch.files[0].source = PatchSource::Commit("history".into());
        } else {
            patch.truncated = true;
        }
        fixture
            .app
            .diff
            .set_test_document(crate::git::patch::DiffDocument {
                lines: patch.display_lines().into(),
                patch: Some(std::sync::Arc::new(patch)),
            });
        fixture.focus();
        press(&mut fixture.app, KeyCode::Char('j'));
        assert_eq!(fixture.app.diff.interaction().cursor.unwrap().hunk, 1);
        assert!(help(&fixture.app, "hunk stage").is_none());
        assert!(help(&fixture.app, "hunk unstage").is_none());
        assert!(text(&render(&fixture.app, false)).contains("Read only"));
    }
}

#[test]
fn test_unstage_preserves_crlf_non_utf8_and_missing_final_newline() {
    let fixture = Fixture::new();
    let repo = Git2Repository::open(fixture.dir.path()).unwrap();
    let content = b"first\xff\r\nlast";
    std::fs::write(fixture.dir.path().join("file.txt"), content).unwrap();
    crate::git::WorktreeRepository::stage(&repo, Path::new("file.txt")).unwrap();
    let patch = repo
        .patch(PatchRequest::Index("file.txt".into()))
        .unwrap()
        .unwrap();
    repo.apply_hunks(
        &patch,
        &[HunkId { file: 0, hunk: 0 }],
        PatchDirection::Unstage,
    )
    .unwrap();
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert_eq!(
        std::fs::read(fixture.dir.path().join("file.txt")).unwrap(),
        content
    );
}

#[test]
fn test_unstage_only_one_hunk_leaves_the_other_in_index() {
    let fixture = Fixture::new();
    let repo = Git2Repository::open(fixture.dir.path()).unwrap();
    crate::git::WorktreeRepository::stage(&repo, Path::new("file.txt")).unwrap();
    let patch = repo
        .patch(PatchRequest::Index("file.txt".into()))
        .unwrap()
        .unwrap();
    repo.apply_hunks(
        &patch,
        &[HunkId { file: 0, hunk: 1 }],
        PatchDirection::Unstage,
    )
    .unwrap();
    let index = String::from_utf8(fixture.index()).unwrap();
    assert!(index.contains("first change"));
    assert!(!index.contains("second change"));
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
}

fn load_file(fixture: &mut Fixture, name: &str) {
    fixture.app.request_refresh(RefreshKind::StatusOnly);
    refresh(&mut fixture.app);
    let index = fixture
        .app
        .files
        .view()
        .tree_nodes
        .iter()
        .position(|node| node.path == Path::new(name))
        .unwrap();
    fixture.app.files.test_view_mut().panel.select(Some(index));
    fixture.app.diff.test_interaction_mut().side = None;
    fixture.app.reload_diff_now();
    crate::flux::test_runtime::finish_background(&mut fixture.app);
}

fn directory_fixture() -> Fixture {
    let mut fixture = Fixture::new();
    std::fs::create_dir(fixture.dir.path().join("dir")).unwrap();
    for name in ["dir/a.txt", "dir/b.txt"] {
        std::fs::write(fixture.dir.path().join(name), Fixture::base()).unwrap();
    }
    let repo = Git2Repository::open(fixture.dir.path()).unwrap();
    crate::git::WorktreeRepository::stage_paths(&repo, &["dir/a.txt".into(), "dir/b.txt".into()])
        .unwrap();
    crate::git::CommitRepository::commit(&repo, "directory").unwrap();
    for name in ["dir/a.txt", "dir/b.txt"] {
        std::fs::write(fixture.dir.path().join(name), Fixture::changed()).unwrap();
    }
    load_file(&mut fixture, "dir");
    fixture
}

fn index_file(fixture: &Fixture, name: &str) -> Option<Vec<u8>> {
    let repo = git2::Repository::open(fixture.dir.path()).unwrap();
    let index = repo.index().unwrap();
    index
        .get_path(Path::new(name), 0)
        .map(|entry| repo.find_blob(entry.id).unwrap().content().to_vec())
}

#[test]
fn test_directory_selection_stages_and_unstages_hunks_across_files() {
    let mut fixture = directory_fixture();
    fixture.focus();
    palette(&mut fixture.app, "hunk select");
    for _ in 0..3 {
        press(&mut fixture.app, KeyCode::Char('j'));
    }
    palette(&mut fixture.app, "hunk select");
    assert_eq!(fixture.app.diff.interaction().selected.len(), 2);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    let first = String::from_utf8(index_file(&fixture, "dir/a.txt").unwrap()).unwrap();
    let second = String::from_utf8(index_file(&fixture, "dir/b.txt").unwrap()).unwrap();
    assert!(first.contains("first change"));
    assert!(!first.contains("second change"));
    assert!(!second.contains("first change"));
    assert!(second.contains("second change"));
    palette(&mut fixture.app, "hunk select");
    press(&mut fixture.app, KeyCode::Char('k'));
    press(&mut fixture.app, KeyCode::Char('k'));
    press(&mut fixture.app, KeyCode::Char('k'));
    palette(&mut fixture.app, "hunk select");
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    for name in ["dir/a.txt", "dir/b.txt"] {
        assert_eq!(
            index_file(&fixture, name).unwrap(),
            Fixture::base().as_bytes()
        );
        assert_eq!(
            std::fs::read_to_string(fixture.dir.path().join(name)).unwrap(),
            Fixture::changed()
        );
    }
}

#[test]
fn test_switching_selection_direction_clears_previous_directory_hunks() {
    let mut fixture = directory_fixture();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    palette(&mut fixture.app, "hunk select");
    assert!(help(&fixture.app, "hunk unstage").is_some());
    press(&mut fixture.app, KeyCode::Char('j'));
    palette(&mut fixture.app, "hunk select");
    assert_eq!(fixture.app.diff.interaction().selected.len(), 1);
    assert!(help(&fixture.app, "hunk stage").is_some());
    assert!(help(&fixture.app, "hunk unstage").is_none());
}

#[test]
fn test_stale_multi_file_selection_leaves_entire_index_unchanged() {
    let fixture = directory_fixture();
    let repo = Git2Repository::open(fixture.dir.path()).unwrap();
    let patch = repo
        .patch(PatchRequest::Directory("dir".into()))
        .unwrap()
        .unwrap();
    std::fs::write(fixture.dir.path().join("dir/b.txt"), "external\n").unwrap();
    assert!(repo
        .apply_hunks(
            &patch,
            &[HunkId { file: 0, hunk: 0 }, HunkId { file: 1, hunk: 0 }],
            PatchDirection::Stage
        )
        .is_err());
    for name in ["dir/a.txt", "dir/b.txt"] {
        assert_eq!(
            index_file(&fixture, name).unwrap(),
            Fixture::base().as_bytes()
        );
    }
}

#[test]
fn test_new_text_file_hunk_roundtrip_with_literal_filename() {
    let mut fixture = Fixture::new();
    let name = "new [a].txt";
    std::fs::write(fixture.dir.path().join(name), "new content\n").unwrap();
    load_file(&mut fixture, name);
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(index_file(&fixture, name).unwrap(), b"new content\n");
    press(&mut fixture.app, KeyCode::Tab);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert!(index_file(&fixture, name).is_none());
    assert_eq!(
        fixture.app.selected_tree_node().unwrap().path,
        Path::new(name)
    );
    assert_eq!(
        std::fs::read(fixture.dir.path().join(name)).unwrap(),
        b"new content\n"
    );
    assert!(help(&fixture.app, "hunk stage").is_some());
}

#[test]
fn test_deleted_text_file_hunk_roundtrip_preserves_missing_worktree_file() {
    let mut fixture = Fixture::new();
    std::fs::remove_file(fixture.dir.path().join("file.txt")).unwrap();
    load_file(&mut fixture, "file.txt");
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert!(index_file(&fixture, "file.txt").is_none());
    press(&mut fixture.app, KeyCode::Tab);
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::base().as_bytes());
    assert!(!fixture.dir.path().join("file.txt").exists());
}

#[test]
fn test_stale_index_unstage_is_rejected_and_reloaded() {
    let mut fixture = Fixture::new();
    fixture.focus();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    press(&mut fixture.app, KeyCode::Tab);
    let external = Git2Repository::open(fixture.dir.path()).unwrap();
    crate::git::WorktreeRepository::stage(&external, Path::new("file.txt")).unwrap();
    press(&mut fixture.app, KeyCode::Char(' '));
    refresh(&mut fixture.app);
    assert_eq!(fixture.index(), Fixture::changed().as_bytes());
    assert!(fixture.app.command_log.iter().any(|entry| !entry.success
        && entry.command.contains("patch changed")
        && entry
            .command
            .contains("Reload the diff and select hunks again")));
    assert_eq!(
        fixture.app.diff.document().patch.as_ref().unwrap().files[0]
            .hunks
            .len(),
        2
    );
}

#[test]
fn test_cached_diff_reentry_keeps_valid_patch_and_rejects_loading_commands() {
    let mut fixture = Fixture::new();
    fixture.focus();
    palette(&mut fixture.app, "hunk select");
    fixture.app.schedule_diff_reload();
    assert!(fixture.app.diff.interaction().selected.is_empty());
    assert!(help(&fixture.app, "hunk stage").is_none());
    fixture.app.flush_pending_diff_reload();
    assert!(fixture.app.diff.document().patch.is_some());
    assert!(help(&fixture.app, "hunk stage").is_some());
    press(&mut fixture.app, KeyCode::Esc);
    press(&mut fixture.app, KeyCode::Enter);
    assert!(fixture.app.diff.interaction().focused);
    assert_eq!(
        fixture.app.diff.document().patch.as_ref().unwrap().files[0]
            .hunks
            .len(),
        2
    );
}

#[test]
fn test_binary_diff_has_no_hunk_focus_or_write_commands() {
    let mut fixture = Fixture::new();
    std::fs::write(fixture.dir.path().join("file.txt"), b"binary\0data").unwrap();
    load_file(&mut fixture, "file.txt");
    assert!(help(&fixture.app, "diff focus").is_none());
    assert!(help(&fixture.app, "hunk stage").is_none());
    assert!(help(&fixture.app, "hunk unstage").is_none());
    assert!(text(&render(&fixture.app, false)).contains("<binary file>"));
}

#[test]
fn test_completion_after_panel_switch_preserves_current_focus_and_refreshes_status() {
    let mut fixture = Fixture::new();
    fixture.focus();
    fixture
        .app
        .apply_repository_action(&DomainAction::ApplySelectedHunks(PatchDirection::Stage));
    let request = fixture
        .app
        .take_effects()
        .into_iter()
        .find(|e| matches!(e, EffectRequest::ApplyHunks { .. }))
        .unwrap();
    press(&mut fixture.app, KeyCode::Char('2'));
    let events = effects::execute(request, &*fixture.app.runtime.repo);
    crate::flux::test_runtime::drive_actions(&mut fixture.app, events);
    refresh(&mut fixture.app);
    assert_eq!(fixture.app.ui.active_panel, SidePanel::LocalBranches);
    assert!(!fixture.app.diff.interaction().focused);
    assert_eq!(fixture.app.files.status().staged.len(), 1);
    assert!(fixture.app.diff.interaction().pending_write.is_none());
}
