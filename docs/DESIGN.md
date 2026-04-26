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

Files panel interaction:

- `AppState.files` stores tree expansion, visible-row selection, multi-select rows, and search state.
- File tree rows are derived from `RepoSnapshot.files`; no UI code reads external state.
- Directories are display targets only and resolve to descendant files from the current snapshot.
- `space` toggles stage state for the current target or selected batch.
- `s` stashes the current target or selected batch through path-limited Git commands.
- `v` enters multi-select mode and toggles row membership.
- `/` switches the bottom keys area into search input until Enter or Esc.
- `d` discard is intentionally not mapped to input until the reusable confirmation dialog exists.

---

## Error Presentation

- Git failures never crash the app.
- Errors are stored in `AppState.status.last_error`.
- The `Log` panel displays the latest error.

---

## Snapshot and Harness Design

- UI panel unit tests assert pure panel projections from `AppState`.
- Full-screen UI tests render fixed terminal sizes through `render_terminal`
  and `ratatui::TestBackend`.
- Harness scenarios drive action sequences and assert:
  - real terminal screen text
  - backend operation trace
  - final mock Git state
- On failure, harness writes artifacts:
  - compatibility text buffer
  - real terminal screen
  - AppState dump
  - git operation trace
  - final mock Git state
  - input sequence
