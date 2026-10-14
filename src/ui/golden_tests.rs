//! UI regression tests exercise production renderers against checked-in buffers.
use super::layout::render_layout;
use super::panels::{render_diff_panel, DiffViewProps};
use super::test_support::{assert_golden, draw, render, serialize, RenderedUi};
use crate::app::{
    dispatch_test_action, dispatch_test_key, App, CommitFieldFocus, DiffSide, InputMode, SidePanel,
};
use crate::file_tree::{build_file_tree, FileTreeNode, FileTreeNodeStatus};
use crate::flux::action::DomainAction;
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::git::patch::{
    HunkId, Patch, PatchChange, PatchFile, PatchHunk, PatchLine, PatchLineKind, PatchRequest,
    PatchSide, PatchSource,
};
use crate::git::{
    BranchInfo, CommitInfo, CommitSyncState, FileEntry, FileStatus, Git2Repository, GraphCell,
    StashInfo,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, layout::Rect, Terminal};
use std::sync::Arc;

struct Fixture {
    app: App,
    // Keep the repository alive until App and its Git handles have been dropped.
    _dir: tempfile::TempDir,
}

impl Fixture {
    fn empty() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        drop(repo);
        let app = App::from_repo(Box::new(Git2Repository::open(dir.path()).unwrap())).unwrap();
        Self { app, _dir: dir }
    }

    fn populated() -> Self {
        let mut fixture = Self::empty();
        let app = &mut fixture.app;
        let file = |path: &str, status| FileEntry {
            path: path.into(),
            status,
        };
        app.git.status.unstaged = vec![
            file("src/main.rs", FileStatus::Modified),
            file("src/old.rs", FileStatus::Deleted),
        ];
        app.git.status.staged = vec![file("README.md", FileStatus::Modified)];
        app.git.status.untracked = vec![file("notes.txt", FileStatus::New)];
        app.ui.files.expanded_dirs.insert("src".into());
        app.ui.files.tree_nodes = build_file_tree(
            &app.git.status.unstaged,
            &app.git.status.untracked,
            &app.git.status.staged,
            &app.ui.files.expanded_dirs,
        );
        app.ui.files.panel.list_state.select(Some(1));
        app.ui.branches.items = vec![
            BranchInfo {
                name: "main".into(),
                is_current: true,
            },
            BranchInfo {
                name: "feature/ui-tests".into(),
                is_current: false,
            },
            BranchInfo {
                name: "release".into(),
                is_current: false,
            },
        ];
        app.ui.commits.items = commits(3);
        app.ui.stash.items = vec![
            StashInfo {
                index: 0,
                message: "WIP UI tests".into(),
            },
            StashInfo {
                index: 1,
                message: "Save documentation".into(),
            },
        ];
        let patch = patch();
        app.git.current_diff = patch.display_lines();
        app.git.current_patch = Some(Arc::new(patch));
        app.push_log("git status", true);
        app.push_log("git fetch: offline", false);
        app.refresh_render_cache();
        fixture
    }

    fn snapshot(&mut self) -> AppStateSnapshotOwned {
        self.app.refresh_render_cache();
        let snapshot = AppStateSnapshotOwned::from_app(&self.app);
        assert!(
            !snapshot.has_pending_refresh_work,
            "fixtures must have no animated background work"
        );
        snapshot
    }
}

fn commits(count: usize) -> Vec<CommitInfo> {
    (0..count)
        .map(|index| {
            let oid = format!("{:07x}{index:033x}", 0xabc1230 + index);
            CommitInfo {
                oid: oid.clone(),
                message: format!("UI change {index}"),
                author: "Ada Lovelace".into(),
                graph: vec![GraphCell {
                    text: "●".into(),
                    lane: index % 2,
                    pipe_oid: Some(oid.clone()),
                    pipe_oids: vec![oid],
                }],
                time: "2026-01-01".into(),
                parent_count: 1,
                parent_oids: Vec::new(),
                sync_state: match index % 3 {
                    0 => CommitSyncState::DefaultBranch,
                    1 => CommitSyncState::RemoteBranch,
                    _ => CommitSyncState::LocalOnly,
                },
            }
        })
        .collect()
}

