use super::operation::*;
use super::patch::*;
use super::*;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
mod lines;
mod preview;

#[test]
fn test_combined_file_patch_groups_sources_and_applies_each_hunk_independently() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let initial = fixture.worktree();
    fixture
        .repo
        .apply_hunks(
            &initial,
            &[HunkId { file: 0, hunk: 1 }],
            PatchDirection::Stage,
        )
        .unwrap();
    let combined = fixture.patch(PatchRequest::File("file.txt".into()));
    assert_eq!(combined.display_file_count(), 1);
    let ids = combined.hunk_ids();
    assert_eq!(ids.len(), 2);
    assert_eq!(combined.files[ids[0].file].source, PatchSource::Worktree);
    assert_eq!(combined.files[ids[1].file].source, PatchSource::Index);
    let rows = combined.display_rows();
    assert_eq!(
        rows.iter()
            .filter(|row| row.line.content.starts_with("--- "))
            .count(),
        1
    );
    let mut previous = None;
    for id in combined.cursor_line_ids() {
        let row = combined.line_row(id).unwrap();
        assert!(previous.is_none_or(|previous| previous < row));
        assert_eq!(rows[row].hunk, Some(id.hunk()));
        previous = Some(row);
    }
    fixture
        .repo
        .apply_hunks(&combined, &[ids[1]], PatchDirection::Unstage)
        .unwrap();
    assert!(fixture.git_ok(&["diff", "--cached"]).is_empty());
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
    let remaining = fixture.patch(PatchRequest::File("file.txt".into()));
    fixture
        .repo
        .apply_hunks(&remaining, &remaining.hunk_ids(), PatchDirection::Stage)
        .unwrap();
    let staged = fixture.patch(PatchRequest::File("file.txt".into()));
    assert!(staged
        .files
        .iter()
        .all(|file| file.source == PatchSource::Index));
    assert_eq!(staged.hunk_ids().len(), 2);
}

struct Fixture {
    dir: TempDir,
    repo: Git2Repository,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let raw = git2::Repository::init(dir.path()).unwrap();
        raw.config().unwrap().set_str("user.name", "Test").unwrap();
        raw.config()
            .unwrap()
            .set_str("user.email", "test@example.com")
            .unwrap();
        raw.config()
            .unwrap()
            .set_bool("commit.gpgsign", false)
            .unwrap();
        raw.config()
            .unwrap()
            .set_bool("core.autocrlf", false)
            .unwrap();
        std::fs::write(dir.path().join("file.txt"), Self::base()).unwrap();
        let mut index = raw.index().unwrap();
        index.add_path(Path::new("file.txt")).unwrap();
        index.write().unwrap();
        let id = index.write_tree().unwrap();
        let tree = raw.find_tree(id).unwrap();
        let sig = raw.signature().unwrap();
        raw.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        let repo = Git2Repository::open(dir.path()).unwrap();
        Self { dir, repo }
    }

    fn base() -> String {
        (1..=40).map(|i| format!("line {i}\n")).collect()
    }
    fn changed() -> String {
        Self::base()
            .replace("line 3\n", "first change\n")
            .replace("line 30\n", "second change\n")
    }
    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        std::fs::write(self.dir.path().join(path), bytes).unwrap();
    }
    fn patch(&self, request: PatchRequest) -> Patch {
        self.repo.patch(request).unwrap().unwrap()
    }
    fn worktree(&self) -> Patch {
        self.patch(PatchRequest::Worktree("file.txt".into()))
    }
    fn git(&self, args: &[&str]) -> std::process::Output {
        Command::new("git")
            .args(args)
            .env("GIT_EDITOR", "true")
            .env("GIT_SEQUENCE_EDITOR", "true")
            .current_dir(self.dir.path())
            .output()
            .unwrap()
    }
    fn git_ok(&self, args: &[&str]) -> String {
        let output = self.git(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }
    fn conflict(&self, rebase: bool) {
        self.git_ok(&["checkout", "-b", "feature"]);
        self.write("file.txt", "feature\n");
        self.git_ok(&["commit", "-am", "feature"]);
        self.git_ok(&["checkout", "-b", "other", "HEAD~1"]);
        self.write("file.txt", "other\n");
        self.git_ok(&["commit", "-am", "other"]);
        if rebase {
            self.git_ok(&["checkout", "feature"]);
            assert!(!self.git(&["rebase", "other"]).status.success());
        } else {
            assert!(!self.git(&["merge", "feature"]).status.success());
        }
    }
}

