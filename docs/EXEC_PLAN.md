# EXEC_PLAN.md

## Current Slice

Branches visual multi-select: finish the AppState-owned multi-select interface
for the remaining left-side list panel so Files, Branches, Commits, and Commit
Files can enter visual selection with `v` and leave it with `Esc`.

## Goal

- keep visual multi-select state owned by `AppState` for Files, Branches,
  Commits, and Commit Files
- make `v` enter multi-select only, not toggle it off
- make `Esc` leave Branches visual multi-select
- keep generic `v` omitted from focused-panel shortcut rows
- preserve pure rendering and Git side effects through existing commands

## Vertical Slice

1. Input and reducer behavior
- add explicit enter/exit actions for Branches visual multi-select
- map `v` to enter visual multi-select for Branches normal mode
- map `Esc` to exit visual multi-select for Branches multi-select mode
- leave Git command behavior unchanged

2. Tests
- add input unit coverage for Branches `v` entry and `Esc` exit
- add reducer unit coverage that Branches movement extends visual selection and
  exit clears it
- add snapshot coverage for Branches batch-selected rows
- add harness scenario that asserts Branches UI and Git state

3. Documentation
- update PRODUCT Branches behavior notes
- update DESIGN Branches visual-mode notes

4. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo test -p ratagit-core`
- `cargo test --bin ratagit input::input_tests`
- `cargo test -p ratagit-ui terminal_buffer_highlights_marked_branches_with_batch_style`
- `cargo test -p ratagit-ui --test snapshots`
- `cargo test -p ratagit-harness --test harness harness_branches_visual_multiselect_marks_rows`
- `cargo test -p ratagit-harness --test harness`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
