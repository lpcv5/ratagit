# EXEC_PLAN.md

## Current Slice

Shortcut A amend staged changes into commits.

### Phase 17: Amend Staged Changes

Status: completed

Goal:

- add `A` as an amend shortcut that folds staged changes into `HEAD` by default
- when the main Commits panel is focused, amend staged changes into the selected
  private commit and replay newer private commits
- preserve unidirectional flow through `UiAction` -> `Command` -> `GitBackend`

Deliverables:

- core action, command, reducer, result handling, and command metadata
- mock backend and real Git backend support through history-rewrite capability
- input mapping and shortcut row copy
- unit tests, UI snapshot updates, mock harness scenario, real-Git smoke coverage
- product documentation update

Validation:

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

Results:

- added `A` amend routing through `UiAction`, `Command`, reducer result
  handling, command metadata, and the history-rewrite backend capability
- implemented mock and Hybrid Git backend support for amending staged changes
  into `HEAD` or a selected private commit
- updated shortcut rendering so global `p`/`P` sync actions remain visible
  before local panel actions
- added core, input, UI snapshot, mock harness, and real-Git smoke coverage
- updated product/design docs and the generated harness scenario manifest

## Goal

- keep the existing unidirectional data flow working while reducing long-term
  maintenance cost
- make Git errors, pending work, Details refreshes, and command metadata explicit
  enough for the compiler and tests to catch invalid states
- harden resource lifecycles around the terminal session and Git child processes
- reduce duplicated render, scheduling, and backend capability logic
- improve large-repository and high-frequency navigation performance based on
  measurable hotspots
- keep every step small, test-covered, and reversible

## Execution Model

Each phase is implemented by a dedicated subagent. The lead agent reviews the
diff, runs validation, and sends issues back to the same phase subagent until the
phase passes.

Every phase must follow this loop:

1. Implement the smallest vertical slice for that phase.
2. Add or update unit tests.
3. Add or update UI snapshots when rendering changes.
4. Add or update harness scenarios for user-visible behavior.
5. Update architecture, design, product, testing, or observability docs when
   behavior or design changes.
6. Run validation before moving to the next phase.

Validation commands:

```text
rtk cargo fmt
rtk cargo clippy --workspace --lib --bins -- -D warnings
rtk cargo test
```

## Phase Tracker

### Phase 0: Baseline And Plan

Status: completed

Deliverables:

- replace this execution plan with the optimization plan
- capture the phase tracker and validation expectations
- confirm no code behavior changes are included in the baseline update

Validation:

- documentation-only review

### Phase 1: Typed Git Error Model

Status: completed

Deliverables:

- introduce `GitErrorKind` while preserving user-facing messages
- classify CLI and git2 errors needed by current behavior
- replace string-based divergent push and unmerged branch checks with typed error
  checks
- keep `GitResult` display behavior stable unless snapshot updates are explicitly
  needed

Tests:

- unit tests for error classification
- reducer/operation tests for divergent push confirmation
- reducer/operation tests for force-delete branch confirmation
- harness scenario if user-visible confirmation behavior changes

Results:

- added shared `GitErrorKind` and `GitFailure` payloads in `ratagit-core`
- changed push and branch-delete command results to carry typed failures
- classified divergent/non-fast-forward push and unmerged branch delete CLI failures
- replaced reducer semantic string checks for force-push and force-delete confirmation
  with `GitErrorKind` checks

### Phase 2: Terminal And Git Process Lifecycle

Status: completed

Deliverables:

- add an RAII terminal session that restores raw mode, alternate screen, and
  cursor on early return
- introduce a Git command runner abstraction with output limits, stderr capture,
  timeout-ready structure, and structured tracing
- migrate the highest-risk CLI paths first: status and diff/details commands

Tests:

- unit tests for command output limits and non-zero exit handling
- tests for truncated output behavior
- smoke validation that normal TUI setup and teardown compile cleanly

Results:

- added a root TUI terminal session guard that restores raw mode, alternate screen,
  and cursor through `Drop` while preserving explicit normal-exit restoration
- introduced a small `GitCommandRunner`/`GitCommandOutput` abstraction in the CLI
  backend with explicit stdout limit, stderr capture, and timeout-ready options
- preserved CLI error classification, stdout byte tracing, optional-lock behavior,
  and bounded status/diff output handling

### Phase 3: Shared Command Scheduler

Status: completed

Deliverables:

