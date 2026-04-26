# EXEC_PLAN.md

## Current Slice

Add global Details scrolling shortcuts while keeping rendering pure and Details
scroll state owned by `AppState`.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- keep rendering pure and derived only from `AppState`
- map `Ctrl+U` / `Ctrl+D` to explicit global scroll actions carrying a line
  count and visible-line count derived from the current Details content height
- scroll Details content for Files and Branches projections without changing Git
  state or focused-panel selection
- move by `max(1, details_content_height * 2 / 5)` lines per shortcut press
- reset Details scroll when the selected details target or refreshed content
  changes
- clamp the stored offset to the last visible page so repeated downward scrolls
  at the bottom do not create hidden overscroll

## Vertical Slice

1. State and input
- add `DetailsPanelState.scroll_offset`
- add `UiAction::DetailsScrollUp` and `UiAction::DetailsScrollDown`
- map `Ctrl+U` and `Ctrl+D` before mode-specific key handling with a
  terminal-size-derived scroll amount

2. Rendering
- apply `scroll_offset` in files and branches Details projections
- keep placeholders, loading rows, and errors deterministic

3. Validation
- add reducer, key-map, panel projection, UI snapshot, and harness tests
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
