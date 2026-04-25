# DESIGN.md

## MVP Design

ratagit MVP uses a left-nav workspace interface with six visible panels:

1. Left column (top -> bottom): Files, Branches, Commits, Stash
2. Right column (top -> bottom): Details, Log
3. Bottom row: current-focused-panel Git operation shortcuts only

The focused panel is highlighted. Left list panels keep deterministic selected row indexes. Right panels are read-only views derived from `AppState`. The app does not render a top branch/focus/status summary.

Focus model:

- default focus starts at `Files`
- `h` / `l` map to `FocusPrev` / `FocusNext` and cycle only left panels
- `FocusPanel` supports direct focus selection (`1..6` in app input map)
- `AppState.last_left_focus` tracks the last active left panel for `Details` projection

---

## Interaction Model

- Input is mapped to explicit `UiAction`.
- `update()` applies state transitions and emits `Command`.
- Command execution is delegated to `GitBackend`.
- Backend output re-enters `update()` as `GitResult`.
- UI rendering reads only `AppState`.

---

## Error Presentation

- Git failures never crash the app.
- Errors are stored in `AppState.status.last_error`.
- The `Log` panel displays the latest error.

---

## Snapshot and Harness Design

- UI tests render fixed terminal sizes.
- Harness scenarios drive action sequences and assert both:
  - rendered UI text
  - backend operation trace
- On failure, harness writes artifacts:
  - buffer
  - AppState dump
  - git operation trace
  - input sequence