- extract shared debounce and coalescing logic from sync and async runtimes
- inject a testable clock instead of calling `Instant::now()` directly in tests
- preserve mutation barriers and stale read dropping

Tests:

- deterministic debounce tests without sleeps
- refresh coalescing tests
- mutation barrier and stale read tests

Results:

- added a shared harness `CommandScheduler` for debounce and queue coalescing
- migrated sync and async runtimes to call scheduler methods with explicit
  `Instant` values while keeping runtime clock access at dispatch/tick boundaries
- kept async write barriers, deferred reads, and stale read dropping in
  `AsyncRuntime`
- moved coalescing coverage into scheduler unit tests and added no-sleep debounce
  tests

### Phase 4: Command Metadata

Status: completed

Deliverables:

- centralize command labels, mutating classification, pending labels, debounce
  keys, and refresh keys
- remove duplicated match logic where possible without changing command enum shape

Tests:

- complete metadata coverage tests for all `Command` variants
- pending-state tests driven by metadata

Results:

- added a centralized core `Command` metadata layer for stable log labels,
  mutating classification, debounce keys, refresh coalescing keys, and pending
  operation labels
- exposed core refresh coalescing keys so the harness scheduler no longer keeps a
  duplicate refresh command match
- expanded command metadata tests to cover branch drilldown commands and refresh
  coalescing key behavior

### Phase 5: Explicit Details Requests

Status: completed

Deliverables:

- add typed Details request targets and request ids
- carry request ids through Details commands and results
- accept only matching Details results
- ensure stale Details results cannot leave loading state stuck

Tests:

- stale files, branch, commit, and commit-file details results are ignored safely
- latest Details request wins during rapid navigation
- cache-hit paths do not set pending work

Results:

- added explicit Details request ids, targets, and current-request tracking in
  `WorkStatusState`
- carried request ids through all Details commands and Git results for files diff,
  branch log, commit diff, and commit-file diff flows
- changed Details result acceptance to require the active request identity and
  target before clearing pending state, so stale results leave the latest request
  intact
- cleared the active Details request on cache hits, empty selections, skipped
  untracked details, and commit-files panel clear/close paths
- snapshot handlers now clear only the pending Details request targets they
  invalidate, preserving split-refresh Details requests from other panels

### Phase 6: Work State Machines

Status: completed

Deliverables:

- split `WorkStatusState` into typed refresh, details, mutation, pagination, and
  commit-files work states
- derive loading indicator state from the typed work states
- remove invalid bool/option combinations where possible

Tests:

- pending and completion tests for each command class
- loading indicator snapshots if output changes

Results:

- split `WorkStatusState` into typed `RefreshWork`, `DetailsWork`,
  `MutationWork`, `PaginationWork`, and `CommitFilesWork` substates
- migrated core, UI, and harness pending-work reads/writes to the typed work
  boundaries while preserving loading text and reducer behavior
- added focused core coverage for independent refresh, details, mutation,
  pagination, and commit-files work transitions

### Phase 7: Input Mode Routing

Status: completed

Deliverables:

- derive an explicit `InputMode` from `AppContext`
- route keyboard handling through mode-specific functions
- preserve modal, search, and panel shortcut priority

Tests:

- focused shortcut tests for each input mode
- modal priority tests
- quit key behavior tests

Results:

- added an explicit app-derived `InputMode` for editor, branch input, modal,
  search, and panel routing states
- split keyboard routing into mode-specific handlers while preserving global
  control keys, search-query fallback, and panel shortcut behavior
- added focused input tests for mode derivation, modal priority, search-query
  fallback, and existing quit-key behavior

### Phase 8: Git Backend Capabilities

Status: completed

Deliverables:

- split `GitBackend` into read, write, and history-rewrite capabilities
- keep a compatibility `GitBackend` composition trait during migration
- start separating hybrid backend provider responsibilities

Tests:

- mock and shared mock capability coverage
- read/write worker compilation and behavior tests

Results:

- split `GitBackend` into object-safe read, write, and history-rewrite
  capability traits while retaining a compatibility composition trait
- kept `Box<dyn GitBackend>` compatibility through boxed forwarding impls for
  each capability surface
- added mock, shared mock, and boxed capability dispatch coverage without
  changing runtime command execution behavior

### Phase 9: Refresh Semantics

Status: completed

Deliverables:

- make split refresh the canonical runtime path
- keep `RepoSnapshot` only as a fixture/compatibility model, or carry full
  `FilesSnapshot` metadata if retained
- ensure large-repository metadata is not lost through full-refresh code paths

