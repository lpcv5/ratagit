# EXEC_PLAN.md

## Goal

Implement left-nav workspace layout:

- left: Files, Branches, Commits, Stash
- right: Details, Log
- bottom: shortcuts for current focused panel only

## Vertical Slice

1. Core state/action update
- add 6-panel `PanelFocus`
- add `UiAction::FocusPanel`
- add `AppState.last_left_focus`
- keep `FocusNext/FocusPrev` cycling on left panels only

2. UI render projection
- switch to left/right workspace rows
- render Details and Log from `AppState`
- render bottom shortcut bar from current focus

3. App input mapping
- support `1..6` panel direct focus
- keep existing action keys

4. Tests
- unit tests for focus transitions and right-panel movement constraints
- snapshot tests for new panel structure and focus-driven shortcuts
- harness scenario for direct focus switching and shortcut visibility

5. Docs and quality gates
- update `PRODUCT.md` and `DESIGN.md`
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
