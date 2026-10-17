## Overview

Core owns lossless patch models; GitBackend owns native patch generation and
application; UI owns pure patch-row projection.
AppContext keeps reviewed patch data under repo.details, interaction under
ui.details, and request ownership under work.details. Load results require a
matching request ID and target; mutation barriers still reject stale reads.
Patch rows retain presentation-side old/new line coordinates independently of
native source LineIds. Marks and visual anchors are pure UI state; operations
resolve them to changed source lines before returning GitBackend commands.
Native file-header bytes live in PatchFile.header independently of writable body
bytes, so read-only/truncated patches preserve Git metadata too. Passive Files
and Commit Files text projects into read-only rows in core; interactive native
rows and passive rows share one pure UI renderer. Source prefixes and indentation are preserved,
and core scroll bounds use the same visible preview rows as the UI.
Runtime teardown stops queued jobs and bounds worker joining to 100 ms. In-flight
native calls cannot be forcibly cancelled; a detached worker exits after its
current call and never starts queued work.

Historical Commit Files reuse native patch loading and AppContext-owned hunk
selection, with captured `CommitFiles` requests and request-ID ownership.
Historical mutations are typed workflow operations through GitBackend's
exclusive mutation worker; they never fall through to staging. Whole-file
history edits restore parent tree entries, and partial edits use reviewed bytes
in a temporary index. Descendant commits are replayed in memory before moving
the branch; conflicting/unsupported histories and checkout obstructions are
rejected first. Worktree-only operations refresh Files while retaining the
Commit Files context; successful history rewrites close it and refresh panels.

The custom patch builder retains immutable native source files in
`repo.custom_patch`, source identity and accumulated whole/line inclusion in
`ui.custom_patch`, and a serial intent queue with independent read ownership in
`work.custom_patch`. Its load/render commands do not share Details debounce
ownership. Reset/replacement and mutation scheduling invalidate pending reads;
navigation preserves completed inclusion. Captured aggregate operations validate
every source file and apply the combined native bytes through one GitBackend
mutation, retaining whole-file metadata and partial-line guards. Preview/copy
are backend projections of the same selection; rendering remains pure.

Repository navigation tabs keep backend-derived typed lists in `repo.tabs`,
active siblings and independent selection/scroll in `ui.tabs`, and per-tab
request ownership in `work.tabs`. `LoadRepositoryTab` is an ordinary read command
in both runtimes. Mutation scheduling invalidates tab requests and cached lists;
the runtime generation barrier continues to reject overlapping stale reads.
Metadata Details are pure projections of the selected entry, and active view
resolution prevents commands from targeting hidden Files/Branches/Commits lists.

ratagit is a lazygit-like Git TUI built with Rust and ratatui.

The system follows a strict unidirectional data flow:

Input → Action → Update → AppContext → Render

---

## Core Principles

### 1. Single Source of Truth

Application UI state must be derived from `AppContext`. Immutable presentation
inputs, including the theme and spinner frame, are supplied explicitly through
`RenderContext`.

- No hidden state
- No implicit global variables
- No interaction state outside AppContext
- No `GitBackend`, runtime handles, environment, clock, or external dependency
  handles inside AppContext

`AppContext` is the pure root state object. Its top-level categories are:

- `repo`: Git/backend-derived data, including status, file rows, branch rows,
  commit rows, stash rows, Commit Files rows, Details text/errors/caches, and
  commit pagination metadata
- `ui`: interaction state, including focus, search, panel selections, scroll
  offsets, tree projection caches, multi-select state, Details scroll offset,
  and modal/editor state
- `work`: pending refresh/details/operation state, commit pagination loading
  intent, Commit Files loading state, and the last completed command label.
  These are typed substates (`RefreshWork`, `DetailsWork`, `MutationWork`,
  `PaginationWork`, and `CommitFilesWork`) rather than loose flags.

Cross-layer helpers must make data/UI dependencies explicit. For example, a
Files helper that needs both repository rows and tree selection state receives
`repo.files.items` and `ui.files` separately instead of a mixed panel state.

---

### 2. Pure Rendering

UI rendering must be pure:

```text
(AppContext, TerminalSize, RenderContext) -> Frame
```

`RenderContext<'a>` carries explicit render-only inputs: the current loading
`spinner_frame` and an immutable `&'a UiTheme`. The executable loads and validates
the theme once through `src/config.rs`, before enabling raw mode or entering the
alternate screen, and owns it for the event loop. Each draw passes that theme to
panel, row, diff, and modal projections. Configuration file IO and environment
lookup stay in the executable; `ratagit-core` remains dependency-free, and the
UI reads neither files nor environment to choose colors. Theme data does not
belong in `AppContext`.

The real TUI advances the spinner frame in its event loop, but rendering receives
it as data and must not read time itself. Convenience test APIs use a default
`RenderContext` with spinner frame zero and `MAIN_UI_THEME` for deterministic
snapshots. Theme-aware callers pass their context explicitly. Preset resolution,
semantic role overrides, and fixed RGB contrast validation are `UiTheme`
operations; the JSON schema and discovery rules are documented in
[Themes](docs/THEMES.md).

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
- UI cannot mutate AppContext
- Core owns all state transitions
- Git is accessed only via `GitBackend`

