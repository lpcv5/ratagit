# App Layer Map

This folder is organized so new contributors can find behavior quickly.

## Entry Files
- `app.rs`: `App` startup composition and `AppState` transitions. The dispatcher owns the state; effects never receive it.
- `runtime.rs`: cloneable Git execution dependencies (`AppRuntime`).
- `background.rs`: owned background read requests and completion payloads.
- `background_poll.rs`: refresh scheduling and application of background completion events; it performs no Git I/O.
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
- `selection.rs`: shared `FileSelection` for staging, discard, stash and visual ranges. Input mapping contains no independent target-selection implementation.
- `hints.rs`: shortcut hints generated from command definitions and the effective keymap. Unbound or shadowed keys are omitted.
- `refresh.rs`: status refresh and file-tree rebuild/expand-collapse helpers.
- `revision_tree.rs`: shared commit/stash tree-mode lifecycle helpers.
- `diff_loader.rs`: diff target selector output -> diff loading logic.

## Data Flow
`UI Input -> Action -> Dispatcher (Flux stores) -> AppState -> AppStateSnapshot -> UI Render`

Reducers enqueue owned effect requests. `flux::runtime::EffectExecutor` runs a bounded number of reads concurrently and serializes local writes in arrival order. Git calls and receiver waits execute on blocking workers, without borrowing state or blocking the UI thread. Fetches share the read lane and can proceed independently of local writes.

`EffectRequest -> AppRuntime / GitRepository -> Completion Action -> Dispatcher -> AppState`

Background completion events carry their task generation. State rejects cancelled or superseded requests before applying a payload; changing the diff selection or requesting a refresh cancels the previous diff generation. Completed events participate in the normal dirty-state and snapshot publication path.

Test input mapping follows the same runtime path via `flux::input_mapper` and test dispatch helpers. The deterministic test driver reuses the production effect executor logic and dispatches its completion events. `flux::runtime_tests` additionally exercises the async scheduler with controlled completion gates.

## Adding Commands and Views

Declare a command in `commands.rs` with its ID, aliases, binding and label, then map its intent to a domain action in `CommandSpec::action`. Availability, key collision priority, palette entries and footer hints are shared; do not add separate input or help tables. Existing keymap configuration names remain compatible. `refresh` is available in the palette and defaults to `r`.

Represent a new view in `ViewContext`, including its source identity and parent when it is a subview. The current adapters derive contexts from existing panel state rather than storing a separate focus model. Search queries are keyed by context, so branch histories and revision file trees have independent search state. `command_tests.rs` covers key/palette parity, custom and conflicting bindings, modal behavior, shared selectors, search isolation and palette rendering.

## Patches and Repository Workflows

`git::capabilities` separates worktree, patch, history, reference, commit, stash and operation interfaces. `GitRepository` is their composition boundary, with a blanket implementation; implement or consume the smallest capability needed. The unused parallel async-trait facade has been removed. Channel-based backend jobs continue to run through the effect executor.

`git::patch::Patch` owns file provenance, paths, object IDs, modes, binary markers, hunks, old/new line numbers and exact line bytes. `DiffDocument` pairs the patch with display lines; textual branch logs explicitly have no patch. Background results, the LRU cache and render snapshots preserve the same patch using `Arc`. UI text is a projection, never the source for index writes.

`ApplySelectedHunks` resolves a current patch and hunk selection into a generation-tagged write request. The backend reloads the index, rejects stale patches, checks all selected hunks together and applies the original patch bytes through `git apply --cached`; reverse application cancels staging. Modified, added, deleted and untracked text files are supported, including selections spanning directory files. Binary, truncated, renamed, mode-only and historical patches remain read only. Worktree contents remain intact. The earlier `StageHunks` backend entry point delegates to the same implementation.

`diff_interaction.rs` owns diff focus, the hunk cursor, explicit selections, staged/unstaged view choice and pending write identity. `HunkView` is shared by key mapping, palette availability, hints and snapshots. Navigation scrolls the hunk header into view; the renderer highlights the cursor and selected hunks, identifies staged/unstaged content and shows the file path. Changing panels, selecting another file or refreshing a patch clears obsolete selections. In directory diffs, selecting a hunk with the opposite staging direction starts a new selection. Repeated write commands are disabled until completion; failures also trigger a fresh status and diff read.

Default keys in the diff are `Tab` to enter/leave, `j`/`k` or arrows (`]`/`[` also work) to navigate, `Space` to select/deselect, `s` to stage and `u` to unstage. With no explicit selection, writes use the current hunk. `Enter` switches a tracked file between staged and unstaged views. `Esc` first clears explicit selections, then returns to the list. Registry aliases are `diff focus`, `diff side`, `hunk next`, `hunk previous`, `hunk select`, `hunk stage` and `hunk unstage`; all bindings are configurable through the global keymap. Modal input retains priority.

Status reads report both index and worktree changes for partially staged paths. While the diff is focused, partial fast status reads do not rebuild the file tree before the complete read, preserving the selected file when reversing an addition makes it untracked again. Patch display row ranges are derived once from hunk metadata for cursor/selection rendering; scrolling does not rebuild patch text.

`git::operation::OperationMachine` separates observed workflow state (idle, in progress or conflicted, with optional rebase progress) from command state (ready, running or failed). Status reads carry the operation epoch, so old reads cannot overwrite command results. `repository_operation.rs` reduces typed control actions and completion events. Continue, skip and abort validate the actual repository state in the backend, then reload it even after failure; failed commands may have changed HEAD or conflicts. CLI operation controls suppress editor prompts and retain the existing message. A future editor flow must supply edited messages separately.

`RebasePlan` models ordered pick/reword/edit/squash/fixup/drop steps with full commit IDs and validates duplicate commits and predecessor requirements. This batch provides the plan model and controls for an existing workflow; starting or editing an interactive rebase and new keyboard commands remain future work. `git::refactor_tests` covers actual patch application, conflict workflows, state transitions and dispatcher/effect event roundtrips. `app::hunk_tests` covers keyboard and palette flows, effective help keys, rendered selections, multi-file writes, reverse application and stale completions.

