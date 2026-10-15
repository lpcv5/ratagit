//! Reproducible, opt-in snapshot allocation measurements, isolated to the test thread.
use super::AppStateSnapshotOwned;
use crate::app::{App, FilesState, SidePanel};
use crate::flux::stores::test_support::MockRepo;
use crate::git::{
    BranchInfo, CommitInfo, CommitSyncState, DiffLine, DiffLineKind, FileEntry, FileStatus,
    GitStatus, GraphCell, StashInfo,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::time::Instant;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct Allocations {
    pub calls: usize,
    pub bytes: usize,
}

thread_local! {
    static COUNTER: Cell<Option<Allocations>> = const { Cell::new(None) };
}

struct MeasuredAllocator;

fn record(bytes: usize) {
    // TLS has a const initializer, so recording allocations never allocates itself.
    let _ = COUNTER.try_with(|counter| {
        if let Some(mut counts) = counter.get() {
            counts.calls += 1;
            counts.bytes += bytes;
            counter.set(Some(counts));
        }
    });
}

// SAFETY: All allocation and deallocation requests are forwarded unchanged to System.
unsafe impl GlobalAlloc for MeasuredAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc(layout);
        if !pointer.is_null() {
            record(layout.size());
        }
        pointer
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let pointer = System.alloc_zeroed(layout);
        if !pointer.is_null() {
            record(layout.size());
        }
        pointer
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let pointer = System.realloc(pointer, layout, size);
        if !pointer.is_null() {
            record(size);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        System.dealloc(pointer, layout);
    }
}

#[global_allocator]
static ALLOCATOR: MeasuredAllocator = MeasuredAllocator;

struct MeasurementGuard;
impl Drop for MeasurementGuard {
    fn drop(&mut self) {
        COUNTER.with(|counter| counter.set(None));
    }
}

pub(crate) fn measure<T>(work: impl FnOnce() -> T) -> (T, Allocations) {
    COUNTER.with(|counter| {
        assert!(counter.get().is_none(), "measurements must not nest");
        counter.set(Some(Allocations::default()));
    });
    let guard = MeasurementGuard;
    let result = work();
    let counts = COUNTER.with(|counter| counter.get().unwrap());
    drop(guard);
    (result, counts)
}

pub(crate) fn fixture(large: bool) -> App {
    let mut app = App::from_repo(Box::new(MockRepo)).unwrap();
    let (commits, files, diff_lines, branches, stashes) = if large {
        (10_000, 5_000, 5_000, 500, 100)
    } else {
        (30, 20, 100, 10, 5)
    };
    app.files = FilesState::new(
        GitStatus {
            unstaged: (0..files)
                .map(|index| FileEntry {
                    path: format!("src/module_{index:05}.rs").into(),
                    status: FileStatus::Modified,
                })
                .collect(),
            ..Default::default()
        },
        true,
    );
    app.revisions.commits.items = (0..commits)
        .map(|index| {
            let oid = format!("{index:040x}");
            CommitInfo {
                oid: oid.clone(),
                message: format!("Change {index}: {}", "message ".repeat(12)),
                author: "Snapshot benchmark author".into(),
                time: "2026-10-15 10:00".into(),
                parent_count: 1,
                parent_oids: vec![format!("{:040x}", index + 1)],
                sync_state: CommitSyncState::DefaultBranch,
                graph: (0..8)
                    .map(|lane| GraphCell {
                        text: "│ ".into(),
                        lane,
                        pipe_oid: Some(oid.clone()),
                        pipe_oids: vec![oid.clone()],
                    })
                    .collect(),
            }
        })
        .collect();
    app.revisions.branches.items = (0..branches)
        .map(|index| BranchInfo {
            name: format!("feature/branch-{index}"),
            is_current: index == 0,
        })
        .collect();
    app.revisions.branches.commits_subview.items = app
        .revisions
        .commits
        .items
        .iter()
        .take(commits / 5)
        .cloned()
        .collect();
    app.revisions.stash.items = (0..stashes)
        .map(|index| StashInfo {
            index,
            message: format!("WIP stash {index}"),
        })
        .collect();
    app.diff.set_test_document(
        (0..diff_lines)
            .map(|index| DiffLine {
                kind: DiffLineKind::Added,
                content: format!("{index:05}: {}", "line ".repeat(50)),
            })
            .collect::<Vec<_>>()
            .into(),
    );
    for index in 0..8 {
        app.push_log(format!("staged src/module_{index}.rs"), true);
    }
    app.input.stash_targets = (0..20)
        .map(|index| format!("src/module_{index}.rs").into())
        .collect();
    app.ui.active_panel = SidePanel::Commits;
    app.refresh_render_cache();
    app
}

fn profile(label: &str, repetitions: usize, mut work: impl FnMut()) {
    for _ in 0..3 {
        work();
    }
    let mut samples = Vec::new();
    let mut counts = Allocations::default();
    for _ in 0..5 {
        let start = Instant::now();
        let (_, measured) = measure(|| {
            for _ in 0..repetitions {
                work();
            }
        });
        samples.push(start.elapsed().as_nanos() / repetitions as u128);
        counts = measured;
    }
    samples.sort_unstable();
    println!(
        "{label}: median_ns={} allocations={} allocated_bytes={}",
        samples[2],
        counts.calls / repetitions,
        counts.bytes / repetitions
    );
}

#[test]
#[ignore = "opt-in allocation/timing profile; run with --ignored --nocapture --test-threads=1"]
fn test_snapshot_profile() {
    for large in [false, true] {
        let app = fixture(large);
        let size = if large { "large" } else { "small" };
        println!(
            "workload={size} commits={} subview_commits={} files={} diff_lines={}",
            app.revisions.commits.items.len(),
            app.revisions.branches.commits_subview.items.len(),
            app.files.view().tree_nodes.len(),
            app.diff.document().lines.len()
        );
        profile(&format!("{size}/from_app"), 20, || {
            black_box(AppStateSnapshotOwned::from_app(black_box(&app.state)));
        });
        let snapshot = AppStateSnapshotOwned::from_app(&app.state);
        profile(&format!("{size}/as_snapshot"), 200, || {
            black_box(black_box(&snapshot).as_snapshot());
        });
        profile(&format!("{size}/clone_files"), 20, || {
            black_box(app.files.view().clone());
        });
        profile(&format!("{size}/clone_branches"), 20, || {
            black_box(app.revisions.view().branches.clone());
        });
        profile(&format!("{size}/clone_commits"), 20, || {
            black_box(app.revisions.view().commits.clone());
        });
        profile(&format!("{size}/clone_stash"), 20, || {
            black_box(app.revisions.view().stash.clone());
        });
        profile(&format!("{size}/clone_diff_lines"), 20, || {
            black_box(app.diff.document().lines.clone());
        });
        profile(&format!("{size}/clone_keymap"), 20, || {
            black_box(snapshot.keymap.clone());
        });
    }
}
