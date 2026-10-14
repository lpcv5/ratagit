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
- `app.rs`: `App` startup composition and `AppState` transitions. The dispatch loop owns the state; effects never receive it.
- `runtime.rs`: cloneable Git execution dependencies (`AppRuntime`).
- `background.rs`: owned background read requests and completion payloads.
- `background_poll.rs`: refresh scheduling and application of background completion events; it performs no Git I/O.
- `diff_coordinator.rs`: translates Flux actions into diff feature inputs and applies
  owned write requests, log messages and refresh intentions; schedules and cancels runtime tasks.
- `files_coordinator.rs`: adapts file-feature updates and immutable selection views;
  coordinates accepted status results with the diff feature.
- `legacy_test_helpers.rs`: synchronous compatibility helpers for older unit tests, excluded from production.
- `command.rs`: reducer output commands and effect requests.
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
- `search.rs`: in-panel search scope, query matching, and vim-style next/prev jumps.
- `selectors.rs`: shared immutable `SelectionView` over reducer state or a render snapshot; revision selection, list navigation and diff targets use the same context.
- `files/selectors.rs`: shared `FileSelection` for staging, discard, stash and visual ranges.
  Selectors return typed operation targets; command mapping translates them into Flux actions.
- `hints.rs`: shortcut hints generated from command definitions and the effective keymap. Unbound or shadowed keys are omitted.
- `files/state.rs`: repository status, file-tree rebuilding and cursor/anchor restoration.
- `revision_tree.rs`: shared commit/stash tree-mode lifecycle helpers.
- `diff/`: owned diff/hunk state, pure updates, read-only selectors and cache/load decisions.

## Data Flow
`UI Input -> Action -> Dispatcher (Flux stores) -> AppState -> AppStateSnapshot -> UI Render`

Reducers enqueue owned effect requests. `flux::runtime::EffectExecutor` runs a bounded number of reads concurrently and serializes local writes in arrival order. Git calls and receiver waits execute on blocking workers, without borrowing state or blocking the UI thread. Fetches share the read lane and can proceed independently of local writes.

`EffectRequest -> AppRuntime / GitRepository -> Completion Action -> Dispatcher -> AppState`

Background completion events carry their task generation. State rejects cancelled or superseded requests before applying a payload; changing the diff selection or requesting a refresh cancels the previous diff generation. Completed events participate in the normal dirty-state and snapshot publication path.

Test input mapping follows the same runtime path via `flux::input_mapper` and test dispatch helpers. The deterministic test driver reuses the production effect executor logic and dispatches its completion events. `flux::runtime_tests` additionally exercises the async scheduler with controlled completion gates.

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
2. Return owned commands and invalidation; execute Git I/O through effects.
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

## Remaining Coupling and Refactoring Roadmap

Store capability bounds now restrict reducer dependencies, and diff/hunk and files
have owned feature state with pure updates. Revision views still use shared
`AppState` helpers. Continue with separate, behavior-preserving changes.

| Phase | Evidence in current code | Change and acceptance criteria |
| --- | --- | --- |
| 1 — complete | Every store previously received the entire state interface. | Narrow all 14 stores and shared helpers; independent reducer tests and existing integration tests pass. |
| 2 — diff/hunk and files complete; revisions next | Diff decisions live in `diff/`; status, file trees, selection and file-operation decisions live in `files/`. Revision transitions still extend `AppState`. | Apply the same ownership boundary to revisions. Keep cross-feature transitions in an application coordinator. A feature test must initialize only that feature and explicit collaborators. |
| 3 — runtime shell | `main.rs` combines terminal lifecycle, logging/performance counters, UI input, dispatch, effect and refresh loops. `EffectRequest` includes both Git work and state-only events. | Extract terminal/session management and runtime orchestration; move state-only scheduling messages to the action path with explicit ordering. Preserve shutdown restoration, generation rejection, serialized writes and bounded reads. |
| 4 — read models and measured performance | `AppStateSnapshotOwned::from_app` clones panel state, diff lines, keymap and logs whenever a dirty snapshot is published. `as_snapshot` also clones command/help data. | Measure snapshot construction and allocations with large histories/diffs first. Introduce per-feature read models and shared immutable payloads or versioned reuse where justified. Preserve golden output and stale-result rejection; demonstrate reduced allocations or latency with the same workload. |
| 5 — backend implementation modules | Git capability traits and separate patch/operation backends exist, but `git/repository.rs` still combines workers, status, history, references and stash implementations. | Separate implementations along existing capability boundaries while retaining one backend composition point. Keep real temporary-repository tests for index, worktree and conflict behavior. |

