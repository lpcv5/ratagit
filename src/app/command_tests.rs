use super::commands::{self, CommandId};
use super::{App, CommitFieldFocus, InputMode, SidePanel, ViewContext};
use crate::config::keymap::Keymap;
use crate::file_tree::{FileTreeNode, FileTreeNodeStatus};
use crate::flux::action::DomainAction;
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::flux::stores::test_support::MockRepo;
use crate::git::{BranchInfo, FileEntry, FileStatus, StashInfo};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn node(path: &str, status: FileTreeNodeStatus, depth: usize) -> FileTreeNode {
    let is_dir = status == FileTreeNodeStatus::Directory;
    FileTreeNode {
        path: path.into(),
        status,
        depth,
        is_dir,
        is_expanded: is_dir,
    }
}

fn fixture_with_keymap(mut keymap: Keymap) -> App {
    keymap.rebuild_indexes();
    let mut app = App::from_repo_with_keymap(Box::new(MockRepo), keymap).unwrap();
    app.ui.files.tree_nodes = vec![
        node("src", FileTreeNodeStatus::Directory, 0),
        node("src/a", FileTreeNodeStatus::Staged(FileStatus::Modified), 1),
        node(
            "src/b",
            FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            1,
        ),
        node("loose", FileTreeNodeStatus::Untracked, 0),
    ];
    app.git.status.staged = vec![FileEntry {
        path: "src/a".into(),
        status: FileStatus::Modified,
    }];
    app.git.status.unstaged = vec![FileEntry {
        path: "src/b".into(),
        status: FileStatus::Modified,
    }];
    app.ui.branches.items = vec![
        BranchInfo {
            name: "main".into(),
            is_current: true,
        },
        BranchInfo {
            name: "feature".into(),
            is_current: false,
        },
    ];
    let mut second = app.ui.commits.items[0].clone();
    second.oid = "def1234567890".into();
    app.ui.commits.items.push(second);
    app.ui.stash.items = vec![
        StashInfo {
            index: 0,
            message: "first".into(),
        },
        StashInfo {
            index: 1,
            message: "second".into(),
        },
    ];
    app
}

fn fixture() -> App {
    fixture_with_keymap(Keymap::default())
}

fn key(text: &str) -> KeyEvent {
    let (text, modifiers) = text
        .strip_prefix("C-")
        .map(|text| (text, KeyModifiers::CONTROL))
        .unwrap_or((text, KeyModifiers::NONE));
    let code = match text {
        "Space" => KeyCode::Char(' '),
        "Enter" => KeyCode::Enter,
        "Esc" => KeyCode::Esc,
        "Tab" => KeyCode::Tab,
        "Backspace" => KeyCode::Backspace,
        "Up" => KeyCode::Up,
        "Down" => KeyCode::Down,
        "Left" => KeyCode::Left,
        "Right" => KeyCode::Right,
        _ => KeyCode::Char(text.chars().next().unwrap()),
    };
    KeyEvent::new(code, modifiers)
}

fn palette(app: &mut App, command: &str) -> Option<super::Command> {
    super::dispatch_test_action(app, DomainAction::StartCommandPalette);
    for c in command.chars() {
        super::dispatch_test_key(app, key(&c.to_string()));
    }
    super::dispatch_test_key(app, key("Enter"))
}

fn signature(action: Option<DomainAction>) -> String {
    format!("{action:?}")
}

#[test]
fn test_palette_and_key_execute_same_contextual_commands() {
    for (panel, binding, command) in [
        (SidePanel::Files, "c", "commit"),
        (SidePanel::Files, "s", "stash"),
        (SidePanel::Files, "Space", "stage"),
        (SidePanel::Files, "d", "discard"),
        (SidePanel::LocalBranches, "n", "branch new"),
        (SidePanel::LocalBranches, "Space", "checkout"),
        (SidePanel::Stash, "p", "stash pop"),
    ] {
        let mut by_key = fixture();
        let mut by_palette = fixture();
        by_key.ui.active_panel = panel;
        by_palette.ui.active_panel = panel;
        let key_result = super::dispatch_test_key(&mut by_key, key(binding));
        let palette_result = palette(&mut by_palette, command);
        assert_eq!(
            format!("{key_result:?}"),
            format!("{palette_result:?}"),
            "{command}"
        );
        assert_eq!(by_key.input.mode, by_palette.input.mode, "{command}");
        assert_eq!(
            by_key.input.stash_targets, by_palette.input.stash_targets,
            "{command}"
        );
        assert_eq!(
            by_key.input.branch_switch_target, by_palette.input.branch_switch_target,
            "{command}"
        );
    }
}

