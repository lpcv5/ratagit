# EXEC_PLAN.md

## Goal

Refine left-nav workspace layout:

- left: Files, Branches, Commits, Stash
- right: Details, Log
- no top branch/focus/status summary
- bottom: Git operation shortcuts for current focused panel only

## Vertical Slice

1. Core state/action update
- keep existing `FocusNext` / `FocusPrev` state transitions

2. UI render projection
- remove top branch/focus/status summary from text and ratatui renderers
- render bottom shortcut bar from current focus with Git operations only

3. App input mapping
- map panel cycle to `h` / `l`
- remove `Tab` / `Shift+Tab` panel navigation
- keep existing Git action keys

4. Tests
- unit test for input mapping
- snapshot tests for hidden top status and Git-operation-only shortcuts
- harness scenario for current focused-panel Git shortcut visibility

5. Docs and quality gates
- update `PRODUCT.md` and `DESIGN.md`
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
