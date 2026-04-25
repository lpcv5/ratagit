## Strategy

ratagit uses a layered testing approach:

1. Unit tests (logic)
2. Integration tests (Git)
3. UI snapshot tests
4. Harness scenario tests

---

## UI Testing

- Use ratatui TestBackend
- Use insta snapshots
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
- assert UI
- assert Git state

---

## Failure Artifacts

On failure, store:

- buffer snapshot
- AppState dump
- logs
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