Tests:

- large-repo metadata preservation tests
- fixture initialization tests

Results:

- made `Command::RefreshAll` execute the split refresh capability methods and
  return a metadata-preserving split refresh result
- kept `RepoSnapshot` as the compatibility/fixture refresh result while deriving
  fixture `index_entry_count` from fixture files instead of zeroing it
- preserved aggregate full-refresh failure semantics so split component failures
  clear all pending refresh targets
- added core and git coverage for large-repository metadata preservation through
  full-refresh command paths

### Phase 10: UI Projection Unification

Status: completed

Deliverables:

- introduce shared panel projection descriptors used by terminal rendering and
  tests
- reduce or explicitly mark legacy text rendering
- keep rendering pure and deterministic

Tests:

- existing terminal snapshots remain stable or are intentionally updated
- small terminal and modal overlay snapshots

Results:

- added a shared `PanelProjection` descriptor for panel identity, focus, titles,
  and projected content rows
- wired terminal rendering and legacy text rendering through the descriptor while
  keeping the Phase 11 styled-span row-source merge separate
- added focused projection coverage for descriptor metadata and row projection

### Phase 11: Single PanelLine Text Source

Status: completed

Deliverables:

- make styled spans the canonical row representation
- derive plain text from spans
- merge duplicated plain/styled file and commit formatting paths

Tests:

- style snapshots for file, commit, search, and selected rows
- plain text compatibility tests

Results:

- made `PanelLine` store canonical spans and derive plain text through a
  `text()` accessor
- updated terminal and legacy text renderers to consume the same span-backed row
  representation without changing snapshots
- merged plain file and commit formatting through their styled span builders and
  added plain-text derivation coverage

### Phase 12: Measured Performance Improvements

Status: completed

Deliverables:

- optimize search match lookup and highlighting allocations
- reduce repeated Details text traversal during scrolling
- add or update performance scenarios for render/search/details/status paths

Tests:

- unit tests for search correctness
- perf-suite coverage for the optimized paths where practical

Results:

- reduced file-tree search rendering work by avoiding tree-wide match mutation
  and using set-backed membership for large match sets
- reduced Details scroll rendering to a single text traversal for nonzero offsets
- added pure UI perf-suite operations for status render, search render, and
  details-scroll render paths

### Phase 13: Module Responsibility Split

Status: completed

Deliverables:

- split oversized files by responsibility after behavior has been stabilized
- keep exports narrow and module names discoverable
- avoid cross-layer rewrites during mechanical moves

Tests:

- full workspace validation after each mechanical split batch

Results:

- split the `ratagit-git` backend trait and error surface from the crate facade
  into a dedicated `backend` module
- preserved root-level public exports for `GitBackend`, capability traits, and
  `GitError` so external imports remain stable
- left command execution, git2/CLI backend implementations, and runtime data flow
  unchanged

### Phase 14: Documentation And Final Validation

Status: completed

Deliverables:

- update `ARCHITECTURE.md`, `docs/DESIGN.md`, `docs/TESTING.md`, and
  `docs/OBSERVABILITY.md` for final architecture
- update `docs/PRODUCT.md` only for user-visible behavior changes
- record final validation results here

Tests:

- full validation suite

Results:

- updated architecture, design, testing, and observability docs for the final
  Phase 1-13 architecture
- left `docs/PRODUCT.md` unchanged because Phase 14 documented internal
  architecture and validation rather than introducing new user-visible behavior
- `rtk cargo fmt` passed
- `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- `rtk cargo test` passed, 480 tests

### Phase 15: Files Tree Navigation Performance

Status: completed

Deliverables:

- remove full-tree Details target resolution from ordinary Files panel movement
- keep folder Details diffs bounded to the first 100 targets without collecting
  every descendant first
- avoid unnecessary lightweight Files tree index rebuilds during folder toggles
- add Files navigation performance coverage comparable to Commit Files navigation

Tests:

- unit tests for bounded Files Details target resolution in lightweight trees
- perf-suite coverage for Files panel navigation
- full validation suite

Results:

- added bounded Files Details target resolution so ordinary Files navigation no
  longer collects every descendant before applying the 100-target Details cap
- reused the lightweight Files tree index for bounded directory Details targets
  and preserved full selection behavior for stage, stash, reset, and discard
- avoided rebuilding lightweight Files tree sources during folder toggles when
  the cached tree index is already current
- added unit coverage for bounded lightweight directory targets and untracked
  directory markers
- added `files-navigation` perf-suite coverage alongside Commit Files navigation
- `rtk cargo fmt` passed
- `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- `rtk cargo test` passed, 482 tests

