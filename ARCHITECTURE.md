## Overview

ratagit is a lazygit-like Git TUI built with Rust and ratatui.

The system follows a strict unidirectional data flow:

Input → Action → Update → AppState → Render

---

## Core Principles

### 1. Single Source of Truth

All UI must be derived from `AppState`.

- No hidden state
- No implicit global variables
- No UI-only state outside AppState

---

### 2. Pure Rendering

UI rendering must be pure:

```text
(AppState, TerminalSize) -> Frame
```

Forbidden:

- calling Git inside render
- reading env/time/random
- mutating state

---

### 3. Layer Separation

```text
CLI → Core → UI
        ↓
      Git
```

Rules:

- UI cannot call Git
- UI cannot mutate AppState
- Core owns all state transitions
- Git is accessed only via `GitBackend`

---

### 4. Side Effects via Commands

Update returns commands:

```rust
fn update(state: &mut AppState, action: Action) -> Vec<Command>
```

Commands:

- Git operations
- async tasks
- IO

The real TUI executes read-only Git commands through a fixed background worker
pool and executes mutating Git commands through one exclusive background worker.
Whole-repository refresh requests are split into independent read commands for
Files/status, Branches, Commits, and Stash so a slow file status scan cannot
delay other left-panel data from reaching `AppState`. The async runtime uses
mutation barriers so stale read results cannot apply after queued repository
mutations. The UI thread remains responsible only for input, reducer updates,
result draining, and pure rendering. Harness scenarios may use the synchronous
runtime to keep mock state assertions deterministic.

---

### 5. Determinism

Same:

- AppState
- terminal size
- input sequence

Must produce identical UI.

---

## Packages

The repository is a Cargo workspace with a root application package and
internal library packages under `libs/`.

### ratagit

- TUI binary entrypoint
- terminal setup and event loop
- backend selection

### ratagit-core

- AppState
- Action
- Reducer (update)
- Command

### ratagit-ui

- Pure rendering functions
- Widgets
- Layout

### ratagit-git

- GitBackend trait
- Mock backend for deterministic harness scenarios
- Hybrid real backend: `git2` handles repo discovery, snapshot metadata, file
  diffs, stage, and unstage
- File status refresh uses `git status --porcelain=v1 -z` inside `GitBackend`
  for large-repository performance, with git2 status as a fallback
- Status refresh chooses `StatusMode::LargeRepoFast` when the index has at
  least 100,000 entries. In this mode `git status` uses
  `--untracked-files=no`, status stdout is capped at 64 MiB, parsed file entries
  are capped at 50,000, and `FilesSnapshot` records whether the status result
  was truncated or skipped untracked scanning.
- Status refresh chooses `StatusMode::HugeRepoMetadataOnly` when the index has
  at least 1,000,000 entries. In this mode the backend skips automatic file
  status collection after counting the index and reports deterministic metadata
  so Commits, Branches, Stash, and bounded Details work can load without a
  whole-working-tree scan.
- Read-only Git CLI commands run with `GIT_OPTIONAL_LOCKS=0` to reduce index
  lock/refresh pressure in very large repositories.
- Internal Git CLI executor handles operations not yet represented through
  git2, such as commit, branch mutation, checkout, stash, reset, nuke, and
  discard

### ratagit-observe

- tracing subscriber setup
- file log sink setup
- environment-derived log level and path configuration
- non-blocking log guard held by the TUI entrypoint

### ratagit-testkit

- fixtures
- UI assertions

### ratagit-harness

- scenario runner
- input driver
- snapshot + assertions

---

## MVP Implementation Notes

- The root package is `ratagit`; `cargo run` starts the TUI from `src/main.rs`.
- Internal libraries are independent Cargo packages under `libs/`:
  - `ratagit-core`
  - `ratagit-ui`
  - `ratagit-git`
  - `ratagit-observe`
  - `ratagit-testkit`
  - `ratagit-harness`
- Shared dependency versions and internal path dependencies live in the root
  `Cargo.toml` via workspace inheritance.
- Runtime command execution uses `ratagit-harness::AsyncRuntime` in the real TUI
  and `ratagit-harness::Runtime` in deterministic harness scenarios to preserve:
  - single source of truth in `AppState`
  - side effects only through `Command` + `GitBackend`
  - pure rendering in `ratagit-ui::render`
- Refresh command execution applies per-panel `GitResult` values independently:
  Files/status, Branches, Commits, and Stash may appear in any worker completion
  order, and pending refresh targets are tracked in `AppState.work`.
- Reusable projections and expensive read results, such as file-tree rows and
  files-detail diffs, are cached only in `AppState` and invalidated by reducer
  state transitions.
- In large-repo mode, the Files tree initializes collapsed with a lightweight
  projection that does not precompute `row_descendants` for every path. Details
  commands resolve deterministic `FileDiffTarget` values from current
  `AppState` and cap automatic file diffs to the first 100 targets.
- Automatic full-commit Details previews are bounded in `GitBackend` so large
  commit patches cannot feed unbounded text into `AppState` or pure rendering.

---

## Event Loop

```text
drain GitResult channel
→ render AppState
→ read input
→ map to Action
→ update(AppState)
→ enqueue Commands
→ worker pool runs GitBackend
→ receive results
→ render
```

---

## Anti-Patterns (Forbidden)

- UI directly mutates state
- UI calls Git
- logic inside render()
- branching based on terminal state outside AppState
- hidden caches not in AppState

---

## Code Structure Rules

### File Size

- max 500 lines per file
- split when growing

### Module Rules

- one responsibility per module
- no cyclic dependencies

### Naming

- explicit > clever
- avoid abbreviations

---

## State Design Rules

AppState must:

- be serializable (for debugging)
- be inspectable
- avoid nested complexity explosion

---

## Action Design Rules

Actions must:

- be explicit
- not carry hidden meaning
- be testable
