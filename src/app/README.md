# Application Architecture and Module Boundaries

This folder is organized so new contributors can find behavior quickly.

## Architecture Decision

Keep the existing Flux pipeline and make its module boundaries enforceable.
Ratatui presents multiple [application patterns](https://ratatui.rs/concepts/application-patterns/);
it does not prescribe a single application framework.

- [Flux](https://ratatui.rs/concepts/application-patterns/flux-architecture/) provides
  the action/dispatcher/store/view data flow already used here. Input and background
  completions must follow that same path.
- [The Elm Architecture](https://ratatui.rs/concepts/application-patterns/the-elm-architecture/)
  separates model, update and view. Retain pure reducers and immutable render snapshots;
  Rust reducers can mutate their owned state without cloning the whole model per action.
- [Component architecture](https://ratatui.rs/concepts/application-patterns/component-architecture/)
  groups a component's state, updates and rendering. Use that ownership principle to
  organize features, while keeping shared repository state and effect scheduling centralized.

For Ratagit, these principles support incremental feature ownership within Flux.
Giving every UI widget its own repository handle, global action channel or independently
mutable copy of shared selection would introduce additional coordination.

## Entry Files
- `../main.rs`: startup configuration, application construction and session/runtime composition.
- `../terminal.rs`: terminal initialization, mouse capture, panic restoration,
  explicit cleanup and unwind protection, with no application scheduling dependency.
- `../runtime/mod.rs`: LocalSet ownership, channel wiring, startup requests and
  shutdown/join order. The UI callback receives only channels and counters.
- `../runtime/loops.rs`: action dispatch, FIFO command consumption, effect
  completion selection and the five-second auto-refresh loop.
- `../runtime/ui.rs`: terminal input polling, immutable snapshot rendering,
  tick decisions and the existing frame timing.
- `../runtime/telemetry.rs`: application logging, performance counters and the
  unchanged debug log format/cadence.
- `app.rs`: `App` startup composition and `AppState` transitions. The dispatch loop owns the state; effects never receive it.
- `runtime.rs`: cloneable Git execution dependencies (`AppRuntime`).
- `help.rs`: contextual keybinding menu state, filtering and selection. The
  command registry supplies effective keys; `HelpStore` reduces typed help
  actions through `HelpAccess` and emits selected commands back into Flux.
- `../background.rs`: the shared read protocol, task identity and immutable completion payloads.
- `../flux/task_manager.rs`: authoritative registration, admission, cancellation and generation acceptance for background reads.
- `background_poll.rs`: refresh scheduling and application of background completion events; it performs no Git I/O.
- `diff_coordinator.rs`: translates Flux actions into diff feature inputs and applies
  owned write requests, log messages and refresh intentions; schedules and cancels runtime tasks.
- `files_coordinator.rs`: adapts file-feature updates and immutable selection views;
  coordinates accepted status results with the diff feature.
- `revisions_coordinator.rs`: translates revision read/cancel intentions into shared
  background tasks and coordinates accepted results with search, diff and redraws.
- `legacy_test_helpers.rs`: synchronous compatibility helpers for older unit tests, excluded from production.
- `command.rs`: queued follow-up actions and repository effect commands.
- `flux/command_output.rs`: the dispatcher-owned command collector, explicit
  publication ordering and cancellation of unpublished background reads.
- `commands.rs`: command IDs, aliases, context availability, default bindings, execution mapping and help labels. Keyboard input, the command palette and shortcut hints all use this registry.
- `context.rs`: `ViewContext` identities (including branch/commit/stash sources), parent views and modal `UiContext`. Contexts are derived from reducer state and included in render snapshots.
- `test_dispatch.rs`: test helpers:
  - `dispatch_test_action(app, action)` dispatches one `DomainAction` through Flux.
  - `map_test_key(app, key)` maps key input via `flux::input_mapper`.
  - `dispatch_test_key(app, key)` maps and dispatches key input through Flux.
- `update_tests.rs`: behavior regression tests (included by `app/mod.rs` under `#[cfg(test)]`).

## Domain Modules
- `panel_nav.rs`: active panel counting and list navigation behavior.
- `input_mode.rs`: commit/branch/stash editor input state transitions.
- `search/`: private search scope/query state, pure matching/jump decisions and immutable selectors.
- `search_coordinator.rs`: adapt visible feature data to searchable text, apply navigation intentions and manage search editor input boundaries.
- `selectors.rs`: shared immutable `SelectionView` over reducer state or a render snapshot; revision selection, list navigation and diff targets use the same context.
- `files/selectors.rs`: shared `FileSelection` for staging, discard, stash and visual ranges.
  Selectors return typed operation targets; command mapping translates them into Flux actions.
- `hints.rs`: shortcut hints generated from command definitions and the effective keymap. Unbound or shadowed keys are omitted.
- `files/state.rs`: repository status, file-tree rebuilding and cursor/anchor restoration.
- `revisions/`: owns branch/commit/stash panel data, selection, tree lifecycle,
  history pagination decisions and graph highlighting.
- `diff/`: owned diff/hunk state, pure updates, read-only selectors and cache/load decisions.

## Data Flow
`UI Input -> Action -> Dispatcher (Flux stores) -> AppState -> AppStateSnapshot -> UI Render`

Reducers enqueue owned effect requests. `flux::runtime::EffectExecutor` runs a bounded number of reads concurrently and serializes local writes in arrival order. Git calls and receiver waits execute on blocking workers, without borrowing state or blocking the UI thread. Fetches share the read lane and can proceed independently of local writes.

`EffectRequest -> AppRuntime / GitRepository -> Completion Action -> Dispatcher -> AppState`

Background completion events carry their task generation. State rejects cancelled or superseded requests before applying a payload; changing the diff selection or requesting a refresh cancels the previous diff generation. Completed events participate in the normal dirty-state and snapshot publication path.

## Background Read Protocol and Lifecycle

`BackgroundLoad` is the only execution description. `TaskRequest` carries that
load together with its derived task key, global generation and admission priority;
`EffectRequest::LoadBackground` carries only the complete request. Coordinators
no longer construct a separate `TaskRequestKind` or duplicate a load next to its
registration. The shared protocol lives outside application composition; its diff
fields refer to feature data models, without execution handles or mutable state.

`TaskManager` owns the latest-generation index and all submitted generations.
Registration happens before command publication. Normal/low reads retain the
six-task admission limit; high-priority reads retain their admission bypass.
Superseded generations still count until their completion arrives or their key
is explicitly cancelled, including when the latest result has already completed.
Cancellation removes all submitted generations for that key and removes matching
unpublished coordinator commands from `CommandOutput`. It does not stop native
Git I/O. Completion retires tracked work once and rejects cancelled, duplicate or
superseded generations before inspecting payload data.

The previous wrapper map, speculative priority/pending queue, synthetic ready
result queue and per-event payload map are removed. These queues did not drive
production execution: requests were registered and immediately marked started,
and each completion was submitted then collected in the same action. Publication
order still belongs to `CommandOutput`, read/write concurrency to `EffectExecutor`,
and native Git ordering to the repository workers. Their ordering policies and
the telemetry log format are unchanged.

Diff requests retain the single `active` task slot across selections. Concrete
diff identity remains in the cache key and selected-target check. Commit and
stash file requests retain their shared task slots, while branch histories retain
branch-specific keys. Operation epoch checks and revision branch/commit/stash
source checks still run in the existing domain/feature owners after generation
acceptance. An obsolete operation observation still cannot change operation
state, while the status payload follows its existing application policy.

Completion events hold `Arc<BackgroundPayload>`. Status, revision lists, file
lists, diff lines and conflict paths use shared immutable storage, adopted by
features through cheap handle clones. The store adapter borrows the completion;
it no longer deep-clones it before checking the generation. Retained events and
snapshots keep their original payloads when state is later changed or replaced.

`flux/task_manager/tests.rs` covers derived request identity, admission accounting,
supersession, cancellation, duplicate/failure acceptance and independent keys.
`app/background_tests.rs` exercises completions through the production dispatcher:
small and large histories clone completions without allocations and have equal,
bounded acceptance allocations; stale histories are rejected without allocations.
It also checks status/conflict/file payload identity, epoch/source rejection,
retained snapshots, silent cancelled failures and retries. Existing runtime and
repository worker regressions retain event order, fast/full refresh, read/write
independence and native worker routing coverage.

## Unified Command Output

`ReduceCtx::command` and application coordinators receive one explicit
`CommandOutput` owned by the dispatcher. `ReduceOutput` contains only UI
invalidation; `AppState` contains no effect outbox. Capability methods that can
schedule or cancel work take this output as a parameter. Pure feature reducers
continue to return typed intentions without depending on the collector.

The collector stores commands in one sequence with explicit publication order.
All store commands precede coordinator requests, retaining insertion order within
each group. This preserves the previous contract where the dispatcher collected
store outputs first and appended `AppState.effects` after the full broadcast.
Cancellation removes matching unpublished coordinator reads from the same
collector. It does not remove unrelated commands or cancel already-running I/O;
task generations continue to reject stale completions.

`App` startup composition builds initial output and transfers it to the dispatcher
before the first action. That output follows the same ordering and cancellation
rules, and is published once. Each dispatch applies invalidation after all stores,
then drains the collector into `DispatchResult.commands`. Follow-up state actions
use the action path described below. The executor retains serialized local writes
and bounded concurrent reads.

Store tests collect commands through the same output contract with narrow state
doubles. Older direct-transition fixtures have test-only `App` adapters using an
explicit output; they cannot add an outbox to production `AppState`. Dispatcher
and runtime tests cover mixed output order, deferred action application,
same-dispatch cancellation, bootstrap publication, write ordering and state events.

## State Actions and Repository Effects

`RuntimeEvent` belongs to `flux/action.rs`. Background processing, refresh flushing,
diff reloads, commit-load decisions and revision tree transitions are queued as
`Command::Sync(DomainAction::Runtime(event))`, created with `Command::runtime`.
They are not variants of `EffectRequest`; that type contains repository I/O only.
Both effect execution and worker-failure conversion match it exhaustively.

`runtime::loops::command_loop` retains the existing FIFO command-consumption point and
completion selection. `flux::runtime::route_command` forwards follow-up actions
directly to the action queue, without calling `EffectExecutor::enqueue`. A state
action is reduced only when the dispatcher receives it; emitting or routing it
does not perform recursive state updates. Repository commands enter the executor
at their original positions in the same queue, preserving write order and the
relative order of sync actions, state events and bootstrap requests.

State decisions can run while the write lane and all six read slots are blocked.
Any Git reads they subsequently request still obey concurrency limits. Shared
cancellation, task generations, dirty tracking and snapshot publication retain
their previous behavior. The deterministic test driver handles these same actions
through its pending action queue, with no state-event fallback in the effect path.

Test input mapping follows the same runtime path via `flux::input_mapper` and test dispatch helpers. The deterministic test driver reuses the production effect executor logic and dispatches its completion events. `flux::runtime_tests` additionally exercises the async scheduler with controlled completion gates.

## Terminal Session and Runtime Ownership

Application/configuration construction still precedes terminal initialization.
`TerminalSession` owns the initialized terminal and installs mouse cleanup before
Ratatui's chained panic diagnostics. Normal completion disables mouse capture,
attempts raw-mode/alternate-screen restoration even after mouse cleanup fails,
then drops the terminal. Application errors take precedence over mouse cleanup
errors, which take precedence over restoration errors. An unfinished session has
a best-effort drop guard; during panic unwinding it avoids repeating restoration
already performed by the panic hook.

The runtime retains the current-thread Tokio executor and LocalSet, starts
dispatch/command/refresh tasks in the original order, and preserves the existing
unbounded action/command queues and snapshot/version watch channels. Dispatch
publishes dirty snapshots and the state version before sending follow-up commands.
The UI receives no mutable AppState. On success or UI error, orchestration sends
shutdown and joins dispatch, command and refresh tasks in order, returning the UI
result unchanged. Dropping the effect executor aborts completion waits without
waiting for uncancellable native Git work.

Runtime regressions exercise production loops with a headless UI callback:
FIFO input/actions/writes, read independence, snapshot publication, initial
auto-refresh, channel closure and shutdown with blocked Git. Terminal unit tests
inject operation failures and unwinding without touching a real console. Unix
PTY subprocess tests additionally verify normal quit restores raw mode and mouse
capture, and panic restoration precedes diagnostics. The PTYs are isolated from
the user's terminal; libc is a Unix-only test dependency.

## Enforced Store Boundaries

`flux::stores::StateAccess` is the combined interface used when the dispatcher
registers heterogeneous stores. Each store implements `Store<S>` with explicit
capability bounds, and receives `ReduceCtx<'_, S>`. Its implementation can only
call methods available through those bounds. There is no default full-state type.

The former `CoreAccess` mixed lifecycle, logging, invalidation, command resolution,
panel focus, branches, stash selection and runtime completion handling. These now
have separate contracts in `flux/stores/state_access.rs`, implemented by the adapter
in `app/state_access_impl.rs`.

| Store | Required state capabilities |
| --- | --- |
| Quit | Lifecycle |
| Files | Files, Log |
| Commit | Log |
| Diff, Ops | Refresh |
| Stash | Log, Stash |
| Runtime | Runtime, Refresh |
| Input | Command, Input, Log, Search |
| Navigation | Navigation, Panel, Refresh, Search |
| Revision | Panel, Revision, Search |
| Search | Input, Log, Search |
| Branch | Branch, Input, Log, Overlay, Refresh |
| Selection | Files, Log, Overlay, Refresh |
| Overlay | Input, Log, Overlay, Refresh, Selection |

Store registration order, action broadcasts, command ordering and effect execution
remain the same. Quit returns full invalidation in `ReduceOutput`; the dispatcher
applies it after all stores have reduced the action. `UiInvalidation::apply` needs
only `InvalidationAccess`, and log helpers need only `LogAccess`. Dirty-region bits
have one definition shared by reducer outputs and application dirty tracking.
`LogAccess::push_log` must also invalidate the command log, including on failures.

When adding a reducer:

1. Declare the capabilities it consumes in its `Store<S>` implementation.
2. Emit owned commands through `ReduceCtx::command`, return invalidation, and
   execute Git I/O through effects. Pass the collector to coordinators explicitly.
3. Implement a new capability on the application adapter only if an existing
   capability does not describe that responsibility.
4. Test decisions with small state doubles, then test cross-feature behavior
   through the dispatcher. Do not add unrelated no-op trait implementations to
   a unit-test double just to satisfy a full-state interface.

`flux/stores/capability_tests.rs` exercises file writes/completions with isolated
file feature state and logging, stash target
selection, debounce boundaries and invalidation without constructing an App or
repository. Quit tests require only lifecycle state. Dispatcher tests cover
broadcast/command order and applying quit invalidation.

## Diff/Hunk Feature Ownership

Diff state used to be split across `GitState` (patch and display lines),
`UiState` (focus, cursor, explicit selection and scroll), `RefreshScheduler`
(reload/debounce metadata), and the application diff cache manager. These now
belong to `AppState::diff: DiffState`, whose internal fields are private to
the feature module.

| Feature file | Responsibility |
| --- | --- |
| `diff/state.rs` | Own the document, interaction, scroll, cache and load identity; decide cache reuse, invalidation and completion acceptance. |
| `diff/update.rs` | Reduce typed `DiffAction` inputs using only diff state and an explicit files-context flag; return `DiffUpdate` with owned writes and cross-feature intentions. |
| `diff/selectors.rs` | Project `DiffView` and `HunkView`; determine focus/navigation/side/write command availability and validated hunk targets. |
| `diff/target.rs` | Apply explicit staged/unstaged choice, derive cache/task identities and decide which repository refresh affects a target. |
| `diff/cache.rs` | Cache documents with their original shared patch objects and preserve existing LRU behavior. |
| `diff/tests.rs` | Test feature decisions directly, without an App, repository or task runtime. |

The production feature has no `AppState`, Flux or terminal dependency. Its Git
dependencies are lossless patch/data models, not repository handles.
`diff/loader.rs` retains the older synchronous loader tests and is compiled only
under `cfg(test)`; production repository reads remain in effects.

The application coordinator obtains the base target from `SelectionView`, passes
only that target and visible file nodes into diff loading decisions, and translates
feature outputs into `EffectRequest` plus status refreshes. Task generations and
cancellation remain in the shared task manager; after generation acceptance, the
feature also rejects payloads that no longer match the selected target.
An in-flight write identity survives a panel/selection change until its completion.

Both snapshot constructors use `DiffState::view()`; command mapping, palette
availability and shortcut hints use `HunkView` selectors. Renderers keep their
immutable snapshot inputs. Production code outside `diff/` cannot directly
mutate a patch, cursor, selection or scroll field.

## Files Feature Ownership

`AppState::files: FilesState` now owns the observed `GitStatus`, visible file tree,
expanded directories, list cursor and visual selection anchor. These fields
previously lived in `GitState` and `UiState`. The feature keeps both status and
its panel projection private; production consumers receive `status()`,
`view()` and `selection(active)` shared views.

| File | Responsibility |
| --- | --- |
| `files/state.rs` | Own status/tree/selection data, preserve the cursor and anchor with `TreeCursor`, and keep collapsed directories across refreshes. |
| `files/update.rs` | Reduce typed navigation, tree, selection and file-operation inputs; return owned `FileWrite` requests and log/refresh/editor/diff intentions. |
| `files/selectors.rs` | Resolve single-node and visual-range targets, directory coverage, stage/unstage partitioning and discard eligibility without depending on Flux. |
| `files_coordinator.rs` | Supply the active files context, apply accepted repository status and coordinate the untracked-file diff side. |
| `flux/stores/files_output.rs` | Translate feature write payloads into effects and apply logs/invalidation at the Flux boundary. |
| `files/tests.rs` | Exercise refresh/selection/operation decisions without constructing an App, repository or task runtime. |

`FilesStore` and `SelectionStore` now use `FilesAccess::reduce_files` instead of
duplicating the feature decisions. `SelectionAccess` retains only the stash-target
and all-file-path queries needed by overlay workflows. List navigation and search
selection update files through feature actions; they no longer borrow mutable
file panel state. Snapshot creation and rendering consume the immutable panel
projection. Fixture mutation helpers are compiled only under `cfg(test)`.

Requests never mutate the observed index/worktree status optimistically.
The shared effect runtime keeps its write ordering and completion refresh policy.
Selection requests require the files context, while completion events remain valid
after a panel change. Successful discard/commit preparation clears visual selection;
failed operations retain it. Discard targets omit untracked files, and partially
covered directories do not include unselected children.

## Revision Feature Ownership

`AppState::revisions: RevisionState` owns branch, commit and stash lists, their
cursors, branch history subviews, commit/stash file trees, fetch progress and the
requested history limit. `UiState` retains shared focus, dirty tracking and render
caches. Production consumers borrow `RevisionState::view()`; they cannot mutate
the feature's panel data directly.

| Feature file | Responsibility |
| --- | --- |
| `revisions/state.rs` | Own private panel data and history pagination metadata; expose shared domain reads for coordinators. |
| `revisions/render.rs` | Publish branch/commit/stash render models and borrow read-only views for renderers and selectors. |
| `revisions/update.rs` | Reduce typed navigation, tree, refresh, completion and pagination inputs; return owned read/cancel intentions and log messages. |
| `revisions/selectors.rs` | Share selected branch, commit, stash, tree node, cursor and visible-count rules across state and snapshots. |
| `revisions/tree.rs` | Preserve existing tree expansion, path-based cursor restoration and parent-list restoration. |
| `revisions/highlight.rs` | Compute the existing commit graph ancestry highlight. |
| `revisions_coordinator.rs` | Adapt feature outputs to task identities and priorities; coordinate accepted payloads with diff refresh and UI invalidation. |
| `revisions/tests.rs` | Test feature decisions without an App, repository, Flux dispatcher or task runtime. |

Shared task generations and cancellation still reject superseded work before
applying a revision payload. The feature separately checks branch names, commit
OIDs and stash identities against the current pending source. Closing a tree
cancels its task and clears its source, so a late completion cannot reopen it.
Accepted completions remain valid after a focus change. Opening dirty history
requests commits before opening its file tree; fast history stays dirty until a
full history result arrives. Pagination retains the existing threshold and step.

Input, commands, search, diff targets and both snapshot constructors use the same
read-only panel data and revision selectors. Existing fixture mutation support is
compiled only under `cfg(test)`. Golden files remain unchanged.

## Domain Cursors and Feature Render Models

`PanelState` owns only an optional selected index. Navigation and tree restoration
use its domain methods; no production application feature imports Ratatui widget
state. `PanelView` exposes selection without a mutation API.

`files/render.rs` and `revisions/render.rs` define owned published models with
private fields and borrowed views containing slices, shared references and copied
metadata. Files expose visible nodes, cursor and visual selection. Revisions expose
list entries, visible tree nodes, source identities, highlights and the branch
subview's loading indicator. Expansion sets, original tree file lists, fetch flags,
dirty history and pagination bookkeeping stay in domain state. `DiffView` and
`SearchView` retain their existing feature read contracts.

`AppStateSnapshotOwned` publishes these feature models instead of cloning whole
panel states. `as_snapshot` borrows their views without allocation or payload
cloning. Command selectors consume the same views directly from domain state or
from a published model, so navigation, command availability and diff identities
share one set of selection rules.

The UI adapter in `ui/panels/selection.rs` creates a fresh `ListState` from
`PanelView`. As before, every frame starts at offset zero and Ratatui/FileTree
calculates a local viewport using selection, area and the existing scroll padding.
Rendering never writes the calculated offset or a clamped selection back to
domain state or retained snapshots. Layout policies implement `DynamicPanel` on
render views rather than domain panel types.

Regression tests compare legacy list scrolling, retained source/cursor/highlight
metadata, selectors through navigation in all seven view contexts, borrowed and
published rendering across resizes, and tree-close restoration after widget
clamping. Allocation checks also prove that snapshots do not retain internal
tree bookkeeping or cause copy-on-write clones when that bookkeeping changes.

## Search Feature Ownership

`search/state.rs` owns the current query, matching row indices, captured view scope
and queries saved by `ViewContext`. These fields no longer live in `InputState`.
Editor mode and the editable text buffer remain application input state; accepting
or cancelling that editor is separate from changing the saved search query.

| Feature file | Responsibility |
| --- | --- |
| `search/state.rs` | Keep search data private and expose a borrowed `SearchView`; retain the existing scope key alias. |
| `search/update.rs` | Reduce explicit query/scope/data inputs, restore or clear queries and return row navigation intentions without modifying a panel. |
| `search/selectors.rs` | Project current/saved queries, query versus match availability and exact match summaries for rendering and command availability. |
| `search_coordinator.rs` | Derive view identities and searchable text from visible feature data, provide the current cursor and apply returned indices through files/revision navigation. Own search editor mode/buffer transitions. |
| `search/tests.rs` | Test matching, scope isolation, restoration, clearing, summaries and inclusive/strict wrapping jumps without AppState, Flux, Git or terminal widgets. |
| `search_coordinator/regression_tests.rs` | Exercise text adaptation, dispatcher action order, cursor routing and restoration against refreshed data. |

Scopes remain distinct for the files tree, branch list, each branch's commit
history, commit list, each commit's file tree, stash list and each stash's file
tree. Only the captured nonempty query is exposed to the matching scope's render
projection; saved queries for other scopes remain available for restoration.
Restoration recomputes indices against the newly adapted dataset and does not
move the cursor. Clearing removes only the captured scope's saved query.

The coordinator preserves the search text contract: visible tree base names with
a trailing slash for directories; branch names; short commit hashes, full author
names and messages; and `stash@{index}` plus stash messages. Hidden tree children
and parent paths are not added to that dataset. Matching remains case-insensitive
Unicode substring matching without trimming or additional normalization.

Opening the search editor still copies the current query before capturing the
view scope. Initial selection uses the first match at or after the current row;
next/previous use strict comparisons and wrap at either end. A single matching
row still returns a navigation intent even if its index is unchanged. Missing
cursors use the existing zero-index fallback; empty or mismatched scopes do not
navigate. Confirming input keeps the query and current cursor.

The Flux stores retain the existing input → query → navigation → diff-reload
action sequence, confirmation/clear log messages and dirty invalidation policy.
The feature emits no commands or effects. Empty-query application and restoration
retain the fast path that skips dataset adaptation, preserving the prior snapshot
and completion allocation budgets. Existing rendering goldens stay unchanged;
a retained-snapshot test also checks that later scope/query changes do not alter
an already published search frame.

## Remaining Coupling and Refactoring Roadmap

Store capability bounds restrict reducer dependencies, and diff/hunk, files,
revisions and search have owned feature state with pure updates. Cross-feature scheduling
remains in application coordinators. Continue with separate, behavior-preserving changes.

| Phase | Evidence in current code | Change and acceptance criteria |
| --- | --- | --- |
| 1 — complete | Every store previously received the entire state interface. | Narrow all 14 stores and shared helpers; independent reducer tests and existing integration tests pass. |
| 2 — complete | Diff, files, revision and search state, decisions and selectors have feature owners. Application coordinators apply read/cancel and navigation intentions. Search domain state is separate from editor input. | Retain pure feature tests and dispatcher/runtime regression coverage for cancellation, completion identity, search scopes, query restoration and selection restoration. |
| 3 — complete | State actions use the action path; terminal lifecycle and runtime orchestration are separate modules, with telemetry outside the entry point. | Retain production-loop and PTY regressions for FIFO routing, shutdown/error precedence, generation rejection, serialized writes and bounded reads. |
| 4 — complete | Measurements identified history/graph payload clones as the main allocation cost. Feature projections now share collections, diff text, keymap and logs; `as_snapshot` borrows render metadata without allocations. | Retain the allocation budget, snapshot isolation, diff-cache sharing, rendering goldens and stale-result rejection regressions. Re-run the opt-in profile below with the same workload before further optimization. |
| 5 — complete | `git/repository.rs` composes one backend; private capability modules own implementations and preserve worker routing. Workers and diagnostics are separate, and history owns its cache and graph projection. | Retain real temporary-repository tests for index, worktree and conflict behavior, plus controlled worker regressions for cross-capability FIFO, independent lanes, round-robin wrapping, failure recovery and fetch probe ordering. |

Use diff/hunk, files, revisions and search as the vertical feature examples for subsequent extraction:
state, reducer decisions, selectors, owned write payloads and render read models
have one feature owner. Narrow read views should reach UI components; mutable
application state and Git handles should not.

Avoid splitting into workspace crates until module dependencies have a stable
direction. The remaining `app`/`flux` type references and shared state adapters
would otherwise become crate dependency cycles. Likewise, additional generic
widget traits should have real consumers before being introduced: `ui/traits.rs`
already contains unused interactive/selection abstractions alongside the used
`DynamicPanel` trait.

Run `cargo fmt --check`, `cargo check`, `cargo test` and
`cargo clippy --all-targets --all-features -- -D warnings` for each phase.
These phases change architecture; existing golden and terminal lifecycle tests
verify rendering and shutdown behavior. End-to-end runtime performance requires
a separate benchmark; snapshot measurements below cover construction and projection only.

## Measured Snapshot Payload Reuse

`Shared<T>` in `shared.rs` holds an `Arc<T>`. Cloning a feature projection shares
its large collections; reducer mutation uses `Arc::make_mut` to detach only the
modified collection while published snapshots retain their previous contents.
Files publish shared tree nodes. Revisions publish shared item lists,
branch subview histories, tree nodes and highlight sets; original tree files and
expanded-directory sets remain internal to the features. Diff
documents share display lines as well as their existing patch `Arc`, including
through the LRU cache. Keymap, command logs and visual-selection indices are also
shared. Navigation, visible loading flags, sources, diff scroll and input metadata
remain owned by each published snapshot.

Collection replacements and tree/subview resets install fresh shared payloads;
they do not copy the old collection merely to discard it. An in-place write to a
payload retained by an older snapshot can still clone that payload once. This
tradeoff preserves snapshot isolation without version counters or a second
invalidation protocol. Mutable feature ownership and the dispatcher remain the
same; shared storage does not bypass task generation, operation epoch or selected
source checks.

`as_snapshot` borrows context, command help, shortcut hints, log entries, stash
targets, operation state and the patch. Test-only direct-state views use owned
`Cow` metadata where it must be derived; production views borrow the already
constructed snapshot. UI functions continue to consume immutable feature views.

The opt-in profile is reproducible with:

```bash
cargo test test_snapshot_profile -- --ignored --nocapture --test-threads=1
```

Measurements below were captured before and after this extraction on the same
Windows/WSL Ubuntu 24.04 host, stable Cargo 1.99.0, with the unoptimized test
profile. The harness uses a test-only `System` allocator wrapper and a thread-local
counter; fixture setup, other test threads and output are excluded. Counts include
allocations, zeroed allocations and reallocations; bytes are requested allocation
bytes, not peak live memory. Timing includes construction/projection and dropping
the result, with three warmups and the median of five batches (20 builds or 200
views per batch). Instrumented test timings are local observations, not a release
frame-rate or end-to-end latency claim.

Both fixtures contain eight graph cells per commit, eight log entries and twenty
stash target paths. The small fixture has 30 commits, 6 branch subview commits,
20 files, 100 diff lines, 10 branches and 5 stashes. The large fixture has 10,000
commits, 2,000 branch subview commits, 5,000 files, 5,000 diff lines of about 256
bytes, 500 branches and 100 stashes. File trees include one group node.

| Workload / path | Before median | After median | Allocation calls before → after | Bytes before → after |
| --- | --- | --- | --- | --- |
| Small `from_app` | 215.479 μs | 78.289 μs | 2,043 → 206 | 117,819 → 7,972 |
| Large `from_app` | 36.562 ms | 76.338 μs | 479,104 → 206 | 24,801,144 → 7,972 |
| Small `as_snapshot` | 6.151 μs | 48 ns | 84 → 0 | 3,220 → 0 |
| Large `as_snapshot` | 5.907 μs | 43 ns | 84 → 0 | 3,220 → 0 |

Isolated large-fixture clone measurements identify the allocation sources:

| Clone source | Before calls / bytes | After calls / bytes |
| --- | --- | --- |
| Files projection | 5,004 / 295,162 | 0 / 0 |
| Branch projection, including subview history | 78,502 / 3,859,780 | 0 / 0 |
| Commits projection, including graph cells | 390,001 / 19,178,890 | 0 / 0 |
| Stash projection | 101 / 4,390 | 0 / 0 |
| Diff display lines | 5,001 / 1,445,000 | 0 / 0 |
| Keymap | 280 / 9,518 | 0 / 0 |

The remaining snapshot allocations derive command availability, shortcut hints,
stash labels and small metadata; this workload no longer allocates in proportion
to history or diff size. Normal CI checks enforce zero allocations in the render
view and equal small/large snapshot allocation counts, with a bounded metadata
budget. Retained-frame tests and existing goldens verify unchanged rendering;
runtime regressions also retain snapshots across rejected old diff completions.

## Adding Commands and Views

Declare a command in `commands.rs` with its ID, aliases, binding and label, then map its intent to a domain action in `CommandSpec::action`. Availability, key collision priority, palette entries and footer hints are shared; do not add separate input or help tables. Existing keymap configuration names remain compatible. `refresh` is available in the palette and defaults to `r`.

Represent a new view in `ViewContext`, including its source identity and parent when it is a subview. The current adapters derive contexts from existing panel state rather than storing a separate focus model. Search queries are keyed by context, so branch histories and revision file trees have independent search state. `command_tests.rs` covers key/palette parity, custom and conflicting bindings, modal behavior, shared selectors, search isolation and palette rendering.

## Patches and Repository Workflows

`git::capabilities` separates worktree, patch, history, reference, commit, stash and operation interfaces. `GitRepository` is their composition boundary, with a blanket implementation; implement or consume the smallest capability needed. The unused parallel async-trait facade has been removed. Channel-based backend jobs continue to run through the effect executor.

`git/repository.rs` remains the single `Git2Repository` composition and construction
entry, re-exported by `git`. Its private modules do not expose additional backend
handles. Each capability module keeps its inner Git implementation beside its
trait methods and worker route; the public capability contracts are unchanged.

| Backend module | Ownership |
| --- | --- |
| `repository/worktree.rs` | Full/fast status, staging, unstaging and discarding paths. |
| `repository/patch.rs` | Structured patch entry points, worktree previews and scoped commit diffs; delegates exact patch application to `patch_backend.rs`. |
| `repository/history.rs`, `repository/history/graph.rs` | Commit lists, branch logs, commit file lists, worker-local reachability cache and graph projection. |
| `repository/references.rs` | Branch reads/writes, automatic stash during checkout and fetch's upstream probe. |
| `repository/stash.rs`, `repository/commit.rs` | Stash reads/mutations and commit creation. |
| `repository/operations.rs` | Workflow snapshot/control entry points; delegates workflow interpretation to `operation_backend.rs`. |
| `repository/worker.rs`, `repository/diagnostics.rs` | Persistent native workers, bounded queues, pool sizing and existing per-job timing logs. |
| `repository/shared.rs` | Signature, path, subprocess and bounded diff projection helpers. |

Worker routes remain explicit in the capability implementations:

| Lane | Calls |
| --- | --- |
| Primary FIFO worker | Full status, all worktree/index writes, structured patches and worktree previews, commit file lists, branch reads/writes, all stash reads/writes, commits, workflow snapshots/controls and the fetch upstream probe. |
| External worker pool | Fast status, full/fast commit lists, branch histories/logs, scoped commit diffs and fetch after the primary probe. |

Each worker retains its own open repository and history cache, with a queue
capacity of 128. External submissions retain atomic round-robin routing. The
default pool size remains half the available parallelism clamped to 2–6;
`RATAGIT_EXTERNAL_WORKERS` remains clamped to 1–12. Synchronous methods still wait
for their channel result; asynchronous methods retain the same enqueue behavior,
including fetch waiting for its primary upstream probe before enqueueing the
external operation. The effect executor's read concurrency and serialized write
lane are unchanged. `repository/tests/scheduling.rs` exercises these boundaries
with worker gates and real repositories, including a local bare remote for fetch.

`git::patch::Patch` owns file provenance, paths, object IDs, modes, binary markers, hunks, old/new line numbers and exact line bytes. `DiffDocument` pairs the patch with display lines; textual branch logs explicitly have no patch. Background results, the LRU cache and render snapshots preserve the same patch using `Arc`. UI text is a projection, never the source for index writes.

`ApplySelectedHunks` resolves a current patch and hunk selection into a generation-tagged write request. The backend reloads the index, rejects stale patches, checks all selected hunks together and applies the original patch bytes through `git apply --cached`; reverse application cancels staging. Modified, added, deleted and untracked text files are supported, including selections spanning directory files. Binary, truncated, renamed, mode-only and historical patches remain read only. Worktree contents remain intact. The earlier `StageHunks` backend entry point delegates to the same implementation.

`diff/state.rs` owns diff focus, the hunk cursor, explicit selections, staged/unstaged view choice and pending write identity. `diff/update.rs` reduces their transitions. `HunkView` is shared by key mapping, palette availability, hints and snapshots. Navigation scrolls the hunk header into view; the renderer highlights the cursor and selected hunks, identifies staged/unstaged content and shows the file path. Changing panels, selecting another file or refreshing a patch clears obsolete selections. In directory diffs, selecting a hunk with the opposite staging direction starts a new selection. Repeated write commands are disabled until completion; failures also trigger a fresh status and diff read.

Default Files/diff keys follow Lazygit: `Enter` opens a file's diff or toggles a directory; `0` focuses the main diff. Diff focus starts in line mode. `j`/`k` or up/down move through diff lines, `h`/`l` or left/right jump hunks, `a` switches line/hunk mode, and `v` selects a range in either mode. `Space` applies the selection to the index in the current side's direction. `Tab` switches staged/unstaged sides inside the diff and cycles side panels outside it (`Shift+Tab` cycles backward). `n`/`N` switch visible files without leaving the diff, and `Esc` returns directly to the file list and clears selection. `d` discards unstaged changes or unstages staged changes. Legacy selection and explicit stage/unstage actions remain available through palette aliases and custom bindings. The keymap loader upgrades exact historical defaults in memory, preserves customized bindings and does not rewrite configuration comments. Modal input retains priority.

Status reads report both index and worktree changes for partially staged paths. While the diff is focused, partial fast status reads do not rebuild the file tree before the complete read, preserving the selected file when reversing an addition makes it untracked again. Patch display row ranges are derived once from hunk metadata for cursor/selection rendering; scrolling does not rebuild patch text.

`git::operation::OperationMachine` separates observed workflow state (idle, in progress or conflicted, with optional rebase progress) from command state (ready, running or failed). Status reads carry the operation epoch, so old reads cannot overwrite command results. `repository_operation.rs` reduces typed control actions and completion events. Continue, skip and abort validate the actual repository state in the backend, then reload it even after failure; failed commands may have changed HEAD or conflicts. CLI operation controls suppress editor prompts and retain the existing message. A future editor flow must supply edited messages separately.

`RebasePlan` models ordered pick/reword/edit/squash/fixup/drop steps with full commit IDs and validates duplicate commits and predecessor requirements. This batch provides the plan model and controls for an existing workflow; starting or editing an interactive rebase and new keyboard commands remain future work. `git::refactor_tests` covers actual patch application, conflict workflows, state transitions and dispatcher/effect event roundtrips. `app::hunk_tests` covers keyboard and palette flows, effective help keys, rendered selections, multi-file writes, reverse application and stale completions.

