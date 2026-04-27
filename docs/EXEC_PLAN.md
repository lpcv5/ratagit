# EXEC_PLAN.md

## Current Slice

Rename the pure root state to `AppContext` and classify state
into `repo`, `ui`, and `work` without changing behavior, rendering text,
keybindings, Git command semantics, or harness expectations.

## Goal

- make `AppContext` the only public root state object
- keep `AppContext` pure: no `GitBackend`, runtime handles, environment, clock,
  or other external dependency handles
- move Git/backend-derived data into `AppContext.repo`
- move focus, selections, scroll offsets, search, projection caches, and modal
  state into `AppContext.ui`
- move pending refresh/details/operation and loading intent into
  `AppContext.work`
- keep render functions pure and keep side effects behind `Command` +
  `GitBackend`

## Vertical Slice

1. Public API rename
- replace the legacy root state name with `AppContext`
- update `update(context: &mut AppContext, action: Action) -> Vec<Command>`
- update render, runtime, test, snapshot, and harness APIs to accept
  `&AppContext`
- remove legacy root state paths without compatibility aliases

2. Categorized state model
- add `RepoState`, `UiState`, and `WorkStatusState` under `AppContext`
- split Files, Branches, Commits, Stash, Commit Files, and Details data/UI
  ownership across `repo` and `ui`
- make helper signatures accept explicit repo data and UI state when both are
  required

3. Tests and harness
- add unit coverage for `AppContext::default()` category defaults
- update regression coverage for focus/search, panel navigation, multi-select,
  modals, Details scrolling, commit pagination, and stale Details rejection
- add a harness scenario that asserts categorized UI state and Git state
- keep UI snapshots visually unchanged

4. Documentation
- update `ARCHITECTURE.md` with pure categorized `AppContext`
- update `docs/DESIGN.md` with `repo` / `ui` / `work` ownership
- do not update `docs/PRODUCT.md` because behavior is unchanged

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo check -p ratagit-core`
- `cargo check --workspace`
- `cargo test --workspace --no-run`
- `cargo test -p ratagit-harness --test harness`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
