# EXEC_PLAN.md

## Current Slice

Select-list modal viewport cap: adjust the shared choice-list body so modal
select lists render at most ten visible items.

## Goal

- keep select-list sizing centralized in the shared modal body helper
- show all options for short modal select lists while capping the visible
  viewport at ten items for future longer lists
- keep existing modal behavior, keyboard flows, and AppState ownership unchanged
- preserve pure rendering and `AppState` as the only source of truth

## Vertical Slice

1. Shared select-list sizing
- derive choice-list height from the number of AppState-provided choices in the
  shared modal helper
- derive choice-menu modal shell height from the capped list viewport so short
  lists are not clipped
- cap the rendered list viewport at ten visible item rows plus borders
- leave reducers and Git commands unchanged

2. Tests
- add a unit assertion for the shared ten-item cap
- update modal snapshots whose short lists now show all choices
- run existing harness scenarios to prove modal workflows still preserve Git
  state

3. Documentation
- update DESIGN modal notes for the select-list viewport rule
- update PRODUCT only if visible behavior semantics change

4. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo test -p ratagit-ui --test snapshots`
- `cargo test -p ratagit-harness --test harness harness_files_reset_menu_select_list_renders_all_short_choices`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
