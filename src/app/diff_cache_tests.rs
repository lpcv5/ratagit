//! Cache regressions use real Git repositories and the production Flux path.
use super::*;
use crate::background::{TaskPriority, TaskRequest};
use crate::flux::action::{Action, DomainAction, RuntimeEvent};
use crate::flux::command_output::CommandOutput;
use crate::flux::dispatcher::Dispatcher;
use crate::flux::effects::{self, EffectRequest};
use crate::flux::snapshot::AppStateSnapshotOwned;
use crate::git::Git2Repository;
use std::path::Path;

struct Fixture {
    directory: tempfile::TempDir,
    app: App,
    dispatcher: Dispatcher,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        git2::Repository::init(directory.path()).unwrap();
        commit(directory.path(), "initial", "base\n");
        let app =
            App::from_repo(Box::new(Git2Repository::open(directory.path()).unwrap())).unwrap();
        Self {
            directory,
            app,
            dispatcher: Dispatcher::with_default_stores(),
        }
    }

    fn dispatch(&mut self, action: Action) -> Vec<Command> {
        let envelope = self.dispatcher.next_envelope(action);
        self.dispatcher
            .dispatch(&mut self.app.state, envelope)
            .commands
    }

    fn execute(&mut self, request: EffectRequest) {
        for action in effects::execute(request, &*self.app.runtime.repo) {
            self.dispatch(action);
        }
    }

    fn observe(&mut self, load: BackgroundLoad) {
        let mut output = CommandOutput::default();
        self.app
            .state
            .queue_background(load, TaskPriority::High, &mut output);
        let Command::Effect(request) = output.drain().remove(0) else {
            panic!("read effect");
        };
        self.execute(request);
    }

    fn status(&mut self) {
        self.observe(BackgroundLoad::Status {
            fast: false,
            operation_epoch: self.app.git.operation.epoch,
        });
    }

    fn select_file(&mut self, path: &str) {
        self.app.ui.active_panel = SidePanel::Files;
        let selected = self
            .app
            .files
            .view()
            .tree_nodes
            .iter()
            .position(|node| node.path == Path::new(path))
            .unwrap();
        self.app.files.test_view_mut().panel.select(Some(selected));
    }

    fn request_diff(&mut self) -> Option<EffectRequest> {
        self.dispatch(Action::Domain(DomainAction::Runtime(
            RuntimeEvent::ReloadDiffNow,
        )))
        .into_iter()
        .find_map(|command| match command {
            Command::Effect(request @ EffectRequest::LoadBackground { .. }) => Some(request),
            _ => None,
        })
    }

    fn load_diff(&mut self) -> diff::DiffCacheIdentity {
        let effect = self.request_diff().expect("fresh read required");
        let EffectRequest::LoadBackground {
            request:
                TaskRequest {
                    load: BackgroundLoad::Diff { cache_key, .. },
                    ..
                },
        } = &effect
        else {
            panic!("diff effect");
        };
        let key = cache_key.clone();
        self.execute(effect);
        key
    }

    fn text(&self) -> String {
        self.app
            .diff
            .document()
            .iter()
            .map(|line| line.content.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn write(&mut self, action: DomainAction) {
        let effect = self
            .dispatch(Action::Domain(action))
            .into_iter()
            .find_map(|command| match command {
                Command::Effect(effect) => Some(effect),
                _ => None,
            })
            .expect("write effect");
        self.execute(effect);
        self.status();
    }
}

fn commit(path: &Path, message: &str, content: &str) {
    std::fs::write(path.join("tracked.txt"), content).unwrap();
    let repo = git2::Repository::open(path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("tracked.txt")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let signature = git2::Signature::now("tester", "test@example.com").unwrap();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent.iter().collect::<Vec<_>>(),
    )
    .unwrap();
}

#[test]
fn test_external_edit_with_unchanged_status_labels_invalidates_cached_diff() {
    let mut fixture = Fixture::new();
    std::fs::write(fixture.directory.path().join("tracked.txt"), "first edit\n").unwrap();
    fixture.status();
    fixture.select_file("tracked.txt");
    let first = fixture.load_diff();
    let retained = AppStateSnapshotOwned::from_app(&fixture.app.state);
    assert!(fixture.request_diff().is_none(), "repeat selection is warm");
    std::fs::write(
        fixture.directory.path().join("tracked.txt"),
        "second edit\n",
    )
    .unwrap();
    fixture.status();
    let second = fixture.load_diff();
    assert_eq!(first.target, second.target);
    assert_ne!(first.revision, second.revision);
    assert!(fixture.text().contains("second edit"));
    assert!(retained
        .current_diff
        .iter()
        .any(|line| line.content.contains("first edit")));
    assert!(!retained
        .current_diff
        .iter()
        .any(|line| line.content.contains("second edit")));
}

#[test]
fn test_branch_moving_under_same_name_requires_new_log_read() {
    let mut fixture = Fixture::new();
    fixture.app.ui.active_panel = SidePanel::LocalBranches;
    fixture.app.revisions.branches.panel.select(Some(0));
    let old = fixture.load_diff();
    assert!(fixture.text().contains("initial"));
    assert!(fixture.request_diff().is_none());
    commit(fixture.directory.path(), "advanced tip", "new tree\n");
    fixture.observe(BackgroundLoad::Branches);
    let current = fixture.load_diff();
    assert_eq!(old.target, current.target);
    assert_ne!(old.revision, current.revision);
    assert!(fixture.text().contains("advanced tip"));
    assert!(
        matches!(current.target, diff::DiffCacheKey::Branch { limit, .. } if limit == diff::BRANCH_DIFF_LIMIT)
    );
}

#[test]
fn test_reusing_stash_index_zero_does_not_reuse_previous_stash_patch() {
    let mut fixture = Fixture::new();
    let stash = |path: &Path, content: &str| {
        std::fs::write(path.join("tracked.txt"), content).unwrap();
        let mut repo = git2::Repository::open(path).unwrap();
        let signature = git2::Signature::now("tester", "test@example.com").unwrap();
        repo.stash_save(&signature, content, None).unwrap();
    };
    stash(fixture.directory.path(), "first stash\n");
    fixture.observe(BackgroundLoad::Stashes);
    fixture.app.ui.active_panel = SidePanel::Stash;
    fixture.app.revisions.stash.panel.select(Some(0));
    let old = fixture.load_diff();
    let retained = AppStateSnapshotOwned::from_app(&fixture.app.state);
    assert!(fixture.text().contains("first stash"));
    assert!(fixture.request_diff().is_none());
    stash(fixture.directory.path(), "second stash\n");
    fixture.observe(BackgroundLoad::Stashes);
    let current = fixture.load_diff();
    assert_eq!(old.target, current.target);
    assert_ne!(old.revision, current.revision);
    assert!(fixture.text().contains("second stash"));
    assert!(!fixture.text().contains("first stash"));
    assert!(retained
        .current_diff
        .iter()
        .any(|line| line.content.contains("first stash")));
}

#[test]
fn test_staging_roundtrips_preserve_tracked_and_untracked_source_identity() {
    for (path, initial) in [
        ("tracked.txt", diff::FileDiffSource::Combined),
        ("new.txt", diff::FileDiffSource::Untracked),
    ] {
        let mut fixture = Fixture::new();
        std::fs::write(fixture.directory.path().join(path), "changed\n").unwrap();
        fixture.status();
        fixture.select_file(path);
        let first = fixture.load_diff();
        assert!(
            matches!(first.target, diff::DiffCacheKey::File { source, .. } if source == initial)
        );
        fixture.write(DomainAction::StageFile(path.into()));
        fixture.select_file(path);
        let staged = fixture.load_diff();
        assert!(matches!(
            staged.target,
            diff::DiffCacheKey::File {
                source: diff::FileDiffSource::Combined,
                ..
            }
        ));
        fixture.write(DomainAction::UnstageFile(path.into()));
        fixture.select_file(path);
        let restored = fixture.load_diff();
        assert_eq!(first.target, restored.target);
        assert_ne!(first.revision, restored.revision);
        assert!(fixture.text().contains("changed"));
    }
}

#[test]
fn test_freshness_rejects_success_and_failure_even_with_current_task_generation() {
    for fail in [false, true] {
        let mut fixture = Fixture::new();
        std::fs::write(fixture.directory.path().join("tracked.txt"), "old edit\n").unwrap();
        fixture.status();
        fixture.select_file("tracked.txt");
        let request = fixture.request_diff().unwrap();
        let Action::System(crate::flux::action::SystemAction::BackgroundCompleted(mut completed)) =
            effects::execute(request, &*fixture.app.runtime.repo).remove(0)
        else {
            panic!("completion");
        };
        std::fs::write(fixture.directory.path().join("tracked.txt"), "new edit\n").unwrap();
        // Exercise freshness independently of TaskManager's cancellation safety net.
        fixture
            .app
            .diff
            .source_refreshed(diff::DiffRefreshSource::Status);
        assert!(fixture.app.tasks.is_current(&completed));
        if fail {
            completed.result = Err(crate::git::GitError::from(git2::Error::new(
                git2::ErrorCode::Locked,
                git2::ErrorClass::Index,
                "obsolete failure must stay silent",
            ))
            .context("diff"));
        }
        let logs = fixture.app.command_log.len();
        fixture.app.ui.dirty.clear();
        fixture.dispatch(Action::System(
            crate::flux::action::SystemAction::BackgroundCompleted(completed),
        ));
        assert_eq!(fixture.app.command_log.len(), logs);
        assert!(!fixture.app.ui.dirty.command_log());
        assert_eq!(fixture.app.diff.cache_stats().entries, 0);
        assert_eq!(fixture.app.diff.cache_stats().rejected, 1);
        fixture.load_diff();
        assert!(fixture.text().contains("new edit"));
        assert!(!fixture.text().contains("old edit"));
    }
}

#[test]
fn test_byte_eviction_preserves_retained_snapshot_and_lossless_patch() {
    let mut fixture = Fixture::new();
    for (path, text) in [("a.txt", "first\n"), ("b.txt", "second\n")] {
        std::fs::write(fixture.directory.path().join(path), text).unwrap();
    }
    fixture.status();
    fixture.select_file("a.txt");
    fixture.load_diff();
    let a_bytes = fixture.app.diff.cache_stats().retained_bytes;
    fixture.select_file("b.txt");
    fixture.load_diff();
    let b_bytes = fixture.app.diff.cache_stats().retained_bytes - a_bytes;
    fixture
        .app
        .diff
        .set_test_cache_limits(50, a_bytes.max(b_bytes));
    fixture.select_file("a.txt");
    fixture.load_diff();
    let retained = AppStateSnapshotOwned::from_app(&fixture.app.state);
    let patch = retained.current_patch.as_ref().unwrap();
    fixture.select_file("b.txt");
    fixture.load_diff();
    let stats = fixture.app.diff.cache_stats();
    assert_eq!(stats.entries, 1);
    assert_eq!(stats.evictions, 1);
    assert!(stats.retained_bytes <= stats.max_bytes);
    assert!(retained
        .current_diff
        .iter()
        .any(|line| line.content.contains("first")));
    assert!(patch.files.iter().any(|file| file
        .hunks
        .iter()
        .any(|hunk| hunk.lines.iter().any(|line| line.content == b"first\n"))));
    assert!(fixture.text().contains("second"));
}

#[test]
fn test_payload_identity_must_match_the_registered_request() {
    let mut fixture = Fixture::new();
    std::fs::write(
        fixture.directory.path().join("tracked.txt"),
        "current edit\n",
    )
    .unwrap();
    fixture.status();
    fixture.select_file("tracked.txt");
    let request = fixture.request_diff().unwrap();
    let Action::System(crate::flux::action::SystemAction::BackgroundCompleted(mut completed)) =
        effects::execute(request, &*fixture.app.runtime.repo).remove(0)
    else {
        panic!("completion");
    };
    let BackgroundPayload::Diff { cache_key, .. } =
        std::sync::Arc::make_mut(completed.result.as_mut().unwrap())
    else {
        panic!("diff payload");
    };
    cache_key.target = diff::DiffCacheKey::File {
        path: "other.txt".into(),
        source: diff::FileDiffSource::Combined,
    };
    assert!(fixture.app.tasks.is_current(&completed));
    assert!(!fixture.app.background_completion_is_current(&completed));
    fixture.dispatch(Action::System(
        crate::flux::action::SystemAction::BackgroundCompleted(completed),
    ));
    assert_eq!(fixture.app.diff.cache_stats().entries, 0);
    assert_eq!(fixture.app.diff.cache_stats().rejected, 1);
    fixture.load_diff();
    assert!(fixture.text().contains("current edit"));
}