---

### 4. Side Effects via Commands

Update returns commands:

```rust
fn update(state: &mut AppContext, action: Action) -> Vec<Command>
```

Commands:

- Git operations
- async tasks
- IO

The real TUI executes read-only Git commands through a fixed background worker
pool and executes mutating Git commands through one exclusive background worker.
Whole-repository refresh requests are split into independent read commands for
Files/status, Branches, Commits, and Stash so a slow file status scan cannot
delay other left-panel data from reaching `AppContext`. The async runtime uses
mutation barriers so stale read results cannot apply after queued repository
mutations. Sync and async runtimes share one command scheduler for debouncing and
refresh coalescing, driven by command metadata from `ratagit-core`. Details
commands carry explicit request ids and targets, and reducers accept only the
result matching the active request. The UI thread remains responsible only for
input, reducer updates, result draining, and pure rendering. Harness scenarios
may use the synchronous runtime to keep mock state assertions deterministic.

---

### 5. Determinism

Same:

- AppContext
- terminal size
- RenderContext (theme and spinner frame)
- input sequence

Must produce identical UI.

---

## Packages

The repository is a Cargo workspace with a root application package and
internal library packages under `libs/`.

### ratagit

- TUI binary entrypoint
- terminal setup and event loop, including the RAII terminal session guard that
  restores raw mode, alternate screen, and cursor on normal exit or early return
- backend selection
- configuration path discovery, JSON parsing, and startup theme loading before
  terminal setup

### ratagit-core

- AppContext
- Action
- Reducer (update)
- Command
- typed Git failure payloads used by reducers for semantic recovery paths such
  as divergent push and unmerged branch delete confirmations
- command metadata for log labels, mutating classification, pending labels,
  debounce keys, and refresh coalescing keys

### ratagit-ui

- Pure rendering functions
- Widgets
- Layout
- Panel projections that describe panel identity, focus, title, and span-backed
  `PanelLine` rows for both terminal rendering and legacy text tests

### ratagit-git

- `backend` module containing `GitBackendRead`, `GitBackendWrite`,
  `GitBackendHistoryRewrite`, the root-compatible `GitBackend` composition
  trait, and `GitError`
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
- Git CLI command execution goes through a command runner that centralizes
  stdout limits, stderr capture, optional-locks mode, timeout-ready options, and
  structured command tracing

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
- shared scheduler used by both sync and async runtimes for debounce and
  coalescing behavior

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
  - single source of truth in `AppContext`
  - side effects only through `Command` + `GitBackend`
  - pure rendering in `ratagit-ui::render`
- Refresh command execution applies per-panel `GitResult` values independently:
  Files/status, Branches, Commits, and Stash may appear in any worker completion
  order, and pending refresh targets are tracked in `AppContext.work`. The
  canonical full-refresh command executes the same split backend capability
  methods as the independent refresh commands and preserves `FilesSnapshot`
  metadata such as large-repo mode, truncation, and skipped scans.
- Reusable projections and expensive read results, such as file-tree rows and
  files-detail diffs, are cached only in `AppContext` and invalidated by reducer
  state transitions.
- Files and Commit Files use the same `AppContext`-owned tree index for
  deterministic parent/child relationships. Folder expand/collapse rebuilds
  visible rows from cached children without rescanning every file path, and
  item changes sync through remove/add/metadata updates. In large-repo mode,
  the Files tree initializes collapsed with a lightweight projection that does
  not precompute `row_descendants` for every path. Details commands resolve
  deterministic `FileDiffTarget` values from current `AppContext` and cap
  automatic file diffs to the first 100 targets.
- Automatic full-commit Details previews are bounded in `GitBackend` so large
  commit patches cannot feed unbounded text into `AppContext` or pure rendering.
- Branches subviews load selected-branch commits through `GitBackend` read-only
  commands and store the results under Branches-owned `AppContext` state while
  reusing the same commit-row and commit-file tree projections as the main
  Commits panel.
- Keyboard input is routed by an explicit `InputMode` derived from `AppContext`;
  global Details scrolling remains first, then modal/editor/search handlers, then
  panel handlers.
- Shared Git workflows keep captured dialogs, copied commit IDs and remote read
  request ownership under `ui.workflow`. Typed operations pass through the existing
  RepositoryOperation command and exclusive worker. Remote reads use request IDs
  and the runtime generation barrier. Editor/clipboard completions return to core.
- `src/bin/perf-suite.rs` covers backend operations plus pure UI render hot paths
  for status render, search render, and Details scroll render.

---

## Event Loop

```text
drain GitResult channel
→ render AppContext
→ read input
→ map to Action
→ update(AppContext)
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
- branching based on terminal state outside AppContext
- hidden caches not in AppContext

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

AppContext must:

- be serializable (for debugging)
- be inspectable
- keep Git-derived data in `repo`
- keep interaction state and render caches in `ui`
- keep pending-work state in `work`
- keep top-level `notices` and `last_operation` only for app-wide feedback
- avoid nested complexity explosion

---

## Action Design Rules

Actions must:

- be explicit
- not carry hidden meaning
- be testable
