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

- Panel unit tests assert each panel's pure projection from `AppContext`.
- Panel projection tests assert shared panel descriptors and span-backed
  `PanelLine` rows. Plain text compatibility tests must derive text from the same
  spans used by terminal rendering.
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
- For manual performance validation, generate synthetic repositories under
  `tmp/perf/` with `cargo run --bin make-large-repo -- --scale large --path
  tmp/perf/large-repo --force`.
- The manual performance suite is run separately with
  `cargo run --bin perf-suite -- --regenerate`; it records Git CLI raw, Git CLI
  parsed, and backend timings and writes reports under `tmp/perf/results/`.
- Normal `cargo test` only runs smoke-sized coverage for the performance tools
  and must not create large or huge repositories.

---

## Real Git Smoke Harness

`ratagit-harness` includes a small real-Git smoke layer that drives the
synchronous harness runtime with `HybridGitBackend`. These tests verify the path
from `UiAction` to `Command`, real backend mutation, refreshed `AppContext`, and
final repository state.

- Keep real-Git smoke tests small and focused on high-risk drift between the
  mock backend and `HybridGitBackend`.
- Use isolated repositories under `tmp/git-tests/` and clean them up with RAII.
- Skip gracefully when `git` is unavailable.
- Assert both harness state and real Git state.
- Do not include these tests in `docs/harness/SCENARIOS.md`; that manifest
  indexes mock `fn harness_*` scenarios only.

---

## Core And Runtime Testing

- Command metadata tests must cover every `Command` variant for log labels,
  mutating classification, pending labels, debounce keys, and refresh coalescing
  keys.
- Work-state tests should target the typed refresh, details, mutation,
  pagination, and Commit Files substates instead of asserting incidental bool
  combinations.
- Details request tests must cover stale request ids and target mismatches for
  Files, Branches, Commits, and Commit Files.
- Scheduler tests use injected `Instant` values and must not sleep. They cover
  debounce, latest-details wins, refresh coalescing, and preservation of mutation
  boundaries.
- Async runtime and harness tests must use explicit wait helpers or channel
  barriers instead of bare sleeps. Wait helpers must include the waited label,
  current `AppContext`, and rendered screen text in timeout failures.
- Git backend capability tests cover read, write, history-rewrite, shared mock,
  boxed dispatch, and root `GitBackend` compatibility.
- Split-refresh tests must verify that full refresh preserves `FilesSnapshot`
  metadata, including large-repo mode, truncation, and skipped-scan flags.

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

Every user-visible behavior change needs a harness scenario. Internal-only
architecture changes can be covered by unit/integration tests when rendered UI
and Git state are intentionally unchanged.

Harness infrastructure changes, such as artifact/reporting improvements or
scenario manifest tooling, should use focused harness/unit tests. They do not
need broad product-flow scenarios unless rendered behavior, user input handling,
or Git state changes.

The scenario index is stored in `docs/harness/SCENARIOS.md`. It is generated
from `libs/ratagit-harness/tests/harness.rs` by scanning `fn harness_*`
definitions and is checked by tests. Update it whenever harness scenario names
are added, removed, or renamed.

---

## Failure Artifacts

On failure, store:

- buffer snapshot
- real screen snapshot
- AppContext dump
- Git operation trace
- final mock Git state
- input sequence
- structured `failure_report.json` with schema version, expectations, typed
  assertion failures, Git operation lines, debug dumps, and sibling artifact
  filenames

---

## Harness Engineering Rules

- scenarios must be small and focused
- one behavior per scenario
- scenarios must be composable
- fixtures must be reusable
- failure artifacts must stay agent-readable
- scenario manifest changes must be reviewed with harness changes

---

## Anti-Patterns

- giant scenarios
- implicit assertions
- relying on timing
- bare sleeps in scheduler, async runtime, or harness tests