#[test]
fn test_patch_retains_two_hunks_and_line_numbers() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    assert!(!patch.truncated);
    assert_eq!(patch.files.len(), 1);
    let file = &patch.files[0];
    assert_eq!(file.source, PatchSource::Worktree);
    assert_eq!(file.hunks.len(), 2);
    let removed = file.hunks[0]
        .lines
        .iter()
        .find(|l| l.kind == PatchLineKind::Removed)
        .unwrap();
    let added = file.hunks[0]
        .lines
        .iter()
        .find(|l| l.kind == PatchLineKind::Added)
        .unwrap();
    assert_eq!((removed.old_line, removed.new_line), (Some(3), None));
    assert_eq!((added.old_line, added.new_line), (None, Some(3)));
    assert_eq!(added.content, b"first change\n");
    assert_eq!(file.old.path.as_deref(), Some(Path::new("file.txt")));
    assert!(file.old.oid.is_some());
}

#[test]
fn test_stage_only_selected_hunk_without_changing_worktree() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    fixture
        .repo
        .stage_hunks(&fixture.worktree(), 0, &[0])
        .unwrap();
    let staged = fixture.git_ok(&["show", ":file.txt"]);
    assert!(staged.contains("first change"));
    assert!(!staged.contains("second change"));
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        Fixture::changed()
    );
    let remaining = fixture.worktree();
    assert_eq!(remaining.files[0].hunks.len(), 1);
    fixture.repo.stage_hunks(&remaining, 0, &[0]).unwrap();
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::changed());
}

#[test]
fn test_stage_later_hunk_independently() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    fixture
        .repo
        .stage_hunks(&fixture.worktree(), 0, &[1])
        .unwrap();
    let staged = fixture.git_ok(&["show", ":file.txt"]);
    assert!(!staged.contains("first change"));
    assert!(staged.contains("second change"));
}

#[test]
fn test_stage_rejects_stale_worktree_patch_and_preserves_index() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    fixture.write("file.txt", "changed again\n");
    assert!(fixture.repo.stage_hunks(&patch, 0, &[0]).is_err());
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::base());
}

#[test]
fn test_stage_rejects_stale_index_patch() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    fixture.repo.stage(Path::new("file.txt")).unwrap();
    assert!(fixture.repo.stage_hunks(&patch, 0, &[0]).is_err());
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::changed());
}

#[test]
fn test_invalid_hunk_selection_never_writes() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    for (file, hunks) in [(4, vec![0]), (0, vec![]), (0, vec![0, 0]), (0, vec![9])] {
        assert!(fixture.repo.stage_hunks(&patch, file, &hunks).is_err());
    }
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::base());
}

#[test]
fn test_patch_preserves_crlf_non_utf8_and_missing_newline() {
    let fixture = Fixture::new();
    fixture.write("file.txt", b"before\r\n");
    fixture.repo.stage(Path::new("file.txt")).unwrap();
    fixture.repo.commit("crlf").unwrap();
    fixture.write("file.txt", b"after\xff\r\nlast");
    let patch = fixture.worktree();
    let lines = &patch.files[0].hunks[0].lines;
    assert!(lines.iter().any(|l| l.content == b"after\xff\r\n"));
    assert!(lines.iter().any(|l| l.kind == PatchLineKind::NoNewline));
    fixture.repo.stage_hunks(&patch, 0, &[0]).unwrap();
    let output = fixture.git(&["show", ":file.txt"]);
    assert_eq!(output.stdout, b"after\xff\r\nlast");
}

#[test]
fn test_binary_patch_is_explicit_and_not_partially_stageable() {
    let fixture = Fixture::new();
    fixture.write("file.txt", b"binary\0data");
    let patch = fixture.worktree();
    assert!(patch.files[0].binary);
    assert!(patch.files[0].hunks.is_empty());
    assert!(patch.validate_hunks(0, &[0]).is_err());
    assert!(patch
        .display_lines()
        .iter()
        .any(|l| l.content == "<binary file>"));
}

