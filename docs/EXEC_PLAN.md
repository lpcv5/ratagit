# EXEC_PLAN.md

## Current Slice

Test coverage hardening.

## Goal

- increase trustworthy regression coverage without changing product behavior
- prefer risk-based tests over percentage-only coverage padding
- keep rendering pure and AppState as the only source of truth
- cover real Git mutations only in isolated repositories under `tmp/`
- keep UI assertions deterministic through fixtures, style assertions, and snapshots
- add main-entry smoke coverage without abstracting the terminal event loop

## Vertical Slice

1. Input mapping
- cover modal priority paths for branch delete, branch rebase, auto-stash, reset,
  discard, editor, branch-create, and search modes
- cover global navigation, refresh, panel focus, quit, and ignored keys

2. Git backend risk paths
- add real Git integration tests for discard, reset soft/mixed, stash defaults,
  auto-stash checkout/rebase, and rename commit-file diffs
- add shared mock backend tests for clone-visible state and operations

3. Runtime and observability
- cover async runtime debounce flushing, render smoke paths, and worker failure
  reporting where practical
- cover observability default path and tmp log initialization/append behavior

4. Main entry smoke
- split only small private helpers needed to test backend selection and initial
  runtime construction
- avoid testing crossterm event-loop internals directly

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
- run `cargo llvm-cov --workspace --summary-only`
