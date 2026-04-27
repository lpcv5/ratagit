# EXEC_PLAN.md

## Current Slice

Show an animated loading spinner and loading type before the bottom key area
while repository work is pending.

## Goal

- keep rendering pure by passing animation frame through an explicit render context
- derive loading visibility and type from `AppContext.work`
- keep the bottom key area unchanged when no work is pending
- animate the spinner in the real TUI without adding animation frame to business state

## Vertical Slice

1. Render context
- add a render context carrying the current spinner frame
- preserve deterministic default render APIs for tests and harnesses

2. Loading indicator
- add a bottom-bar loading indicator component
- show spinner and loading type before shortcut keys when work is pending
- derive type from operations, refreshes, details, Commit Files, and commit pagination

3. Tests and harness
- add unit coverage for loading type priority and spinner frame selection
- add UI snapshot/render coverage for the bottom loading prefix
- update harness coverage for async refresh loading state

4. Documentation
- update `docs/PRODUCT.md` because behavior changes

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo check -p ratagit-core`
- `cargo check --workspace`
- `cargo test --workspace --no-run`
- `cargo test -p ratagit-harness --test harness`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
