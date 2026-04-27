# EXEC_PLAN.md

## Current Slice

Shared file-tree and Commit Files toggle optimization.

## Goal

- improve Files and Commit Files folder expand/collapse responsiveness
- keep tree rendering pure and derived only from `AppState`
- use one shared tree index for Files and Commit Files
- avoid rescanning every file path when toggling one visible directory
- keep Commit Files status letters (`A/M/D/R/C/T`) but color only the status
  marker, leaving file names in the default foreground
- measure the actual toggle + render path, not only backend diff commands
- continue reusing the existing synthetic large repository; do not regenerate it

## Vertical Slice

1. Core tree projection
- replace the Files-only lightweight index with a shared `FileTreeIndex`
- use the shared index from both `FilesPanelState` and `CommitFilesPanelState`
- build visible rows from cached child relationships and row metadata
- keep directory pathspec resolution deterministic without precomputing all
  descendants in Commit Files
- sync item changes through the index by removing, adding, or metadata-updating
  changed source paths

2. UI rendering
- preserve existing tree text output and snapshots
- keep normal rendering on borrowed `tree_rows`
- render tree rows with spans so status icons/letters are colored separately
  from file names

3. Performance suite
- keep `files-tree-toggle`
- add `commit-files-tree-toggle` to model repeated Commit Files folder
  expand/collapse and terminal rendering after loading changed files

4. Tests
- add focused core coverage for shared-index Files and Commit Files projections
- add UI coverage for status-marker-only coloring
- update/add harness coverage for Commit Files folder toggling
- keep existing core, UI snapshot, and harness coverage passing

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
- run `cargo run --release --bin perf-suite -- --scales large --operations files-tree-toggle,commit-files-tree-toggle --iterations 5 --warmup 1`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
- `cargo run --release --bin perf-suite -- --scales large --operations files-tree-toggle,commit-files-tree-toggle --iterations 5 --warmup 1`

Latest performance report:
`tmp/perf/results/perf-1777287858-284120600.md`

Large median results:

- `commit-files-tree-toggle`: backend 172 ms, parsed 173 ms, ratio 0.99
- `files-tree-toggle`: backend 250 ms, parsed 248 ms, ratio 1.01

Previous performance report:
`tmp/perf/results/perf-1777286616-423078100.md`

Previous large median results:

- `files-tree-toggle`: backend 239 ms, parsed 245 ms, ratio 0.98

Earlier performance report:
`tmp/perf/results/perf-1777285628-377399100.md`

Earlier large median results:

- `commit-files-navigation`: backend 137 ms, parsed 135 ms, ratio 1.01
- `commit-files-directory-diff`: backend 581 ms, parsed 582 ms, ratio 1.00