#[test]
fn test_visual_commit_from_palette_keeps_selection_scope() {
    let mut app = fixture();
    app.ui.files.visual_mode = true;
    app.ui.files.visual_anchor = Some(2);
    app.ui.files.panel.list_state.select(Some(2));
    assert!(matches!(
        super::map_test_key(&app, key("c")),
        Some(DomainAction::PrepareCommitFromSelection)
    ));
    assert!(matches!(
        app.resolve_command_palette_command("commit"),
        Some(DomainAction::PrepareCommitFromSelection)
    ));
    let mut dispatcher = crate::flux::dispatcher::Dispatcher::with_default_stores();
    let envelope = dispatcher.next_envelope(crate::flux::action::Action::Domain(
        app.resolve_command_palette_command("commit").unwrap(),
    ));
    let result = dispatcher.dispatch(&mut app.state, envelope);
    assert!(
        matches!(result.commands.as_slice(), [super::Command::Effect(crate::flux::effects::EffectRequest::PrepareCommitFromVisualSelection { paths })] if paths == &[std::path::PathBuf::from("src/b")])
    );
}

#[test]
fn test_visual_commit_key_and_palette_open_same_editor() {
    let mut by_key = fixture();
    let mut by_palette = fixture();
    for app in [&mut by_key, &mut by_palette] {
        app.ui.files.visual_mode = true;
        app.ui.files.visual_anchor = Some(2);
        app.ui.files.panel.list_state.select(Some(2));
    }
    super::dispatch_test_key(&mut by_key, key("c"));
    palette(&mut by_palette, "commit");
    assert_eq!(by_key.input.mode, Some(InputMode::CommitEditor));
    assert_eq!(by_palette.input.mode, by_key.input.mode);
    assert!(!by_palette.ui.files.visual_mode);
}

#[test]
fn test_custom_keys_are_used_by_input_help_and_footer() {
    let mut keymap = Keymap::default();
    keymap
        .files
        .bindings
        .insert("toggle_stage".into(), vec!["x".into()]);
    keymap
        .global
        .bindings
        .insert("commit".into(), vec!["C-e".into()]);
    let app = fixture_with_keymap(keymap);
    assert!(matches!(
        super::map_test_key(&app, key("x")),
        Some(DomainAction::StageFile(_))
    ));
    assert!(super::map_test_key(&app, key("Space")).is_none());
    assert!(matches!(
        super::map_test_key(&app, key("C-e")),
        Some(DomainAction::StartCommitInput)
    ));
    assert!(app.shortcut_hints().contains(&("x".into(), "Stage".into())));
    assert!(app
        .shortcut_hints()
        .contains(&("C-e".into(), "Commit".into())));
    let available = app.available_commands();
    assert_eq!(
        available
            .iter()
            .find(|command| command.name == "stage")
            .unwrap()
            .keys,
        ["x"]
    );
    assert_eq!(
        available
            .iter()
            .find(|command| command.name == "commit")
            .unwrap()
            .keys,
        ["C-e"]
    );
}

#[test]
fn test_unbound_command_has_no_fake_default_hint() {
    let mut keymap = Keymap::default();
    keymap
        .files
        .bindings
        .insert("toggle_stage".into(), Vec::new());
    let app = fixture_with_keymap(keymap);
    assert!(super::map_test_key(&app, key("Space")).is_none());
    assert!(!app
        .shortcut_hints()
        .iter()
        .any(|(_, label)| label == "Stage"));
    assert!(app
        .available_commands()
        .iter()
        .find(|command| command.name == "stage")
        .unwrap()
        .keys
        .is_empty());
    assert!(matches!(
        app.resolve_command_palette_command("stage"),
        Some(DomainAction::StageFile(_))
    ));
}

#[test]
fn test_key_collision_is_reflected_in_help_and_footer() {
    let mut keymap = Keymap::default();
    keymap
        .files
        .bindings
        .insert("toggle_stage".into(), vec!["q".into(), "x".into()]);
    let app = fixture_with_keymap(keymap);
    assert!(matches!(
        super::map_test_key(&app, key("q")),
        Some(DomainAction::Quit)
    ));
    assert!(matches!(
        super::map_test_key(&app, key("x")),
        Some(DomainAction::StageFile(_))
    ));
    assert_eq!(
        app.available_commands()
            .iter()
            .find(|command| command.name == "stage")
            .unwrap()
            .keys,
        ["x"]
    );
    assert!(app.shortcut_hints().contains(&("x".into(), "Stage".into())));
}

