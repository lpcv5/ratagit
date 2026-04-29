# EXEC_PLAN.md

## Active Slice

Name: Modal Height Three Fifths

Status: completed

## Problem

After widening and centering modals, fullscreen modal height still needs to feel
more substantial. The shared modal shell should make the modal outer frame use
three fifths of the terminal height instead of content-driven heights.

## Smallest Slice

- update shared modal geometry so modal outer height targets 3/5 of the
  terminal area
- preserve minimum-height and small-terminal clamping behavior
- keep the existing centered placement so the taller modal remains centered
- refresh deterministic UI snapshots and cursor assertions affected by the
  taller modal
- update design documentation for modal height behavior

## Non-Goals

- no AppContext, reducer, command, runtime, or Git behavior changes
- no business-modal API changes
- no changes to modal width behavior

## Expected Files

- `docs/EXEC_PLAN.md`
- `docs/DESIGN.md`
- `libs/ratagit-ui/src/modal.rs`
- `libs/ratagit-ui/tests/snapshots.rs`
- `libs/ratagit-ui/tests/snapshots/*.snap`
- `libs/ratagit-harness/tests/harness.rs`

## Tests

- update focused modal geometry unit tests to assert 3/5 height at normal and
  fullscreen terminal sizes
- refresh affected modal snapshots
- update cursor/harness assertions that depend on modal vertical geometry

## Harness Decision

No new harness scenario is needed. The existing fullscreen discard confirmation
scenario covers user-visible modal rendering at wide/fullscreen size, and this
slice changes only shared modal geometry.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

## Completion Evidence

- updated shared modal geometry so modal outer frames target three fifths of
  the terminal height
- preserved centered placement, width behavior, and small-terminal clamps
- updated focused modal geometry tests for the new 3/5 height behavior
- refreshed affected modal snapshots, including the fullscreen modal snapshot
- updated UI and harness cursor assertions affected by the taller shared modal
- updated design documentation for three-fifths modal height
- validation passed: `cargo fmt`
- validation passed: `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed: `cargo test`
