# App Layer Map

This folder is organized so new contributors can find behavior quickly.

## Entry Files
- `app.rs`: `App` state model and state-manipulation methods.
- `command.rs`: command abstraction.
- `test_dispatch.rs`: test helpers:
  - `dispatch_test_action(app, action)` dispatches one `DomainAction` through Flux.
  - `map_test_key(app, key)` maps key input via `flux::input_mapper`.
  - `dispatch_test_key(app, key)` maps and dispatches key input through Flux.
- `update_tests.rs`: behavior regression tests (included by `app/mod.rs` under `#[cfg(test)]`).

## Domain Modules
- `panel_nav.rs`: active panel counting and list navigation behavior.
- `input_mode.rs`: commit/branch/stash editor input state transitions.
- `selection.rs`: visual selection, batch stage/unstage target extraction, and selection-scoped commit/stash helpers.
- `search.rs`: in-panel search scope, query matching, and vim-style next/prev jumps.
- `selectors.rs`: active-panel selection helpers (selected file/branch/commit/stash and diff target selection).
- `hints.rs`: shortcut hint composition and key-display mapping.
- `refresh.rs`: status refresh and file-tree rebuild/expand-collapse helpers.
- `revision_tree.rs`: shared commit/stash tree-mode lifecycle helpers.
- `diff_loader.rs`: diff target selector output -> diff loading logic.

## Data Flow
`UI Input -> Action -> Dispatcher (Flux stores) -> App -> AppStateSnapshot -> UI Render`

Test input mapping follows the same runtime path via `flux::input_mapper` and test dispatch helpers.