#[test]
fn test_truncated_patch_is_explicit_and_not_stageable() {
    let fixture = Fixture::new();
    fixture.write("file.txt", "new\n".repeat(6000));
    let patch = fixture.worktree();
    assert!(patch.truncated);
    assert!(patch.validate_hunks(0, &[0]).is_err());
    assert!(
        patch
            .files
            .iter()
            .flat_map(|f| &f.hunks)
            .map(|h| h.lines.len())
            .sum::<usize>()
            < 5000
    );
    assert!(patch
        .display_lines()
        .last()
        .unwrap()
        .content
        .contains("truncated"));
}

#[test]
fn test_untracked_and_deleted_patch_sides() {
    let fixture = Fixture::new();
    fixture.write("new.txt", "new\n");
    let patch = fixture.patch(PatchRequest::Untracked("new.txt".into()));
    assert_eq!(patch.files[0].source, PatchSource::Untracked);
    assert!(patch.files[0].old.path.is_none());
    assert_eq!(patch.files[0].hunks[0].lines[0].new_line, Some(1));
    assert!(patch.validate_hunks(0, &[0]).is_ok());
    std::fs::remove_file(fixture.dir.path().join("file.txt")).unwrap();
    let deleted = fixture.worktree();
    assert_eq!(deleted.files[0].change, PatchChange::Deleted);
    assert!(deleted.files[0].new.path.is_none());
}

#[test]
fn test_untracked_patch_rejects_tracked_paths() {
    let fixture = Fixture::new();
    fixture.write("file.txt", "change\n");
    assert!(fixture
        .repo
        .patch(PatchRequest::Untracked("file.txt".into()))
        .is_err());
}

#[test]
fn test_patch_pathspec_treats_glob_characters_literally() {
    let fixture = Fixture::new();
    fixture.write("[a].txt", "literal\n");
    fixture.write("a.txt", "other\n");
    let patch = fixture.patch(PatchRequest::Untracked("[a].txt".into()));
    assert_eq!(patch.files.len(), 1);
    assert_eq!(
        patch.files[0].new.path.as_deref(),
        Some(Path::new("[a].txt"))
    );
}

#[test]
fn test_directory_patch_keeps_index_worktree_and_untracked_sources() {
    let fixture = Fixture::new();
    std::fs::create_dir(fixture.dir.path().join("dir")).unwrap();
    fixture.write("dir/file.txt", "base\n");
    fixture.repo.stage(Path::new("dir/file.txt")).unwrap();
    fixture.repo.commit("dir").unwrap();
    fixture.write("dir/file.txt", "index\n");
    fixture.repo.stage(Path::new("dir/file.txt")).unwrap();
    fixture.write("dir/file.txt", "worktree\n");
    fixture.write("dir/new.txt", "new\n");
    let patch = fixture.patch(PatchRequest::Directory("dir".into()));
    assert_eq!(patch.files.len(), 3);
    assert_eq!(
        patch.files.iter().map(|f| &f.source).collect::<Vec<_>>(),
        vec![
            &PatchSource::Index,
            &PatchSource::Worktree,
            &PatchSource::Untracked
        ]
    );
}

#[test]
fn test_commit_and_stash_patch_preserve_provenance() {
    let fixture = Fixture::new();
    fixture.write("file.txt", "committed\n");
    fixture.repo.stage(Path::new("file.txt")).unwrap();
    let oid = fixture.repo.commit("commit").unwrap();
    let patch = fixture.patch(PatchRequest::Commit {
        oid: oid.clone(),
        path: Some("file.txt".into()),
    });
    assert_eq!(patch.files[0].source, PatchSource::Commit(oid));
    assert!(patch.validate_hunks(0, &[0]).is_err());
    fixture.write("file.txt", "stash\n");
    fixture
        .repo
        .stash_push_paths(&["file.txt".into()], "stash")
        .unwrap();
    let patch = fixture.patch(PatchRequest::Stash {
        index: 0,
        path: Some("file.txt".into()),
    });
    assert_eq!(patch.files[0].source, PatchSource::Stash(0));
    assert!(patch.files[0].hunks[0]
        .lines
        .iter()
        .any(|l| l.content == b"stash\n"));
}