#[test]
fn test_search_binding_shadows_branch_create_hint_until_search_clears() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::LocalBranches;
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("main".into()));
    assert!(matches!(
        super::map_test_key(&app, key("n")),
        Some(DomainAction::SearchNext)
    ));
    assert!(!app.shortcut_hints().iter().any(|(_, label)| label == "New"));
    assert!(app
        .available_commands()
        .iter()
        .find(|command| command.name == "branch new")
        .unwrap()
        .keys
        .is_empty());
    super::dispatch_test_action(&mut app, DomainAction::SearchClear);
    assert!(matches!(
        super::map_test_key(&app, key("n")),
        Some(DomainAction::StartBranchCreateInput)
    ));
    assert!(app.shortcut_hints().contains(&("n".into(), "New".into())));
}

#[test]
fn test_branch_subview_has_its_own_commands_and_return_context() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::LocalBranches;
    super::dispatch_test_key(&mut app, key("Enter"));
    assert_eq!(
        app.view_context(),
        ViewContext::BranchCommits {
            branch: "main".into()
        }
    );
    assert_eq!(app.view_context().parent(), Some(ViewContext::Branches));
    assert!(app
        .shortcut_hints()
        .contains(&("Enter".into(), "Refresh".into())));
    assert!(app.resolve_command_palette_command("checkout").is_none());
    assert!(super::map_test_key(&app, key("Space")).is_none());
    assert!(!app
        .available_commands()
        .iter()
        .any(|command| command.name == "checkout"));
    super::dispatch_test_key(&mut app, key("Esc"));
    assert_eq!(app.view_context(), ViewContext::Branches);
}

#[test]
fn test_escape_clears_search_before_leaving_visual_or_revision_view() {
    let mut app = fixture();
    app.ui.files.visual_mode = true;
    app.ui.files.visual_anchor = Some(0);
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("src".into()));
    assert!(app
        .shortcut_hints()
        .contains(&("Esc".into(), "ClearSearch".into())));
    super::dispatch_test_key(&mut app, key("Esc"));
    assert!(app.ui.files.visual_mode);
    assert!(app.input.search_query.is_empty());
    super::dispatch_test_key(&mut app, key("Esc"));
    assert!(!app.ui.files.visual_mode);
    app.ui.active_panel = SidePanel::Commits;
    super::dispatch_test_key(&mut app, key("Enter"));
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("path".into()));
    super::dispatch_test_key(&mut app, key("Esc"));
    assert!(app.ui.commits.tree_mode.active);
    super::dispatch_test_key(&mut app, key("Esc"));
    assert_eq!(app.view_context(), ViewContext::Commits);
}

#[test]
fn test_confirmation_context_consumes_only_confirmation_keys() {
    let mut app = fixture();
    for mode in [InputMode::CommitAllConfirm, InputMode::BranchSwitchConfirm] {
        app.input.mode = Some(mode);
        for text in ["q", "j", "Tab", "Backspace"] {
            assert!(super::map_test_key(&app, key(text)).is_none());
        }
        for text in ["y", "Y", "Enter"] {
            assert!(matches!(
                super::map_test_key(&app, key(text)),
                Some(
                    DomainAction::CommitAllConfirm(true) | DomainAction::BranchSwitchConfirm(true)
                )
            ));
        }
        for text in ["n", "N", "Esc"] {
            assert!(matches!(
                super::map_test_key(&app, key(text)),
                Some(
                    DomainAction::CommitAllConfirm(false)
                        | DomainAction::BranchSwitchConfirm(false)
                )
            ));
        }
        assert!(!app
            .shortcut_hints()
            .iter()
            .any(|(_, label)| label == "Quit"));
    }
}

#[test]
fn test_editor_hint_tracks_enter_behavior_and_focus() {
    let mut app = fixture();
    app.start_commit_editor();
    assert!(app
        .shortcut_hints()
        .contains(&("Enter".into(), "Confirm".into())));
    super::dispatch_test_key(&mut app, key("Tab"));
    assert_eq!(app.input.commit_focus, CommitFieldFocus::Description);
    assert!(app
        .shortcut_hints()
        .contains(&("Enter".into(), "Newline".into())));
    super::dispatch_test_key(&mut app, key("Enter"));
    assert_eq!(app.input.commit_description_buffer, "\n");
}

