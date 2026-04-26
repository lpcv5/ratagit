# EXEC_PLAN.md

## Current Slice

Keep Commits scroll position continuous when an incremental commit page finishes
loading.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- keep rendering pure and derived only from `AppState`
- treat full refresh and incremental append as different reconciliation paths
- preserve Commits scroll direction/origin when appending a page
- keep boundary behavior that advances to the first new commit if the user was
  waiting at the loaded tail
- prevent the rendered window from jumping to the top-reserve position after a
  page arrives

## Vertical Slice

1. State and input
- add an append-specific Commits reconciliation helper
- use it only for `GitResult::CommitsPage`

2. Rendering
- add a UI unit test that renders the list before and after page append
- assert the first visible row moves continuously by one row rather than jumping
  to the top-reserve layout

3. Validation
- keep existing prefetch/lazy-load/harness coverage passing
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