Use diff/hunk and files as the vertical feature examples for subsequent extraction:
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
verify rendering and shutdown behavior. No runtime speedup is claimed without a benchmark.

## Adding Commands and Views

Declare a command in `commands.rs` with its ID, aliases, binding and label, then map its intent to a domain action in `CommandSpec::action`. Availability, key collision priority, palette entries and footer hints are shared; do not add separate input or help tables. Existing keymap configuration names remain compatible. `refresh` is available in the palette and defaults to `r`.

Represent a new view in `ViewContext`, including its source identity and parent when it is a subview. The current adapters derive contexts from existing panel state rather than storing a separate focus model. Search queries are keyed by context, so branch histories and revision file trees have independent search state. `command_tests.rs` covers key/palette parity, custom and conflicting bindings, modal behavior, shared selectors, search isolation and palette rendering.

## Patches and Repository Workflows

`git::capabilities` separates worktree, patch, history, reference, commit, stash and operation interfaces. `GitRepository` is their composition boundary, with a blanket implementation; implement or consume the smallest capability needed. The unused parallel async-trait facade has been removed. Channel-based backend jobs continue to run through the effect executor.

`git::patch::Patch` owns file provenance, paths, object IDs, modes, binary markers, hunks, old/new line numbers and exact line bytes. `DiffDocument` pairs the patch with display lines; textual branch logs explicitly have no patch. Background results, the LRU cache and render snapshots preserve the same patch using `Arc`. UI text is a projection, never the source for index writes.

`ApplySelectedHunks` resolves a current patch and hunk selection into a generation-tagged write request. The backend reloads the index, rejects stale patches, checks all selected hunks together and applies the original patch bytes through `git apply --cached`; reverse application cancels staging. Modified, added, deleted and untracked text files are supported, including selections spanning directory files. Binary, truncated, renamed, mode-only and historical patches remain read only. Worktree contents remain intact. The earlier `StageHunks` backend entry point delegates to the same implementation.

`diff/state.rs` owns diff focus, the hunk cursor, explicit selections, staged/unstaged view choice and pending write identity. `diff/update.rs` reduces their transitions. `HunkView` is shared by key mapping, palette availability, hints and snapshots. Navigation scrolls the hunk header into view; the renderer highlights the cursor and selected hunks, identifies staged/unstaged content and shows the file path. Changing panels, selecting another file or refreshing a patch clears obsolete selections. In directory diffs, selecting a hunk with the opposite staging direction starts a new selection. Repeated write commands are disabled until completion; failures also trigger a fresh status and diff read.

Default keys in the diff are `Tab` to enter/leave, `j`/`k` or arrows (`]`/`[` also work) to navigate, `Space` to select/deselect, `s` to stage and `u` to unstage. With no explicit selection, writes use the current hunk. `Enter` switches a tracked file between staged and unstaged views. `Esc` first clears explicit selections, then returns to the list. Registry aliases are `diff focus`, `diff side`, `hunk next`, `hunk previous`, `hunk select`, `hunk stage` and `hunk unstage`; all bindings are configurable through the global keymap. Modal input retains priority.

Status reads report both index and worktree changes for partially staged paths. While the diff is focused, partial fast status reads do not rebuild the file tree before the complete read, preserving the selected file when reversing an addition makes it untracked again. Patch display row ranges are derived once from hunk metadata for cursor/selection rendering; scrolling does not rebuild patch text.

`git::operation::OperationMachine` separates observed workflow state (idle, in progress or conflicted, with optional rebase progress) from command state (ready, running or failed). Status reads carry the operation epoch, so old reads cannot overwrite command results. `repository_operation.rs` reduces typed control actions and completion events. Continue, skip and abort validate the actual repository state in the backend, then reload it even after failure; failed commands may have changed HEAD or conflicts. CLI operation controls suppress editor prompts and retain the existing message. A future editor flow must supply edited messages separately.

`RebasePlan` models ordered pick/reword/edit/squash/fixup/drop steps with full commit IDs and validates duplicate commits and predecessor requirements. This batch provides the plan model and controls for an existing workflow; starting or editing an interactive rebase and new keyboard commands remain future work. `git::refactor_tests` covers actual patch application, conflict workflows, state transitions and dispatcher/effect event roundtrips. `app::hunk_tests` covers keyboard and palette flows, effective help keys, rendered selections, multi-file writes, reverse application and stale completions.

