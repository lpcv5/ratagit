# EXEC_PLAN.md

## Current Slice

Add a Files-panel discard confirmation modal on `d`.

## Goal

- open a deterministic confirmation modal from Files focus
- target the current file/directory row or visual-selected file targets
- keep confirmation state in `AppState`
- execute discard side effects only through `Command::DiscardFiles` + `GitBackend`
- preserve the repository-level `D` reset menu behavior

## Vertical Slice

1. Core state and reducer
- add discard confirmation state with resolved target paths
- add `UiAction` variants for open, confirm, and cancel
- emit `Command::DiscardFiles` on confirm
- report success/failure through notices, `last_operation`, and refresh follow-up

2. Input mapping + UI modal overlay
- map lowercase `d` in Files focus
- while confirmation modal is active, map Enter to confirm and Esc to cancel
- render a discard modal with selected target count/path summary and warning text
- show discard-specific help in the bottom shortcut row

3. Validation
- add reducer, key mapping, snapshot, and harness tests
- update product/design docs for file-targeted discard behavior
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
