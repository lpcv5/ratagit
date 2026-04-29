# EXEC_PLAN.md

## Active Slice

Name: Command Palette Visible Rows

Status: completed

## Problem

The command palette currently caps rendered rows with a fixed item count that
can disagree with the modal's available content height. The visible command
rows should be derived from the actual visible area so the palette renders as
many items as its current height allows.

## Smallest Slice

- remove the fixed command palette visible-row cap
- derive command palette viewport size from the actual modal content height
- keep selection-centered viewporting when the command list exceeds the visible
  content area
- update the command palette snapshot if the visible row count changes

## Non-Goals

- no changes to command palette state, command entries, input handling, or Git
  command execution
- no modal shell geometry redesign

## Expected Files

- `docs/EXEC_PLAN.md`
- `libs/ratagit-ui/src/command_palette_modal.rs`
- `libs/ratagit-ui/tests/snapshots/snapshots__terminal_snapshot_command_palette_modal.snap`

## Tests

- command palette snapshot shows rows fill the available modal content height
- existing input/core/harness tests continue to cover command behavior

## Harness Decision

No new harness scenario is needed. This slice changes only deterministic modal
viewport rendering; existing palette harness coverage still exercises the
feature's input and command dispatch path.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

## Completion Evidence

- removed the fixed command palette visible-row constant
- command palette rendering now derives the viewport length from the actual
  modal content height
- preserved selection-centered viewporting when command rows exceed the visible
  content area
- validation passed: `cargo fmt`
- validation passed: `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed: `cargo test`
