## Strategy

ratagit uses a layered testing approach:

1. Unit tests (logic)
2. Integration tests (Git)
3. UI snapshot tests
4. Harness scenario tests

---

## UI Testing

- Panel unit tests assert each panel's pure projection from `AppState`.
- Full-screen integration tests render `render_terminal` with
  `ratatui::TestBackend`.
- Use insta snapshots for full-screen terminal buffers.
- Cursor/selection color is asserted through `ratatui::TestBackend` buffer cell
  styles because text snapshots intentionally do not encode terminal colors.
- Text snapshots must not rely on visible cursor markers such as `>`.
- Fixed sizes:
  - 80x24
  - 100x30
  - 120x40

---

## Fixtures

All UI tests must use fixtures:

- empty_repo
- dirty_repo
- many_files
- conflict
- unicode_paths

---

## Snapshot Rules

- Snapshots must be deterministic
- No timestamps
- No random content

---

## Harness Testing

Scenarios must:

- simulate user input
- assert real `render_terminal` screen text
- assert Git operation trace
- assert final mock Git state

---

## Failure Artifacts

On failure, store:

- buffer snapshot
- real screen snapshot
- AppState dump
- Git operation trace
- final mock Git state
- input sequence

---

## Harness Engineering Rules

- scenarios must be small and focused
- one behavior per scenario
- scenarios must be composable
- fixtures must be reusable

---

## Anti-Patterns

- giant scenarios
- implicit assertions
- relying on timing