#[test]
fn test_shared_discard_selector_deduplicates_and_excludes_untracked_files() {
    let mut app = fixture();
    app.ui.files.tree_nodes = vec![
        node("src", FileTreeNodeStatus::Directory, 0),
        node("src/a", FileTreeNodeStatus::Staged(FileStatus::Modified), 1),
        node(
            "src/a",
            FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            1,
        ),
        node("src/new", FileTreeNodeStatus::Untracked, 1),
        node(
            "other",
            FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            0,
        ),
    ];
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    let borrowed = snapshot.as_snapshot();
    let snapshot_paths = borrowed
        .command_env()
        .selection
        .file_selection()
        .prepare_discard_targets_from_selection();
    assert_eq!(snapshot_paths, app.prepare_discard_targets_from_selection());
    assert_eq!(snapshot_paths, [std::path::PathBuf::from("src/a")]);
    assert_eq!(
        signature(super::map_test_key(&app, key("d"))),
        signature(app.resolve_command_palette_command("discard"))
    );
}

#[test]
fn test_partial_visual_directory_selection_does_not_expand_scope() {
    let mut app = fixture();
    app.ui.files.visual_mode = true;
    app.ui.files.visual_anchor = Some(0);
    app.ui.files.panel.list_state.select(Some(1));
    let selected = app.visual_selected_indices();
    let (stage, unstage) = app.partition_toggle_targets(&selected);
    assert!(stage.is_empty());
    assert_eq!(unstage, [std::path::PathBuf::from("src/a")]);
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    assert_eq!(
        snapshot
            .as_snapshot()
            .command_env()
            .selection
            .file_selection()
            .prepare_discard_targets_from_selection(),
        [std::path::PathBuf::from("src/a")]
    );
    assert_eq!(
        signature(super::map_test_key(&app, key("Space"))),
        signature(app.resolve_command_palette_command("stage"))
    );
}

#[test]
fn test_directory_stage_selector_stops_at_next_sibling() {
    let mut app = fixture();
    app.ui.files.tree_nodes[2].status = FileTreeNodeStatus::Staged(FileStatus::Modified);
    assert!(
        matches!(super::map_test_key(&app, key("Space")), Some(DomainAction::UnstageFile(path)) if path == std::path::Path::new("src"))
    );
    assert_eq!(
        signature(super::map_test_key(&app, key("Space"))),
        signature(app.resolve_command_palette_command("stage"))
    );
}

#[test]
fn test_branch_list_and_each_branch_history_have_separate_search_queries() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::LocalBranches;
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("main".into()));
    super::dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.input.search_query.is_empty());
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("test".into()));
    super::dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    assert_eq!(app.input.search_query, "main");
    app.ui.branches.panel.list_state.select(Some(1));
    super::dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.input.search_query.is_empty());
    super::dispatch_test_action(
        &mut app,
        DomainAction::SearchSetQuery("feature query".into()),
    );
    super::dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    app.ui.branches.panel.list_state.select(Some(0));
    super::dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert_eq!(app.input.search_query, "test");
    assert!(app.has_search_for_active_scope());
    assert!(matches!(
        super::map_test_key(&app, key("n")),
        Some(DomainAction::SearchNext)
    ));
}

#[test]
fn test_commit_file_search_is_scoped_to_revision_identity() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::Commits;
    super::dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("first files".into()));
    super::dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    app.ui.commits.panel.list_state.select(Some(1));
    super::dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert!(app.input.search_query.is_empty());
    super::dispatch_test_action(
        &mut app,
        DomainAction::SearchSetQuery("second files".into()),
    );
    super::dispatch_test_action(&mut app, DomainAction::RevisionCloseTree);
    app.ui.commits.panel.list_state.select(Some(0));
    super::dispatch_test_action(&mut app, DomainAction::RevisionOpenTreeOrToggleDir);
    assert_eq!(app.input.search_query, "first files");
}

#[test]
fn test_every_advertised_binding_matches_palette_resolution() {
    let mut app = fixture();
    for panel in SidePanel::ALL {
        app.ui.active_panel = panel;
        for command in app.available_commands() {
            for binding in command.keys {
                assert_eq!(
                    signature(super::map_test_key(&app, key(&binding))),
                    signature(app.resolve_command_palette_command(command.name)),
                    "{} on {panel:?} ({binding})",
                    command.name
                );
            }
        }
    }
}

