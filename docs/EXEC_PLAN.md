# EXEC_PLAN.md

## Active Slice

Name: Remove Details line-number columns

Status: completed

## Problem

The user no longer wants line numbers in Details. Both passive previews and
interactive patches currently add old/new number columns and a separator.

## Smallest Slice

Remove number formatting and its separator from the shared diff row renderer.
Keep cursor/mark indicators, native diff content, background colors and stable
entry geometry. Retain source coordinates in core for patch mapping/operations.

## Non-Goals

Change GitBackend, source coordinates, selection, scrolling or loading behavior.

## Expected Files

Shared UI diff renderer; affected unit/UI/keyboard/harness assertions and
snapshots; product/design docs. No cross-layer production changes are needed.

## Tests

- Shared rows add only cursor/mark space before native diff content.
- Passive, pending and interactive frames have no number column or separator.
- Diff colors, marks, native text and stable entry geometry still pass.

## Harness Decision

Update the existing enter/leave diff scenario to assert unnumbered diff rows
and unchanged Git state. Keep existing failure artifacts and real Git coverage.

## Validation

cargo fmt, cargo fmt --check and git diff --check passed.
cargo clippy --workspace --lib --bins -- -D warnings passed.
cargo test --target-dir tmp/native-diff-build --no-fail-fast passed 606 tests.
Reviewed updated snapshots, then all 83 UI integration/snapshot tests passed
with normal comparison. Logs: tmp/details-no-numbers-update-tests.log,
tmp/details-no-numbers-snapshots.log and tmp/details-no-numbers-clippy.log.

## Completion Evidence

The shared renderer omits number formatting, number columns and their separator.
Existing native-text, color, mark, loading and geometry assertions pass. The
enter/leave harness scenario now asserts unnumbered rows and unchanged Git
state. Core coordinates and all patch operations remain covered by existing
real Git tests. Updated the passive-preview snapshot name and current docs.
