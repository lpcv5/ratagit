# EXEC_PLAN.md

## Current Slice

Commit Files navigation hot-path optimization.

## Goal

- improve `j` / `k` responsiveness in the Commit Files subpanel for commits
  with many changed files
- avoid cloning or recomputing full tree projections during ordinary movement
  and rendering
- measure the actual movement + render path, not only backend diff commands
- continue reusing the existing synthetic large repository; do not regenerate it

## Vertical Slice

1. Core navigation
- use cached Commit Files `tree_rows.len()` for cursor movement
- read the selected Commit Files row from cached `tree_rows` instead of cloning
  the whole visible tree
- keep an item path index for Commit Files so selected file details do not scan
  every changed file on each move
- preserve directory pathspec behavior for folder diffs and rename old-path
  behavior for file diffs

2. UI rendering
- render Files and Commit Files from borrowed tree-row slices in the normal
  non-search path
- only clone tree rows when search highlighting needs per-row `matched` flags
- avoid cloning search matches into a `BTreeSet` during every render
- avoid counting every Details diff line when the Details scroll offset is zero

3. Performance suite
- add `commit-files-navigation` to model Commit Files subpanel movement and
  terminal rendering after loading changed files
- keep `commit-files-directory-diff` for the backend directory diff path

4. Tests
- keep existing core and UI snapshot coverage passing
- run the new perf operation against the existing large synthetic repo

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
- run `cargo run --release --bin perf-suite -- --scales large --operations commit-files-navigation,commit-files-directory-diff --iterations 5 --warmup 1`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
- `cargo run --release --bin perf-suite -- --scales large --operations commit-files-navigation,commit-files-directory-diff --iterations 5 --warmup 1`

Latest performance report:
`tmp/perf/results/perf-1777285628-377399100.md`

Large median results:

- `commit-files-navigation`: backend 137 ms, parsed 135 ms, ratio 1.01
- `commit-files-directory-diff`: backend 581 ms, parsed 582 ms, ratio 1.00
