# EXEC_PLAN.md

## Active Slice

Name: Phase 4 Refactor Review And Validation

Status: completed

## Problem

The multi-phase redundancy refactor has modified shared selection state,
left-view routing, input key helpers, and modal choice rendering. Before final
validation, the codebase needs a read-only review for remaining stale patterns,
unhelpful abstractions, clippy-exposed redundancy, and line-count impact.

## Smallest Slice

- run a targeted duplicate/stale-pattern search
- review the new `LinearListSelection`, `ActiveLeftView`, input key helpers,
  and choice menu helper for clarity and net value
- apply only mechanical fixes if clippy or review finds dead code or obvious
  redundancy
- run final full validation

## Non-Goals

- no public behavior changes
- no UI rendering changes
- no new abstraction families
- no broad code movement
- no optional behavior or UX changes

## Expected Files

- `docs/EXEC_PLAN.md`
- source files only if review finds mechanical cleanup
- `docs/REFACTORING_TODO.md` if completion notes need updating

## Tests

- full formatting, clippy, tests, and exec plan validation pass

## Harness Decision

No new harness scenario expected unless review finds an uncovered user-visible
routing change. This phase is review and final validation.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
cargo test -p ratagit --test exec_plan
```

## Completion Evidence

- Worker D reviewed stale menu actions, confirm state access, selection helper
  duplication, branch/main commit-files routing, and modal key mappings
- fixed a stale commit-files shortcut footer discovered during review
- updated affected terminal snapshots for commit-files shortcut footer changes
- validation passed:
  `cargo test -p ratagit-ui --test snapshots terminal_snapshot_commit_files`
- validation passed: `cargo fmt`
- validation passed: `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed: `cargo test`
