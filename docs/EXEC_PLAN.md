# EXEC_PLAN.md

## Current Slice

Tune panel sizing and titles for the six-panel workspace while preserving pure
rendering and existing keyboard behavior.

## Goal

- remove `<empty>` / `<none>` placeholders from all panels
- collapse unfocused Stash panel to one content row
- dynamically expand focused Files/Branches/Commits panel when content overflows
- prefix all panel titles with `[1]..[6]` numeric focus hints

## Vertical Slice

1. Shared layout calculator
- add a pure left-panel height calculator used by both `terminal.rs` and
  compatibility `text.rs`
- keep deterministic ratio baseline and avoid layout drift across render paths
- apply Stash collapse and focused-panel expansion rules in one place

2. Panel rendering behavior
- remove empty placeholders from list, details, and log projections
- keep existing content rows, selection styles, and key hints
- apply numbered titles consistently in both render paths

3. Tests and harness
- add unit tests for layout collapse/expand and deterministic borrowing
- add unit tests that ensure empty placeholder text is absent
- update terminal snapshots and add a long-list expansion snapshot
- extend harness expectations with `screen_not_contains` and add a scenario that
  verifies numbered titles plus no empty placeholders

4. Quality gates
- run `cargo fmt`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
- run `cargo test -p ratagit-ui`
- run `cargo test -p ratagit-harness`
