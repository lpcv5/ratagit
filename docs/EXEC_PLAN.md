# EXEC_PLAN.md

## Active Slice

Name: Rebase-Based Commit History Rewrite

Status: completed

## Problem

Commit delete, squash, fixup, and reword currently rewrite history with a
manual `reset --hard` plus `cherry-pick` replay path. These user-facing history
rewrite actions should use Git's own interactive rebase machinery so their
backend behavior is aligned with Git's native rewrite model while preserving the
existing AppContext commands, UI behavior, safety checks, and harness-visible
state changes.

## Smallest Slice

- replace the manual replay implementation for delete/squash/fixup/reword with
  scripted `git rebase -i`
- keep public `Command` and `GitBackendHistoryRewrite` interfaces unchanged
- add internal per-command Git environment support for rebase editor scripts
- preserve clean-worktree, root-commit, root-parent squash/fixup, and
  merge-commit protections
- update product/design docs to describe rebase-backed commit rewrites
- add focused backend planner tests and keep existing harness scenarios passing

## Non-Goals

- no UI changes or snapshot updates
- no automatic force-push after rewriting pushed history
- no support for rewriting merge commits
- no support for root-parent squash/fixup via `git rebase --root`
- no changes to staged amend behavior in this slice

## Expected Files

- `docs/EXEC_PLAN.md`
- `docs/PRODUCT.md`
- `docs/DESIGN.md`
- `libs/ratagit-git/src/cli.rs`
- `libs/ratagit-git/tests/git2_tmp.rs`

## Tests

- unit tests for rebase todo planning, including delete, squash, fixup, reword,
  multi-select ordering, non-contiguous selections, root rejection, root-parent
  squash/fixup rejection, and merge rejection
- real Git backend tests proving the existing linear rewrite scenarios still
  work through the new rebase path
- existing core and harness tests remain the coverage for unchanged UI and
  command behavior

## Harness Decision

No new harness scenario is required because the user-visible command surface,
screen notices, and Git state outcomes should remain unchanged. Existing
commit rewrite harness scenarios continue to assert UI and Git state for squash,
fixup, reword, pushed reword, and delete.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
cargo test -p ratagit --test exec_plan
```

## Completion Evidence

- replaced commit delete/squash/fixup/reword's manual reset/cherry-pick replay
  path with scripted `git rebase -i`
- added per-command Git environment support for temporary sequence/message
  editor scripts, including Git for Windows-compatible shell script paths
- added rebase planner unit tests and a real Git non-contiguous squash scenario
- updated product/design docs to describe rebase-backed commit rewrites
- validation passed:
  `cargo fmt`
- validation passed:
  `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed:
  `cargo test`
- validation passed:
  `cargo test -p ratagit --test exec_plan`
