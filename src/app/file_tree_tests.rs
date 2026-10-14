use super::*;
use crate::file_tree::FileTreeNodeStatus;
use crate::flux::action::Action;
use crate::flux::dispatcher::Dispatcher;
use crate::flux::effects::{self, EffectRequest};
use std::path::Path;

fn fixture() -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(dir.path()).unwrap();
    for path in ["a.txt", "z.txt"] {
        std::fs::write(dir.path().join(path), "before\n").unwrap();
    }
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    for path in ["a.txt", "z.txt"] {
        std::fs::write(dir.path().join(path), "after\n").unwrap();
    }
    std::fs::create_dir(dir.path().join("new")).unwrap();
    std::fs::write(dir.path().join("new/file.txt"), "new\n").unwrap();
    let app = App::from_repo(Box::new(
        crate::git::Git2Repository::open(dir.path()).unwrap(),
    ))
    .unwrap();
    (dir, app)
}

fn select(app: &mut App, path: &str) {
    let index = app
        .ui
        .files
        .tree_nodes
        .iter()
        .position(|node| node.path == Path::new(path))
        .unwrap();
    app.ui.files.panel.list_state.select(Some(index));
}

fn assert_selected(app: &App, path: &str) {
    assert_eq!(
        app.selected_tree_node().map(|node| node.path.as_path()),
        Some(Path::new(path))
    );
}

async fn refresh(app: &mut App, selected: &str) {
    let mut dispatcher = Dispatcher::with_default_stores();
    let original = app.selected_tree_node().unwrap().path.clone();
    app.process_background_refresh_tick();
    let mut pending: std::collections::VecDeque<_> = app.take_effects().into();
    let mut complete_status_seen = false;
    // Check both the fast and complete status, before either can hide a cursor jump.
    while let Some(request) = pending.pop_front() {
        let fast = matches!(
            &request,
            EffectRequest::LoadBackground {
                load: BackgroundLoad::Status { fast: true, .. },
                ..
            }
        );
        let complete_status = matches!(
            &request,
            EffectRequest::LoadBackground {
                load: BackgroundLoad::Status { fast: false, .. },
                ..
            }
        );
        complete_status_seen |= complete_status;
        for action in effects::run(request, &app.runtime).await {
            let envelope = dispatcher.next_envelope(action);
            pending.extend(
                dispatcher
                    .dispatch(&mut app.state, envelope)
                    .commands
                    .into_iter()
                    .map(|command| {
                        let Command::Effect(effect) = command else {
                            panic!("expected background effect")
                        };
                        effect
                    }),
            );
        }
        if fast {
            assert_eq!(app.selected_tree_node().unwrap().path, original);
        } else if complete_status {
            assert_selected(app, selected);
        }
    }
    assert!(complete_status_seen);
    assert_selected(app, selected);
}

async fn toggle(app: &mut App) {
    let action = app
        .selection()
        .file_selection()
        .toggle_stage_action()
        .unwrap();
    let command = dispatch_test_action(app, action).unwrap();
    let Command::Effect(request) = command else {
        panic!("expected Git effect")
    };
    assert!(matches!(
        request,
        EffectRequest::StageFile(_) | EffectRequest::UnstageFile(_)
    ));
    let actions = effects::run(request, &app.runtime).await;
    assert!(matches!(actions.as_slice(), [Action::Domain(_)]));
    crate::flux::test_runtime::drive_actions(app, actions);
}

#[tokio::test]
async fn test_file_tree_stage_preserves_untracked_cursor_during_fast_refresh() {
    let (_dir, mut app) = fixture();
    select(&mut app, "a.txt");
    toggle(&mut app).await;
    // Navigation while the Git write refresh is pending must remain authoritative.
    select(&mut app, "new");
    refresh(&mut app, "new").await;
}

#[tokio::test]
async fn test_file_tree_stage_untracked_directory_preserves_cursor() {
    let (_dir, mut app) = fixture();
    select(&mut app, "new");
    toggle(&mut app).await;
    refresh(&mut app, "new").await;
    assert!(app.selected_tree_node().unwrap().is_dir);
}

#[tokio::test]
async fn test_file_tree_unstage_new_file_preserves_cursor() {
    let (_dir, mut app) = fixture();
    select(&mut app, "new");
    toggle(&mut app).await;
    refresh(&mut app, "new").await;
    select(&mut app, "new/file.txt");
    toggle(&mut app).await;
    // Git folds a wholly untracked directory back into one entry.
    refresh(&mut app, "new").await;
    assert!(matches!(
        app.selected_tree_node().unwrap().status,
        FileTreeNodeStatus::Untracked
    ));
}

#[test]
fn test_file_tree_refresh_preserves_visual_anchor_path_when_rows_are_inserted() {
    let (_dir, mut app) = fixture();
    select(&mut app, "z.txt");
    let anchor = app
        .ui
        .files
        .tree_nodes
        .iter()
        .position(|node| node.path == Path::new("new"))
        .unwrap();
    app.ui.files.visual_mode = true;
    app.ui.files.visual_anchor = Some(anchor);
    let mut status = app.git.status.clone();
    status.untracked.push(crate::git::FileEntry {
        path: "b/file.txt".into(),
        status: crate::git::FileStatus::New,
    });
    app.apply_status_refresh(status);
    assert_selected(&app, "z.txt");
    assert_eq!(
        app.ui.files.tree_nodes[app.ui.files.visual_anchor.unwrap()].path,
        Path::new("new")
    );
}

#[test]
fn test_file_tree_refresh_keeps_existing_directories_collapsed() {
    let (_dir, mut app) = fixture();
    let mut status = app.git.status.clone();
    status.untracked.push(crate::git::FileEntry {
        path: "b/file.txt".into(),
        status: crate::git::FileStatus::New,
    });
    app.apply_status_refresh(status.clone());
    select(&mut app, "b");
    app.toggle_selected_dir();
    app.apply_status_refresh(status);
    assert_selected(&app, "b");
    assert!(!app.selected_tree_node().unwrap().is_expanded);
    assert!(!app
        .ui
        .files
        .tree_nodes
        .iter()
        .any(|node| node.path == Path::new("b/file.txt")));
}

#[tokio::test]
async fn test_file_tree_stage_unstage_tracked_file_keeps_cursor_and_diff_target() {
    let (_dir, mut app) = fixture();
    select(&mut app, "z.txt");
    for staged in [true, false] {
        toggle(&mut app).await;
        refresh(&mut app, "z.txt").await;
        assert_eq!(
            matches!(
                app.selected_tree_node().unwrap().status,
                FileTreeNodeStatus::Staged(_)
            ),
            staged
        );
        assert!(
            matches!(app.selection().diff_target(), super::diff_loader::DiffTarget::File { path, .. } if path == Path::new("z.txt"))
        );
    }
}