fn patch() -> Patch {
    let side = PatchSide {
        path: Some("README.md".into()),
        oid: None,
        mode: 0o100644,
    };
    Patch {
        request: PatchRequest::Worktree("README.md".into()),
        truncated: false,
        files: vec![PatchFile {
            source: PatchSource::Worktree,
            change: PatchChange::Modified,
            old: side.clone(),
            new: side,
            binary: false,
            encoded: Some(b"diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n@@ -1,2 +1,2 @@\n # Ratagit\n-Old description\n+Tested terminal UI\n".to_vec()),
            hunks: vec![PatchHunk {
                old_start: 1,
                old_lines: 2,
                new_start: 1,
                new_lines: 2,
                header: b"@@ -1,2 +1,2 @@\n".to_vec(),
                lines: vec![
                    PatchLine {
                        kind: PatchLineKind::Context,
                        old_line: Some(1),
                        new_line: Some(1),
                        content: b"# Ratagit\n".to_vec(),
                    },
                    PatchLine {
                        kind: PatchLineKind::Removed,
                        old_line: Some(2),
                        new_line: None,
                        content: b"Old description\n".to_vec(),
                    },
                    PatchLine {
                        kind: PatchLineKind::Added,
                        old_line: None,
                        new_line: Some(2),
                        content: b"Tested terminal UI\n".to_vec(),
                    },
                ],
            }],
        }],
    }
}

fn layout(snapshot: &AppStateSnapshotOwned, width: u16, height: u16) -> RenderedUi {
    render(width, height, |frame| {
        render_layout(frame, &snapshot.as_snapshot())
    })
}

#[test]
fn test_empty_layout_golden() {
    assert_golden(
        "layout_empty",
        &layout(&Fixture::empty().snapshot(), 80, 24),
    );
}

#[test]
fn test_populated_layout_panel_focus_goldens() {
    let mut fixture = Fixture::populated();
    for (panel, name) in [
        (SidePanel::Files, "layout_files"),
        (SidePanel::LocalBranches, "layout_branches"),
        (SidePanel::Commits, "layout_commits"),
        (SidePanel::Stash, "layout_stash"),
    ] {
        fixture.app.ui.active_panel = panel;
        assert_golden(name, &layout(&fixture.snapshot(), 120, 32));
    }
}

#[test]
fn test_overflow_expansion_and_scrolling_goldens() {
    let mut fixture = Fixture::populated();
    fixture.app.ui.files.tree_nodes = (0..30)
        .map(|index| FileTreeNode {
            path: format!("file_{index:02}.rs").into(),
            status: FileTreeNodeStatus::Unstaged(FileStatus::Modified),
            depth: 0,
            is_dir: false,
            is_expanded: false,
        })
        .collect();
    fixture.app.ui.branches.items = (0..30)
        .map(|index| BranchInfo {
            name: format!("branch_{index:02}"),
            is_current: index == 0,
        })
        .collect();
    fixture.app.ui.commits.items = commits(30);
    fixture.app.ui.files.panel.list_state.select(Some(29));
    fixture.app.ui.branches.panel.list_state.select(Some(29));
    fixture.app.ui.commits.panel.list_state.select(Some(29));
    for (panel, name) in [
        (SidePanel::Files, "overflow_files"),
        (SidePanel::LocalBranches, "overflow_branches"),
        (SidePanel::Commits, "overflow_commits"),
    ] {
        fixture.app.ui.active_panel = panel;
        assert_golden(name, &layout(&fixture.snapshot(), 120, 24));
    }
}

#[test]
fn test_narrow_layout_and_unicode_golden() {
    let mut fixture = Fixture::populated();
    fixture.app.ui.files.tree_nodes[1].path = "src/界面.rs".into();
    fixture.app.ui.commits.items[0].message = "修复界面 e\u{301}".into();
    assert_golden(
        "layout_narrow_unicode",
        &layout(&fixture.snapshot(), 60, 18),
    );
}

