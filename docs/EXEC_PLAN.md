# EXEC_PLAN.md

## Current Slice

Files tree expand/collapse hot-path optimization.

## Goal

- improve Files panel folder expand/collapse responsiveness in large-repo fast
  mode
- keep tree rendering pure and derived only from `AppState`
- avoid rescanning every file path when toggling one visible directory in the
  lightweight tree projection
- measure the actual toggle + render path, not only backend diff commands
- continue reusing the existing synthetic large repository; do not regenerate it

## Vertical Slice

1. Core tree projection
- add an `AppState.files` lightweight tree index for large-repo fast mode
- build visible Files rows from cached child relationships and file statuses
- keep directory pathspec resolution deterministic without precomputing all
  descendants
- clear or rebuild the cache when file status items change

2. UI rendering
- preserve existing Files rendering output and snapshots
- keep normal rendering on borrowed `tree_rows`

3. Performance suite
- add `files-tree-toggle` to model repeated Files folder expand/collapse and
  terminal rendering after loading large-repo status

4. Tests
- add focused core coverage for the cached lightweight projection
- update/add harness coverage for large-repo folder toggling
- keep existing core, UI snapshot, and harness coverage passing

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
- run `cargo run --release --bin perf-suite -- --scales large --operations files-tree-toggle --iterations 5 --warmup 1`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
- `cargo run --release --bin perf-suite -- --scales large --operations files-tree-toggle --iterations 5 --warmup 1`

Latest performance report:
`tmp/perf/results/perf-1777286616-423078100.md`

Large median results:

- `files-tree-toggle`: backend 239 ms, parsed 245 ms, ratio 0.98

Previous performance report:
`tmp/perf/results/perf-1777285628-377399100.md`

Previous large median results:

- `commit-files-navigation`: backend 137 ms, parsed 135 ms, ratio 1.01
- `commit-files-directory-diff`: backend 581 ms, parsed 582 ms, ratio 1.00
