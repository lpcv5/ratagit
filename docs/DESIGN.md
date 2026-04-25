# DESIGN.md

## MVP Design

ratagit MVP uses a simple panel-oriented interface with five visible sections:

1. Status
2. Files
3. Commits
4. Branches
5. Stash

The focused panel is highlighted and all list panels keep a deterministic selected row index.

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
- The top summary line displays the latest error.

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
