# App Layer Map

This folder is organized so new contributors can find behavior quickly.

## Entry Files
- `app.rs`: `App` state model and state-manipulation methods.
- `message.rs`: TEA message definitions.
- `update.rs`: top-level message router.
- `command.rs`: command abstraction.
- `tests/update_tests.inc`: update-layer behavioral tests and mock repository setup (included by `update.rs` under `#[cfg(test)]`).

## Domain Modules
- `panel_nav.rs`: active panel counting and list navigation behavior.
- `input_mode.rs`: commit/branch/stash editor input state transitions.
- `selection.rs`: visual selection, batch stage/unstage target extraction, and selection-scoped commit/stash helpers.
- `selectors.rs`: active-panel selection helpers (selected file/branch/commit/stash and diff target selection).
- `hints.rs`: shortcut hint composition and key-display mapping.
- `refresh.rs`: status refresh and file-tree rebuild/expand-collapse helpers.
- `revision_tree.rs`: shared commit/stash tree-mode lifecycle helpers.
- `diff_loader.rs`: diff target selector output -> diff loading logic.
- `update_handlers/`: message handlers grouped by domain:
  - `navigation.rs`
  - `revision.rs`
  - `staging.rs`
  - `stash.rs`
  - `branch.rs`
  - `commit.rs`
  - `quit.rs`

## Data Flow
`UI -> Message -> update.rs -> update_handlers/* -> App -> GitRepository`