#[test]
fn test_merge_conflicts_and_abort_observed_from_real_repository() {
    let fixture = Fixture::new();
    fixture.conflict(false);
    let observed = fixture.repo.operation_snapshot().unwrap();
    assert_eq!(observed.kind, Some(OperationKind::Merge));
    assert_eq!(
        observed.conflicts,
        vec![std::path::PathBuf::from("file.txt")].into()
    );
    assert!(!observed.allows(OperationControl::Continue));
    assert!(!observed.allows(OperationControl::Skip));
    assert!(fixture
        .repo
        .control_operation(OperationControl::Continue)
        .is_err());
    fixture
        .repo
        .control_operation(OperationControl::Abort)
        .unwrap();
    assert_eq!(
        fixture.repo.operation_snapshot().unwrap(),
        RepositoryOperation::default()
    );
    assert_eq!(
        std::fs::read_to_string(fixture.dir.path().join("file.txt")).unwrap(),
        "other\n"
    );
}

#[test]
fn test_rebase_conflicts_progress_and_continue_after_resolution() {
    let fixture = Fixture::new();
    fixture.conflict(true);
    let observed = fixture.repo.operation_snapshot().unwrap();
    assert!(matches!(observed.kind, Some(OperationKind::Rebase { .. })));
    assert_eq!(
        observed.progress,
        Some(OperationProgress {
            current: 1,
            total: 1
        })
    );
    fixture.write("file.txt", "resolved\n");
    fixture.repo.stage(Path::new("file.txt")).unwrap();
    assert!(fixture
        .repo
        .operation_snapshot()
        .unwrap()
        .conflicts
        .is_empty());
    fixture
        .repo
        .control_operation(OperationControl::Continue)
        .unwrap();
    assert_eq!(
        fixture.repo.operation_snapshot().unwrap(),
        RepositoryOperation::default()
    );
    assert_eq!(fixture.git_ok(&["show", "HEAD:file.txt"]), "resolved\n");
}

#[test]
fn test_rebase_skip_and_abort() {
    for control in [OperationControl::Skip, OperationControl::Abort] {
        let fixture = Fixture::new();
        fixture.conflict(true);
        fixture.repo.control_operation(control).unwrap();
        assert_eq!(
            fixture.repo.operation_snapshot().unwrap(),
            RepositoryOperation::default()
        );
        let expected = if control == OperationControl::Skip {
            "other\n"
        } else {
            "feature\n"
        };
        assert_eq!(fixture.git_ok(&["show", "HEAD:file.txt"]), expected);
    }
}

#[test]
fn test_operation_control_rejects_idle_repository() {
    let fixture = Fixture::new();
    for control in [
        OperationControl::Continue,
        OperationControl::Skip,
        OperationControl::Abort,
    ] {
        assert!(fixture.repo.control_operation(control).is_err());
    }
}

#[test]
fn test_operation_machine_rejects_overlapping_and_stale_events() {
    let mut machine = OperationMachine::default();
    machine.observe(
        0,
        RepositoryOperation {
            kind: Some(OperationKind::Rebase { interactive: true }),
            ..Default::default()
        },
    );
    let id = machine.begin(OperationControl::Continue).unwrap();
    assert!(machine.begin(OperationControl::Abort).is_err());
    assert!(!machine.observe(0, RepositoryOperation::default()));
    assert!(!machine.observe(id, RepositoryOperation::default()));
    assert!(!machine.finish(id + 1, Ok(()), Some(Default::default())));
    assert!(machine.finish(
        id,
        Err("conflict".into()),
        Some(RepositoryOperation {
            kind: Some(OperationKind::Rebase { interactive: true }),
            conflicts: vec!["file.txt".into()].into(),
            progress: None
        })
    ));
    assert!(matches!(
        machine.command,
        OperationCommandState::Failed { .. }
    ));
    assert!(machine.begin(OperationControl::Continue).is_err());
    let retry = machine.begin(OperationControl::Abort).unwrap();
    assert!(!machine.finish(id, Ok(()), Some(Default::default())));
    assert!(machine.finish(retry, Ok(()), Some(Default::default())));
    assert_eq!(machine.command, OperationCommandState::Ready);
    assert!(machine.observed.kind.is_none());
}

