## Strategy

ratagit uses a layered testing approach:

1. Unit tests (logic)
2. Integration tests (Git)
3. UI snapshot tests
4. Harness scenario tests

---

## Test Code Quality

Test code should make behavior and scenario flow easy to understand. It does not
need to satisfy the same strict clippy gate as production code, but it must:

- compile and pass under `cargo test`
- keep snapshots and harness scenarios deterministic
- isolate real Git mutations under workspace `tmp/`
- fail with useful artifacts or assertions

Clippy feedback on test-only targets is advisory unless it points to broken
test flow, nondeterminism, or an assertion that no longer verifies behavior.

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

## Real Git Backend Tests

- Any test that executes a real Git backend or the real `git` binary must create
  and use an isolated repository under workspace `tmp/`.
- Standard root: `<workspace>/tmp/git-tests/<unique-case>`.
- Tests must clean up their temporary repositories on completion.
- Never run real-git mutation tests directly in the workspace repository.

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
