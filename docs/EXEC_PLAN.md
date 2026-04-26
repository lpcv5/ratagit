# EXEC_PLAN.md

## Current Slice

Add a Files-panel repository reset menu on `D`.

## Goal

- open a deterministic reset select list from Files focus
- support mixed, soft, hard, and Nuke choices
- keep reset menu state in `AppState`
- execute reset side effects only through `Command` + `GitBackend`
- keep lowercase `d` available for a future file-targeted discard/reset flow

## Vertical Slice

1. Core state and reducer
- add reset menu state and reset choice/mode types
- add `UiAction` variants for open, move, confirm, and cancel
- emit `Command::Reset` or `Command::Nuke` on confirm
- report success/failure through notices, `last_operation`, and refresh follow-up

2. Input mapping + UI modal overlay
- map uppercase `D` in Files focus
- while reset menu is active, map `j`/`k` and arrow keys to menu movement
- render a reset modal with options and the selected option description
- show reset-specific help in the bottom shortcut row

3. Validation
- add reducer, key mapping, mock backend, CLI backend, snapshot, and harness tests
- update product/design docs for reset and Nuke behavior
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