#[test]
fn test_rebase_plan_validation_tracks_retained_predecessors() {
    let oid = |n| format!("{n:040x}");
    let mut plan = RebasePlan {
        onto: oid(1),
        steps: vec![
            RebaseStep {
                action: RebaseAction::Drop,
                oid: oid(2),
            },
            RebaseStep {
                action: RebaseAction::Squash,
                oid: oid(3),
            },
        ],
    };
    assert!(plan.validate().is_err());
    plan.steps[0].action = RebaseAction::Pick;
    assert!(plan.validate().is_ok());
    plan.steps[1].action = RebaseAction::Fixup;
    assert!(plan.validate().is_ok());
    plan.steps[1].oid = plan.steps[0].oid.clone();
    assert!(plan.validate().is_err());
    plan.steps[1] = RebaseStep {
        action: RebaseAction::Edit,
        oid: oid(3),
    };
    assert!(plan.validate().is_ok());
    plan.steps[0].action = RebaseAction::Reword;
    assert!(plan.validate().is_ok());
    plan.onto = "HEAD~2".into();
    assert!(plan.validate().is_err());
}

#[test]
fn test_index_patch_preserves_rename_paths() {
    let fixture = Fixture::new();
    fixture.git_ok(&["mv", "file.txt", "renamed.txt"]);
    let patch = fixture.patch(PatchRequest::Directory("".into()));
    let file = patch
        .files
        .iter()
        .find(|f| f.source == PatchSource::Index)
        .unwrap();
    assert_eq!(file.change, PatchChange::Renamed);
    assert_eq!(file.old.path.as_deref(), Some(Path::new("file.txt")));
    assert_eq!(file.new.path.as_deref(), Some(Path::new("renamed.txt")));
}

#[test]
fn test_external_index_changes_are_reloaded_before_partial_staging() {
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    fixture.git_ok(&["add", "file.txt"]);
    assert!(fixture.repo.stage_hunks(&patch, 0, &[0]).is_err());
    assert!(fixture.worktree().files.is_empty());
}

#[test]
fn test_patch_with_only_file_mode_change_is_not_partially_stageable() {
    let fixture = Fixture::new();
    fixture.git_ok(&["update-index", "--chmod=+x", "file.txt"]);
    let patch = fixture.patch(PatchRequest::Index("file.txt".into()));
    assert_eq!(patch.files[0].old.mode, 0o100644);
    assert_eq!(patch.files[0].new.mode, 0o100755);
    assert!(patch.files[0].hunks.is_empty());
    assert!(patch
        .display_lines()
        .iter()
        .any(|l| l.content.contains("mode 100644 -> 100755")));
    assert!(patch.validate_hunks(0, &[0]).is_err());
}

fn app(fixture: &Fixture) -> crate::app::App {
    crate::app::App::from_repo(Box::new(Git2Repository::open(fixture.dir.path()).unwrap())).unwrap()
}

#[test]
fn test_structured_patch_reaches_state_cache_and_owned_snapshot() {
    use crate::flux::snapshot::{AppStateSnapshot, AppStateSnapshotOwned};
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let mut app = app(&fixture);
    app.reload_diff_now();
    crate::flux::test_runtime::finish_background(&mut app);
    let patch = app.state.diff.document().patch.clone().unwrap();
    assert_eq!(patch.files[0].hunks.len(), 2);
    assert_eq!(
        app.state.diff.document().lines.len(),
        patch.display_lines().len()
    );
    let owned = AppStateSnapshotOwned::from_app(&app.state);
    let borrowed = AppStateSnapshot::from_app(&app.state);
    assert!(std::sync::Arc::ptr_eq(
        owned.current_patch.as_ref().unwrap(),
        &patch
    ));
    assert!(std::ptr::eq(
        borrowed.current_patch.unwrap(),
        patch.as_ref()
    ));
    assert_eq!(
        owned.as_snapshot().operation.observed,
        borrowed.operation.observed
    );
    app.request_refresh(crate::app::RefreshKind::StatusOnly);
    assert!(app.state.diff.document().patch.is_none());
    let retained = AppStateSnapshotOwned::from_app(&app.state);
    assert!(retained.current_patch.is_none());
    assert!(std::sync::Arc::ptr_eq(
        retained.retained_diff_patch.as_ref().unwrap(),
        &patch
    ));
    assert!(std::ptr::eq(
        retained.as_snapshot().retained_diff_patch.unwrap(),
        patch.as_ref()
    ));
}

