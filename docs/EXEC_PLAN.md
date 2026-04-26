# EXEC_PLAN.md

## Current Slice

Generalize `/` search across left panels and subpanels.

## Goal

- keep search state in `AppState` as a generic panel-scoped model
- support Files, Branches, Commits, Stash, and Commit Files
- keep rendering pure and deterministic
- match case-insensitive text from the visible row identity
- let `Enter`, `n`, and `N` move selection through matches
- refresh Details after search navigation when the focused panel owns a Details projection
- keep `/` search available as a baseline key without listing it in the normal shortcut bar

## Vertical Slice

1. State and input
- add generic search scope/state under `AppState`
- replace Files-only search actions with generic search actions
- map `/`, search text input, `Enter`, `Esc`, `n`, and `N` for all searchable left contexts

2. Rendering
- highlight search matches in all searchable panels from `AppState.search`
- preserve the `search: <query>` input bar while typing
- leave normal bottom shortcuts focused on panel-specific common actions only

3. Validation
- cover reducer behavior for all search scopes
- update UI snapshots and panel unit tests
- add harness scenarios for normal left-panel search and Commit Files search
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