#[test]
fn test_context_and_diff_selectors_match_owned_snapshot_for_nested_views() {
    let mut app = fixture();
    app.ui.commits.tree_mode.active = true;
    app.ui.commits.tree_mode.selected_source = Some("abc1234567890".into());
    app.ui.commits.tree_mode.nodes = vec![node(
        "committed",
        FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        0,
    )];
    app.ui.stash.tree_mode.active = true;
    app.ui.stash.tree_mode.selected_source = Some(1);
    app.ui.stash.tree_mode.nodes = vec![node(
        "stashed",
        FileTreeNodeStatus::Unstaged(FileStatus::Modified),
        0,
    )];
    app.ui.branches.commits_subview_active = true;
    app.ui.branches.commits_subview_source = Some("feature".into());
    app.ui.branches.commits_subview.items = app.ui.commits.items.clone();
    for panel in SidePanel::ALL {
        app.ui.active_panel = panel;
        let snapshot = AppStateSnapshotOwned::from_app(&app.state);
        assert_eq!(snapshot.context, app.ui_context());
        assert_eq!(
            format!(
                "{:?}",
                snapshot.as_snapshot().command_env().selection.diff_target()
            ),
            format!("{:?}", app.selection().diff_target())
        );
    }
}

#[test]
fn test_refresh_is_executable_from_key_and_palette() {
    let mut by_key = fixture();
    let mut by_palette = fixture();
    assert!(matches!(
        super::map_test_key(&by_key, key("r")),
        Some(DomainAction::Refresh)
    ));
    assert!(matches!(
        by_palette.resolve_command_palette_command(" ReFrEsH "),
        Some(DomainAction::Refresh)
    ));
    super::dispatch_test_key(&mut by_key, key("r"));
    palette(&mut by_palette, "refresh");
    assert_eq!(
        by_key.pending_refresh_kind(),
        by_palette.pending_refresh_kind()
    );
    assert!(by_key.ui.commits.dirty);
}

#[test]
fn test_palette_renders_registry_keys_and_modal_help() {
    let mut keymap = Keymap::default();
    keymap
        .files
        .bindings
        .insert("toggle_stage".into(), vec!["x".into()]);
    let mut app = fixture_with_keymap(keymap);
    app.start_command_palette();
    app.input.buffer = "toggle stage".into();
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 40)).unwrap();
    terminal
        .draw(|frame| crate::ui::panels::render_command_palette(frame, &snapshot.as_snapshot()))
        .unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(content.contains("x  stage  (Stage)"));
    assert!(content.contains("Enter Run"));
    assert!(content.contains("Esc Cancel"));
    assert!(!content.contains("Space  stage"));
}

#[test]
fn test_palette_aliases_are_unique_and_context_unavailable_commands_are_hidden() {
    let mut aliases = std::collections::HashSet::new();
    for spec in commands::COMMANDS {
        for alias in spec.aliases {
            assert!(aliases.insert(alias), "duplicate alias {alias}");
        }
    }
    let app = fixture();
    assert!(app.resolve_command_palette_command("checkout").is_none());
    assert!(!app
        .available_commands()
        .iter()
        .any(|command| command.name == "checkout"));
    let env = app.command_env();
    assert_eq!(
        commands::resolve_key(&key("c"), app.keymap(), &env)
            .unwrap()
            .id,
        CommandId::Commit
    );
}

#[test]
fn test_palette_filter_and_execution_share_case_and_whitespace_normalization() {
    let mut app = fixture();
    app.ui.active_panel = SidePanel::LocalBranches;
    app.start_command_palette();
    app.input.buffer = "  BRANCH   new  ".into();
    let snapshot = AppStateSnapshotOwned::from_app(&app.state);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(140, 40)).unwrap();
    terminal
        .draw(|frame| crate::ui::panels::render_command_palette(frame, &snapshot.as_snapshot()))
        .unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(content.contains("n  branch new  (New)"));
    super::dispatch_test_key(&mut app, key("Enter"));
    assert_eq!(app.input.mode, Some(InputMode::CreateBranch));
}

#[test]
fn test_background_refresh_recomputes_search_command_availability() {
    let mut app = fixture();
    super::dispatch_test_action(&mut app, DomainAction::SearchSetQuery("src".into()));
    assert!(app.has_search_for_active_scope());
    super::dispatch_test_action(&mut app, DomainAction::Refresh);
    assert!(app.ui.files.tree_nodes.is_empty());
    assert_eq!(app.input.search_query, "src");
    assert!(!app.has_search_for_active_scope());
    assert!(!app
        .available_commands()
        .iter()
        .any(|command| command.name == "search next"));
}
