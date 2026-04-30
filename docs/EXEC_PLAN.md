# EXEC_PLAN.md

## Active Slice

Name: Commit History Rewrite For Pushed Commits

Status: completed

## Problem

Commit rewrite actions currently reject commits that are already pushed or
reachable from main/upstream, so users cannot intentionally rewrite existing
commit history from ratagit. The rewrite flows should support editing linear
history regardless of commit hash status while preserving the existing safety
checks for dirty worktrees, merge commits, root-parent edge cases, and backend
execution through `GitBackend`.

## Smallest Slice

- remove the AppContext-level unpushed/private precheck for commit rewrite
  actions
- remove the Git backend private/main/upstream rejection for replay-based
  history rewrites
- keep clean-worktree, staged-only amend, merge-commit, root-commit, and
  root-parent squash/fixup protections unchanged
- update docs to describe the new pushed-history behavior
- cover pushed commit rewrite in unit/backend/harness tests

## Non-Goals

- no automatic force push after rewriting public history
- no new confirmation modal in this slice
- no support for rewriting merge commits
- no UI layout or rendering changes
- no broader Git replay algorithm refactor

## Expected Files

- `docs/EXEC_PLAN.md`
- `docs/PRODUCT.md`
- `docs/DESIGN.md`
- `docs/harness/SCENARIOS.md`
- `libs/ratagit-core/src/commit_workflow.rs`
- `libs/ratagit-core/src/editor.rs`
- `libs/ratagit-core/tests/core.rs`
- `libs/ratagit-git/src/cli.rs`
- `libs/ratagit-git/src/mock.rs`
- `libs/ratagit-git/src/lib.rs`
- `libs/ratagit-git/tests/git2_tmp.rs`
- `libs/ratagit-harness/tests/harness.rs`

## Tests

- unit tests for reducers allowing pushed rewrite targets while still rejecting
  merge commits
- backend tests for mock and hybrid replay of pushed/main-reachable commits
- harness scenario asserting pushed commit rewrite updates UI and Git state

## Harness Decision

Required because this changes user-visible commit rewrite behavior and Git
state semantics. Add a focused scenario that rewords a pushed commit and asserts
the screen notice, backend operation, and rewritten Git state.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
cargo test -p ratagit --test exec_plan
```

## Completion Evidence

- removed unpushed/private prechecks from core commit rewrite and commit reword
  entry points while keeping clean-worktree and merge-commit guards
- removed backend main/upstream/private rejection from replay-based history
  rewrite commands while keeping root and merge protections
- added reducer, mock backend, hybrid Git backend, and harness coverage for
  rewriting pushed/main-reachable commits
- no UI rendering changes; no snapshot updates required
- validation passed:
  `cargo fmt`
- validation passed:
  `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed:
  `cargo test`
- validation passed:
  `cargo test -p ratagit --test exec_plan`