#[test]
fn test_partial_stage_effect_updates_state_only_through_completion_events() {
    use crate::app::Command as AppCommand;
    use crate::flux::{
        action::{Action, DomainAction},
        dispatcher::Dispatcher,
        effects,
    };
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let mut app = app(&fixture);
    let patch = std::sync::Arc::new(fixture.worktree());
    let mut dispatcher = Dispatcher::with_default_stores();
    let envelope = dispatcher.next_envelope(Action::Domain(DomainAction::StageHunks {
        patch,
        file: 0,
        hunks: vec![0],
    }));
    let commands = dispatcher.dispatch(&mut app.state, envelope).commands;
    let effect = commands
        .into_iter()
        .find_map(|c| match c {
            AppCommand::Effect(e) => Some(e),
            _ => None,
        })
        .unwrap();
    let events = effects::execute(effect, &*app.runtime.repo);
    assert!(fixture
        .git_ok(&["show", ":file.txt"])
        .contains("first change"));
    assert!(app.state.files.status().staged.is_empty());
    crate::flux::test_runtime::drive_actions(&mut app, events);
    crate::flux::test_runtime::drive_actions(
        &mut app,
        [Action::Domain(DomainAction::Runtime(
            crate::flux::action::RuntimeEvent::FlushPendingRefresh { log_success: false },
        ))],
    );
    assert_eq!(app.state.files.status().staged.len(), 1);
    assert!(app
        .state
        .command_log
        .iter()
        .any(|entry| entry.command == "Selected hunks staged" && entry.success));
}

#[test]
fn test_invalid_partial_stage_action_emits_no_write_effect() {
    use crate::flux::{action::DomainAction, effects::EffectRequest};
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let mut app = app(&fixture);
    app.apply_repository_action(&DomainAction::StageHunks {
        patch: std::sync::Arc::new(fixture.worktree()),
        file: 0,
        hunks: vec![99],
    });
    assert!(!app
        .take_effects()
        .iter()
        .any(|e| matches!(e, EffectRequest::StageHunks { .. })));
    assert_eq!(fixture.git_ok(&["show", ":file.txt"]), Fixture::base());
}

#[test]
fn test_repository_control_roundtrip_uses_dispatcher_and_effect_events() {
    use crate::flux::action::{Action, DomainAction};
    let fixture = Fixture::new();
    fixture.conflict(true);
    let mut app = app(&fixture);
    assert!(matches!(
        app.state.git.operation.observed.kind,
        Some(OperationKind::Rebase { .. })
    ));
    assert!(!app.state.git.operation.observed.conflicts.is_empty());
    crate::flux::test_runtime::drive_actions(
        &mut app,
        [Action::Domain(DomainAction::ControlRepositoryOperation(
            OperationControl::Abort,
        ))],
    );
    assert_eq!(
        app.state.git.operation.command,
        OperationCommandState::Ready
    );
    assert_eq!(
        app.state.git.operation.observed,
        RepositoryOperation::default()
    );
    assert_eq!(
        fixture.repo.operation_snapshot().unwrap(),
        RepositoryOperation::default()
    );
}