#[test]
fn test_search_and_visual_selection_from_actions_golden() {
    let mut fixture = Fixture::populated();
    for action in [
        DomainAction::ToggleVisualSelectMode,
        DomainAction::ListDown,
        DomainAction::StartSearchInput,
        DomainAction::InputChar('r'),
        DomainAction::InputChar('s'),
    ] {
        dispatch_test_action(&mut fixture.app, action);
    }
    assert!(fixture.app.ui.files.visual_mode);
    assert_eq!(fixture.app.input.mode, Some(InputMode::Search));
    assert_golden("files_visual_search", &layout(&fixture.snapshot(), 120, 32));
}

#[test]
fn test_search_highlights_all_panel_types_goldens() {
    for (panel, query, name) in [
        (SidePanel::LocalBranches, "MAIN", "search_branches"),
        (SidePanel::Commits, "UI", "search_commits"),
        (SidePanel::Stash, "UI", "search_stash"),
    ] {
        let mut fixture = Fixture::populated();
        fixture.app.ui.active_panel = panel;
        dispatch_test_action(&mut fixture.app, DomainAction::SearchSetQuery(query.into()));
        assert_golden(name, &layout(&fixture.snapshot(), 120, 32));
    }
}

#[test]
fn test_revision_subviews_and_graph_highlight_goldens() {
    let mut fixture = Fixture::populated();
    let nodes = fixture.app.ui.files.tree_nodes.clone();
    fixture.app.ui.active_panel = SidePanel::Commits;
    let highlighted_oid = fixture.app.ui.commits.items[0].oid.clone();
    fixture
        .app
        .ui
        .commits
        .highlighted_oids
        .insert(highlighted_oid);
    assert_golden(
        "commits_graph_highlight",
        &layout(&fixture.snapshot(), 120, 32),
    );
    fixture.app.ui.commits.tree_mode.active = true;
    fixture.app.ui.commits.tree_mode.selected_source =
        Some(fixture.app.ui.commits.items[0].oid.clone());
    fixture.app.ui.commits.tree_mode.nodes = nodes.clone();
    assert_golden("commit_files", &layout(&fixture.snapshot(), 120, 32));
    fixture.app.ui.active_panel = SidePanel::Stash;
    fixture.app.ui.stash.tree_mode.active = true;
    fixture.app.ui.stash.tree_mode.selected_source = Some(0);
    fixture.app.ui.stash.tree_mode.nodes = nodes;
    assert_golden("stash_files", &layout(&fixture.snapshot(), 120, 32));
    fixture.app.ui.active_panel = SidePanel::LocalBranches;
    fixture.app.ui.branches.commits_subview_active = true;
    fixture.app.ui.branches.commits_subview_source = Some("main".into());
    fixture.app.ui.branches.commits_subview.items = commits(3);
    assert_golden("branch_commits", &layout(&fixture.snapshot(), 120, 32));
    fixture.app.ui.branches.commits_subview_loading = true;
    assert_golden(
        "branch_commits_loading",
        &layout(&fixture.snapshot(), 120, 32),
    );
}

fn diff(snapshot: &AppStateSnapshotOwned) -> RenderedUi {
    render(80, 8, |frame| {
        let view = snapshot.as_snapshot();
        render_diff_panel(
            frame,
            frame.area(),
            DiffViewProps {
                hunks: view.command_env().diff,
                lines: view.current_diff,
                scroll: view.diff_scroll,
                active_panel: view.active_panel,
                is_loading: view.diff_loading,
            },
        );
    })
}

