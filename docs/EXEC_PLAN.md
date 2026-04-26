# EXEC_PLAN.md

## Current Slice

Refactor existing modal rendering into a reusable internal modal system.

## Goal

- keep commit, stash, reset, and discard modal behavior unchanged
- move shared modal shell, sizing, footer, input block, and tone styling into `ratagit-ui`
- use semantic modal tones: info for editor modals, warning for reset, danger for discard
- preserve pure rendering from `AppState` and terminal size only

## Vertical Slice

1. Shared UI primitives
- add an internal `modal` module with `ModalSpec`, `ModalTone`, `ModalFrame`, layout helpers, input blocks, warning text, and action footers
- add theme helpers for info, warning, danger, muted, and footer styles

2. Existing modal migration
- update editor modals to reuse the shared shell and input block while keeping cursor placement deterministic
- update reset and discard modals to reuse the shared shell, footer, and tone helpers
- do not change reducer state, input mapping, commands, or Git backend behavior

3. Validation
- update modal snapshots and add tone style assertions
- keep reset/discard harness scenarios asserting UI, Git operation traces, and final Git state
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
