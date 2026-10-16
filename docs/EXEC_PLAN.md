# EXEC_PLAN.md

## Active Slice

Name: Prepare feature/newarch promotion to main

Status: validated; integration pending

## Problem

The user designated feature/newarch as the authoritative implementation for
main and retired the old main implementation. The branches have diverged;
ordinary conflict resolution would reintroduce retired architecture files.
The all-target CI Clippy check also finds redundant casts in two UI tests.

## Smallest Slice

Remove the redundant test casts, validate the current implementation, and record
old main as merged using the ours merge strategy so its history remains reachable
while the feature/newarch tree stays authoritative. Push feature/newarch, create
a PR targeting main, validate its checks, and merge the reviewed head.

## Non-Goals

Restore the retired implementation, rewrite remote history, change product
behavior, delete branches, or alter another worktree's local main checkout.

## Expected Files

docs/EXEC_PLAN.md and the two UI integration tests flagged by Clippy. The
integration merge must preserve the validated feature/newarch tree exactly.

## Tests

Run formatting, locked compilation, production and all-target strict Clippy,
and locked workspace tests including existing snapshots and harness scenarios.
Check that the integration merge tree equals its first parent's tree, old main
is an ancestor, the worktree is clean, and the PR merges the validated head.
Existing tests cover the cast cleanup; no new tests are needed.

## Harness Decision

No new scenario: product rendering, input handling, and backend Git semantics
are unchanged. Existing harness and real Git tests validate the adopted version.

## Validation

Planned: cargo fmt; cargo fmt --check; cargo check --locked;
cargo clippy --workspace --lib --bins -- -D warnings;
cargo clippy --locked --all-targets --all-features -- -D warnings;
cargo test --locked; git diff --check; merge ancestry/tree checks.
PR CI and final delivery are tracked in the pull request.

## Completion Evidence

Formatting, locked compilation, production and all-target strict Clippy passed.
cargo test --locked passed 606 tests with zero failures or ignored tests,
including snapshots and harness coverage. Log: tmp/promote-newarch-tests.log.
The integration merge and its ancestry/tree checks remain pending.