### Phase 16: Large Repository Profile Analysis

Status: completed

Deliverables:

- add optional built-in profile mode to the manual performance suite
- record stage timings for Git commands, backend calls, parsing, tree
  initialization, reducer/update loops, target selection, and render loops
- report profile bottlenecks in JSON and Markdown without changing default TUI
  behavior

Tests:

- unit coverage for profile option parsing and bottleneck summary selection
- smoke perf-suite coverage for profile rows and report output
- full validation suite

Results:

- added `--profile` to `perf-suite` and upgraded reports to
  `ratagit.perf-suite.v2`
- added `profiles` JSON rows and a Markdown `Profile Bottlenecks` section
- kept large and huge profile runs manual; normal tests still use smoke-sized
  repositories only

## Latest Validation

- Phase 0: documentation-only review complete; behavior unchanged
- Phase 1: `rtk cargo fmt` passed
- Phase 1: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 1: `rtk cargo test` passed, 445 tests
- Phase 2: `rtk cargo fmt` passed
- Phase 2: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 2: `rtk cargo test` passed, 449 tests
- Phase 3: `rtk cargo fmt` passed
- Phase 3: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 3: `rtk cargo test` passed, 451 tests
- Phase 4: `rtk cargo fmt` passed
- Phase 4: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 4: `rtk cargo test` passed, 453 tests
- Phase 5: `rtk cargo fmt` passed
- Phase 5: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 5: `rtk cargo test` passed, 460 tests
- Phase 6: `rtk cargo fmt` passed
- Phase 6: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 6: `rtk cargo test` passed, 465 tests
- Phase 7: `rtk cargo fmt` passed
- Phase 7: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 7: `rtk cargo test` passed, 468 tests
- Phase 8: `rtk cargo fmt` passed
- Phase 8: targeted `rtk cargo test -p ratagit-git` passed, 76 tests
- Phase 8: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 8: `rtk cargo test` passed, 471 tests
- Phase 9: targeted refresh tests passed for `ratagit-core`, `ratagit-git`, and
  `ratagit-harness`
- Phase 9: `rtk cargo fmt` passed
- Phase 9: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 9: `rtk cargo test` passed, 476 tests
- Phase 10: targeted `rtk cargo test -p ratagit-ui` passed, 118 tests
- Phase 10: `rtk cargo fmt` passed
- Phase 10: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 10: `rtk cargo test` passed, 477 tests
- Phase 11: targeted `rtk cargo test -p ratagit-ui` passed, 120 tests
- Phase 11: targeted `rtk cargo test -p ratagit-ui --test snapshots` passed, 70
  tests
- Phase 11: `rtk cargo fmt` passed
- Phase 11: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 11: `rtk cargo test` passed, 479 tests
- Phase 12: targeted `rtk cargo test -p ratagit-ui` passed, 121 tests
- Phase 12: targeted `rtk cargo test --bin perf-suite` passed, 15 tests
- Phase 12: `rtk cargo fmt` passed
- Phase 12: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 12: `rtk cargo test` passed, 480 tests
- Phase 12: perf smoke passed for `status-render`, `search-render`, and
  `details-scroll-render`; result `perf-1777319049-197283500`
- Phase 13: targeted `rtk cargo test -p ratagit-git` passed, 79 tests
- Phase 13: `rtk cargo fmt` passed
- Phase 13: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 13: `rtk cargo test` passed, 480 tests
- Phase 14: `rtk cargo fmt` passed
- Phase 14: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 14: `rtk cargo test` passed, 480 tests
- Phase 15: targeted `rtk cargo test -p ratagit-core` passed, 139 tests
- Phase 15: targeted `rtk cargo test --bin perf-suite` passed, 15 tests
- Phase 15: `rtk cargo fmt` passed
- Phase 15: `rtk cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 15: `rtk cargo test` passed, 482 tests
- Phase 16: targeted `cargo test --bin perf-suite` passed, 16 tests
- Phase 16: `cargo fmt` passed
- Phase 16: `cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 16: `cargo test` passed
- Phase 16: profile smoke passed for `status`, `files-navigation`, and
  `status-render`; result `perf-1777465934-980516600`
- Phase 17: `cargo fmt` passed
- Phase 17: `cargo clippy --workspace --lib --bins -- -D warnings` passed
- Phase 17: `cargo test` passed
