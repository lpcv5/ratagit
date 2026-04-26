# EXEC_PLAN.md

## Current Slice

Remove large-repository startup blocking by running Git commands asynchronously
and caching deterministic file-tree projections in AppState.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- render the first TUI frame before repository refresh completes
- keep rendering pure and derived only from `AppState`
- make pending refresh/details/operation state visible in the UI
- avoid rebuilding file tree rows on every render frame
- avoid repeated status scans during details diff refresh

## Vertical Slice

1. Runtime behavior
- add an async runtime with a single Git worker thread and channel-delivered results
- keep the existing synchronous runtime for deterministic harness scenarios
- use the async runtime from the real TUI entrypoint

2. Backend and render optimization
- cache visible file-tree rows and row descendants in `FilesPanelState`
- reuse the latest status file list for untracked details diff generation
- avoid git2 `Sort::TIME` for the recent-commits revwalk because it is slow on
  very large histories; keep head-first recent commit ordering
- cap untracked diff file count and bytes for directory selections
- add tracing events around hybrid backend refresh and diff steps

3. Validation
- add pending-state reducer tests and stale details-result protection
- add loading UI snapshots and an async harness test that blocks refresh
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