#[test]
fn test_diff_loading_empty_and_side_hints_goldens() {
    let mut snapshot = Fixture::empty().snapshot();
    snapshot.diff_loading = true;
    assert_golden("diff_loading", &diff(&snapshot));
    snapshot.diff_loading = false;
    for (panel, name) in [
        (SidePanel::Files, "diff_empty_files"),
        (SidePanel::LocalBranches, "diff_empty_branches"),
        (SidePanel::Commits, "diff_empty_commits"),
        (SidePanel::Stash, "diff_empty_stash"),
    ] {
        snapshot.active_panel = panel;
        assert_golden(name, &diff(&snapshot));
    }
    for (side, name) in [
        (DiffSide::Staged, "diff_empty_staged"),
        (DiffSide::Unstaged, "diff_empty_unstaged"),
    ] {
        snapshot.diff_interaction.side = Some(side);
        assert_golden(name, &diff(&snapshot));
    }
}

#[test]
fn test_diff_hunk_focus_and_selection_from_actions_golden() {
    let mut fixture = Fixture::populated();
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleDiffFocus);
    dispatch_test_action(&mut fixture.app, DomainAction::ToggleHunkSelection);
    assert!(fixture.app.ui.diff.focused);
    assert!(fixture
        .app
        .ui
        .diff
        .selected
        .contains(&HunkId { file: 0, hunk: 0 }));
    assert_golden("diff_hunk_selected", &layout(&fixture.snapshot(), 120, 32));
    let mut snapshot = fixture.snapshot();
    snapshot.diff_interaction.pending_write = Some(1);
    assert_golden("diff_hunk_applying", &diff(&snapshot));
    snapshot.diff_interaction.pending_write = None;
    Arc::make_mut(snapshot.current_patch.as_mut().unwrap()).truncated = true;
    assert_golden("diff_hunk_read_only", &diff(&snapshot));
}

#[test]
fn test_diff_scroll_and_ansi_colors_golden() {
    let mut snapshot = Fixture::populated().snapshot();
    snapshot.current_patch = None;
    snapshot.current_diff = vec![
        crate::git::DiffLine {
            kind: crate::git::DiffLineKind::Header,
            content: "hidden by scroll".into(),
        },
        crate::git::DiffLine {
            kind: crate::git::DiffLineKind::Context,
            content: "\u{1b}[1;31merror\u{1b}[0m plain \u{1b}[38;2;82;196;208mrgb\u{1b}[0m".into(),
        },
        crate::git::DiffLine {
            kind: crate::git::DiffLineKind::Added,
            content: "new line".into(),
        },
        crate::git::DiffLine {
            kind: crate::git::DiffLineKind::Removed,
            content: "old line".into(),
        },
    ];
    snapshot.diff_scroll = 1;
    assert_golden("diff_scroll_ansi", &diff(&snapshot));
    snapshot.diff_scroll = usize::MAX;
    assert_golden("diff_scroll_clamped", &diff(&snapshot));
}

#[test]
fn test_diff_unicode_cell_boundaries_golden() {
    let mut snapshot = Fixture::populated().snapshot();
    snapshot.current_patch = None;
    snapshot.current_diff = vec![crate::git::DiffLine {
        kind: crate::git::DiffLineKind::Context,
        content: "界面测试 e\u{301} 👩\u{200d}💻".into(),
    }];
    assert_golden("diff_unicode", &diff(&snapshot));
}

#[test]
fn test_overlay_and_input_cursor_goldens() {
    for (mode, name) in [
        (InputMode::CommitEditor, "overlay_commit_message"),
        (InputMode::StashEditor, "overlay_stash"),
        (InputMode::BranchSwitchConfirm, "overlay_switch_branch"),
        (InputMode::CommitAllConfirm, "overlay_commit_all"),
        (InputMode::CreateBranch, "input_create_branch"),
    ] {
        let mut fixture = Fixture::populated();
        fixture.app.input.mode = Some(mode);
        fixture.app.input.buffer = "feature/ui".into();
        fixture.app.input.commit_message_buffer = "test: lock UI buffers".into();
        fixture.app.input.commit_description_buffer =
            "Cover panels and overlays.\nKeep styles stable.".into();
        fixture.app.input.stash_message_buffer = "UI work in progress".into();
        fixture.app.input.stash_targets = vec!["README.md".into(), "notes.txt".into()];
        fixture.app.input.branch_switch_target = Some("feature/ui-tests".into());
        assert_golden(name, &layout(&fixture.snapshot(), 120, 40));
        if mode == InputMode::CommitEditor {
            dispatch_test_action(&mut fixture.app, DomainAction::InputTab);
            assert_eq!(
                fixture.app.input.commit_focus,
                CommitFieldFocus::Description
            );
            assert_golden(
                "overlay_commit_description",
                &layout(&fixture.snapshot(), 120, 40),
            );
        }
    }
}

