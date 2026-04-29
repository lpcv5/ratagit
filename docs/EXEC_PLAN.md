# EXEC_PLAN.md

## Active Slice

Name: Modal Input Border Focus

Status: completed

## Problem

Commit and stash editor modals highlight the active input field by applying a
background color to the input content. Input focus should be communicated by the
field border only, so typed text keeps the same background as inactive inputs.

## Smallest Slice

- remove active-input content background styling from the shared modal input
  block
- preserve active border/title highlighting for commit, stash, and branch-name
  input modals
- add focused buffer-style coverage for active editor input content and borders
- update product/design docs for the border-only input focus behavior

## Non-Goals

- no AppContext, reducer, input-routing, command, runtime, or Git changes
- no modal geometry, cursor positioning, text wrapping, or footer changes
- no changes to selectable choice-list highlighting

## Expected Files

- `docs/EXEC_PLAN.md`
- `docs/DESIGN.md`
- `docs/PRODUCT.md`
- `libs/ratagit-ui/src/theme.rs`
- `libs/ratagit-ui/tests/snapshots.rs`

## Tests

- assert active commit editor field text has no focus background
- assert active stash editor field text has no focus background
- assert active input field borders/titles still use the modal tone style

## Harness Decision

No new harness scenario is needed. Existing editor modal harness scenarios cover
opening and cursor behavior; this slice changes only buffer styles, which are
covered by focused UI tests.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

## Completion Evidence

- removed the active input content background from the shared modal input block
- preserved modal tone highlighting on active input borders and titles
- added buffer-style coverage for commit subject and stash title text keeping
  reset backgrounds while active
- updated product and design docs for border-only input focus
- validation passed: `cargo fmt`
- validation passed: `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed: `cargo test`