#[test]
fn test_failed_control_rereads_external_state_and_rejects_stale_completion() {
    use crate::flux::action::DomainAction;
    use crate::flux::effects::{self, EffectRequest};
    let fixture = Fixture::new();
    fixture.conflict(true);
    let mut app = app(&fixture);
    app.apply_repository_action(&DomainAction::ControlRepositoryOperation(
        OperationControl::Abort,
    ));
    let request = app
        .take_effects()
        .into_iter()
        .find(|e| matches!(e, EffectRequest::ControlRepositoryOperation { .. }))
        .unwrap();
    // A different Git client has already ended the workflow before our command starts.
    fixture.git_ok(&["rebase", "--abort"]);
    let events = effects::execute(request, &*app.runtime.repo);
    crate::flux::test_runtime::drive_actions(&mut app, events);
    assert!(matches!(
        app.state.git.operation.command,
        OperationCommandState::Failed { .. }
    ));
    assert_eq!(
        app.state.git.operation.observed,
        RepositoryOperation::default()
    );
    let epoch = app.state.git.operation.epoch;
    app.apply_repository_action(&DomainAction::RepositoryOperationFinished {
        id: epoch - 1,
        result: Ok(()),
        observed: Some(RepositoryOperation {
            kind: Some(OperationKind::Merge),
            ..Default::default()
        }),
    });
    assert_eq!(app.state.git.operation.epoch, epoch);
    assert!(app.state.git.operation.observed.kind.is_none());
}

#[test]
fn test_interactive_rebase_is_identified_in_operation_snapshot() {
    let fixture = Fixture::new();
    fixture.conflict(true);
    fixture
        .repo
        .control_operation(OperationControl::Abort)
        .unwrap();
    assert!(!fixture.git(&["rebase", "-i", "other"]).status.success());
    let observed = fixture.repo.operation_snapshot().unwrap();
    assert_eq!(
        observed.kind,
        Some(OperationKind::Rebase { interactive: true })
    );
    assert_eq!(observed.phase(), RepositoryPhase::Conflicted);
    assert_eq!(
        observed.progress,
        Some(OperationProgress {
            current: 1,
            total: 1
        })
    );
}

#[test]
fn test_old_status_completion_does_not_restore_aborted_operation() {
    use crate::flux::action::RuntimeEvent;
    use crate::flux::action::{Action, DomainAction};
    use crate::flux::effects::{self, EffectRequest};
    let fixture = Fixture::new();
    fixture.conflict(true);
    let mut app = app(&fixture);
    app.request_refresh(crate::app::RefreshKind::StatusOnly);
    app.apply_runtime_event(&RuntimeEvent::FlushPendingRefresh { log_success: false });
    let request = app
        .take_effects()
        .into_iter()
        .find(|e| {
            matches!(
                e,
                EffectRequest::LoadBackground {
                    request: crate::background::TaskRequest {
                        load: crate::app::BackgroundLoad::Status { .. },
                        ..
                    }
                }
            )
        })
        .unwrap();
    let old_events = effects::execute(request, &*app.runtime.repo);
    crate::flux::test_runtime::drive_actions(
        &mut app,
        [Action::Domain(DomainAction::ControlRepositoryOperation(
            OperationControl::Abort,
        ))],
    );
    crate::flux::test_runtime::drive_actions(&mut app, old_events);
    assert_eq!(
        app.state.git.operation.observed.phase(),
        RepositoryPhase::Idle
    );
    assert_eq!(
        app.state.git.operation.command,
        OperationCommandState::Ready
    );
}

#[test]
fn test_patch_document_consumer_only_requires_patch_capability() {
    struct PatchOnly(Patch);
    impl PatchRepository for PatchOnly {
        fn patch(&self, _: PatchRequest) -> Result<Option<Patch>, GitError> {
            Ok(Some(self.0.clone()))
        }
        fn diff_unstaged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            panic!("must use structured patch")
        }
        fn diff_staged(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            panic!("must use structured patch")
        }
        fn diff_untracked(&self, _: &Path) -> Result<Vec<DiffLine>, GitError> {
            panic!("must use structured patch")
        }
        fn commit_diff_scoped(&self, _: &str, _: Option<&Path>) -> Result<Vec<DiffLine>, GitError> {
            panic!("must use structured patch")
        }
    }
    let fixture = Fixture::new();
    fixture.write("file.txt", Fixture::changed());
    let patch = fixture.worktree();
    let document = document(&PatchOnly(patch.clone()), patch.request.clone())
        .unwrap()
        .unwrap();
    assert_eq!(*document.patch.unwrap(), patch);
    assert!(!document.lines.is_empty());
}