#[test]
fn test_palette_key_input_filter_and_close_goldens() {
    let mut fixture = Fixture::populated();
    let mut terminal = Terminal::new(TestBackend::new(120, 40)).unwrap();
    let snapshot = fixture.snapshot();
    let baseline = draw(&mut terminal, |frame| {
        render_layout(frame, &snapshot.as_snapshot())
    });
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE),
    );
    assert_eq!(fixture.app.input.mode, Some(InputMode::CommandPalette));
    for ch in "commit".chars() {
        dispatch_test_key(
            &mut fixture.app,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE),
        );
    }
    let snapshot = fixture.snapshot();
    assert_golden(
        "palette_filtered",
        &draw(&mut terminal, |frame| {
            render_layout(frame, &snapshot.as_snapshot())
        }),
    );
    fixture.app.input.buffer = "no-such-command".into();
    assert_golden("palette_no_matches", &layout(&fixture.snapshot(), 120, 40));
    dispatch_test_key(
        &mut fixture.app,
        KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    );
    assert_eq!(fixture.app.input.mode, None);
    let snapshot = fixture.snapshot();
    let restored = draw(&mut terminal, |frame| {
        render_layout(frame, &snapshot.as_snapshot())
    });
    // Opening the palette adds a log entry. Compare with a fresh render of the
    // current state to verify popup erasure while preserving that log change.
    assert_eq!(layout(&snapshot, 120, 40).buffer, restored.buffer);
    assert_eq!(baseline.cursor, restored.cursor);
    assert_golden("palette_closed", &restored);
}

#[test]
fn test_resize_clears_previous_layout_golden() {
    let snapshot = Fixture::populated().snapshot();
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    draw(&mut terminal, |frame| {
        render_layout(frame, &snapshot.as_snapshot())
    });
    terminal.backend_mut().resize(60, 18);
    terminal.resize(Rect::new(0, 0, 60, 18)).unwrap();
    let resized = draw(&mut terminal, |frame| {
        render_layout(frame, &snapshot.as_snapshot())
    });
    assert_eq!(serialize(&resized), serialize(&layout(&snapshot, 60, 18)));
    assert_golden("layout_resized", &resized);
}

#[test]
fn test_loading_indicator_all_frames_golden() {
    let ui = render(24, 10, |frame| {
        let mut indicator =
            super::components::loading_indicator::LoadingIndicator::with_message("git status");
        for y in 0..10 {
            frame.render_widget(indicator.clone(), Rect::new(0, y, 24, 1));
            indicator.tick();
        }
    });
    assert_golden("loading_indicator_frames", &ui);
}

#[test]
fn test_rendering_does_not_mutate_snapshot() {
    let snapshot = Fixture::populated().snapshot();
    let before = (
        snapshot.files.panel.list_state,
        snapshot.branches.panel.list_state,
        snapshot.commits.panel.list_state,
        snapshot.stash.panel.list_state,
    );
    let first = layout(&snapshot, 120, 32);
    let second = layout(&snapshot, 120, 32);
    assert_eq!(serialize(&first), serialize(&second));
    assert_eq!(
        before,
        (
            snapshot.files.panel.list_state,
            snapshot.branches.panel.list_state,
            snapshot.commits.panel.list_state,
            snapshot.stash.panel.list_state
        )
    );
}
