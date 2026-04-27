# EXEC_PLAN.md

## Current Slice

Add global repository sync shortcuts: `p` pulls, `P` pushes, and a rejected
non-fast-forward push opens an explicit force-push confirmation.

## Goal

- expose pull and push through `Command` + `GitBackend`
- keep shortcut dispatch pure and state-driven
- require user confirmation before force pushing after a divergent remote error
- refresh repo state after successful sync operations

## Vertical Slice

1. Sync commands
- add pull and push commands/results
- implement CLI, hybrid, mock, and boxed backend behavior
- record operation labels and pending state

2. Force-push confirmation
- add pure UI state for force-push confirmation
- open confirmation only for non-fast-forward/divergent push failures
- confirm with Enter, cancel with Esc

3. Tests and harness
- add unit coverage for key mapping, command metadata, reducer behavior, and
  push divergence detection
- add UI snapshot coverage for the confirmation modal
- add harness scenarios for pull/push and force-push confirmation

4. Documentation
- update `docs/PRODUCT.md` because behavior changes

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
