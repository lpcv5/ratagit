//! Exercise reducers without constructing AppState, a repository or a runtime.
//! The small state types also keep capability requirements checked by Rust.

use super::state_access::{FilesAccess, InvalidationAccess, LogAccess, RefreshAccess, StashAccess};
use super::{DiffStore, DirtyHint, FilesStore, StashStore, Store, UiInvalidation};
use crate::app::files::{FilesAction, FilesState, FilesUpdate};
use crate::app::{Command, RefreshKind};
use crate::flux::action::RuntimeEvent;
use crate::flux::action::{Action, DomainAction, SystemAction};
use crate::flux::command_output::CommandOutput;
use crate::flux::effects::EffectRequest;
use std::time::Duration;

fn reduce<S: ?Sized>(
    store: &mut impl Store<S>,
    state: &mut S,
    action: Action,
) -> super::test_support::TestReduceOutput {
    super::test_support::reduce_state(store, state, action)
}

#[derive(Default)]
struct LogState(Vec<(String, bool)>);

impl LogAccess for LogState {
    fn push_log(&mut self, message: String, success: bool) {
        self.0.push((message, success));
    }
}

#[derive(Default)]
struct FileState {
    feature: FilesState,
    log: LogState,
}

impl FilesAccess for FileState {
    fn reduce_files(&mut self, action: FilesAction<'_>) -> FilesUpdate {
        self.feature.update(action, true)
    }
}

impl LogAccess for FileState {
    fn push_log(&mut self, message: String, success: bool) {
        self.log.push_log(message, success);
    }
}

#[test]
fn test_file_request_and_completion_with_isolated_feature_state() {
    let mut state = FileState::default();
    let mut store = FilesStore::new();
    let path = std::path::PathBuf::from("selected.txt");
    let output = reduce(
        &mut store,
        &mut state,
        Action::Domain(DomainAction::StageFile(path.clone())),
    );
    assert!(matches!(
        output.commands.as_slice(),
        [Command::Effect(EffectRequest::StageFile(target))] if target == &path
    ));
    assert!(state.log.0.is_empty());

    let output = reduce(
        &mut store,
        &mut state,
        Action::Domain(DomainAction::StageFileFinished {
            path,
            result: Ok(()),
        }),
    );
    assert_eq!(state.log.0, vec![("staged selected.txt".to_string(), true)]);
    assert!(output.commands.is_empty());
    assert_eq!(output.invalidation, UiInvalidation::all());
}

#[test]
fn test_file_failure_logs_without_emitting_a_retry() {
    let mut state = FileState::default();
    let output = reduce(
        &mut FilesStore::new(),
        &mut state,
        Action::Domain(DomainAction::UnstageFileFinished {
            path: "selected.txt".into(),
            result: Err("index locked".into()),
        }),
    );
    assert_eq!(
        state.log.0,
        vec![(
            "unstage failed selected.txt: index locked".to_string(),
            false
        )]
    );
    assert!(output.commands.is_empty());
    // LogAccess implementations are responsible for invalidating the log.
    assert_eq!(output.invalidation, UiInvalidation::none());
}

struct StashState {
    selected: Option<usize>,
    log: LogState,
}

impl StashAccess for StashState {
    fn selected_stash_index(&self) -> Option<usize> {
        self.selected
    }
}

impl LogAccess for StashState {
    fn push_log(&mut self, message: String, success: bool) {
        self.log.push_log(message, success);
    }
}

#[test]
fn test_stash_commands_preserve_selected_repository_index() {
    let mut state = StashState {
        selected: Some(7),
        log: LogState::default(),
    };
    let mut store = StashStore::new();
    for action in [
        DomainAction::StashApplySelected,
        DomainAction::StashPopSelected,
        DomainAction::StashDropSelected,
    ] {
        let output = reduce(&mut store, &mut state, Action::Domain(action.clone()));
        match (action, output.commands.as_slice()) {
            (DomainAction::StashApplySelected, [Command::Effect(EffectRequest::StashApply(7))])
            | (DomainAction::StashPopSelected, [Command::Effect(EffectRequest::StashPop(7))])
            | (DomainAction::StashDropSelected, [Command::Effect(EffectRequest::StashDrop(7))]) => {
            }
            _ => panic!("stash action must preserve both operation and selected index"),
        }
    }
    assert!(state.log.0.is_empty());
}

#[test]
fn test_stash_without_selection_emits_no_write() {
    let mut state = StashState {
        selected: None,
        log: LogState::default(),
    };
    let output = reduce(
        &mut StashStore::new(),
        &mut state,
        Action::Domain(DomainAction::StashDropSelected),
    );
    assert!(output.commands.is_empty());
    assert_eq!(state.log.0, vec![("no stash selected".to_string(), false)]);
}

struct RefreshState {
    pending: bool,
    elapsed: Duration,
}

impl RefreshAccess for RefreshState {
    fn request_refresh(&mut self, _: RefreshKind, _output: &mut CommandOutput) {
        panic!("the debounce reducer must not request a repository refresh");
    }

    fn has_pending_diff_reload(&self) -> bool {
        self.pending
    }

    fn diff_reload_debounce_elapsed(&self, debounce: Duration) -> bool {
        self.elapsed >= debounce
    }

    fn schedule_diff_reload(&mut self, _output: &mut CommandOutput) {
        panic!("the debounce reducer must not restart the debounce timer");
    }
}

#[test]
fn test_diff_debounce_with_only_refresh_state() {
    for (pending, elapsed_ms, should_flush) in [
        (false, 80, false),
        (true, 79, false),
        (true, 80, true),
        (true, 81, true),
    ] {
        let mut state = RefreshState {
            pending,
            elapsed: Duration::from_millis(elapsed_ms),
        };
        let output = reduce(
            &mut DiffStore::new(),
            &mut state,
            Action::System(SystemAction::Tick),
        );
        if should_flush {
            assert!(matches!(
                output.commands.as_slice(),
                [Command::Sync(crate::flux::action::DomainAction::Runtime(
                    RuntimeEvent::FlushPendingDiffReload
                ))]
            ));
        } else {
            assert!(output.commands.is_empty());
        }
    }
}

#[derive(Default)]
struct RenderState {
    full_redraws: usize,
    regions: DirtyHint,
}

impl InvalidationAccess for RenderState {
    fn mark_all_dirty(&mut self) {
        self.full_redraws += 1;
    }

    fn mark_dirty(&mut self, hint: DirtyHint) {
        self.regions.0 |= hint.0;
    }
}

#[test]
fn test_invalidation_merges_regions_without_other_state_capabilities() {
    let mut state = RenderState::default();
    UiInvalidation::none().apply(&mut state);
    assert_eq!(state.regions, DirtyHint::default());
    assert_eq!(state.full_redraws, 0);

    let mut invalidation = UiInvalidation::diff();
    invalidation.merge(UiInvalidation::log_and_overlay());
    invalidation.apply(&mut state);
    assert_eq!(
        state.regions,
        DirtyHint(DirtyHint::DIFF | DirtyHint::COMMAND_LOG | DirtyHint::OVERLAY)
    );
    assert_eq!(state.full_redraws, 0);

    UiInvalidation::all().apply(&mut state);
    assert_eq!(state.full_redraws, 1);
}
