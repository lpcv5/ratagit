# EXEC_PLAN.md

## Active Slice

Name: Exec Plan V2 Discipline

Status: completed

## Problem

`EXEC_PLAN.md` has been used too often as a completion log instead of a
pre-change scope contract. The current file also mixes the active slice with a
long historical phase tracker, which makes it easy for agents to update results
after implementation without using the plan to control scope.

## Smallest Slice

- make this file the active-slice entry point only
- add a required, agent-readable template for future work
- add a lightweight repository test that fails when the active slice is missing
  required fields
- update agent/development-loop docs so plan updates happen before the first
  code edit for non-trivial work

## Non-Goals

- no product behavior changes
- no UI, Git, runtime, or harness behavior changes
- no new harness scenario, because this is repository-process infrastructure
- no broad rewrite of historical phase notes

## Expected Files

- `AGENTS.md`
- `docs/DEVELOPMENT_LOOP.md`
- `docs/EXEC_PLAN.md`
- `docs/exec-plans/README.md`
- `tests/exec_plan.rs`

## Tests

- add a repository-level test that reads `docs/EXEC_PLAN.md`
- require the active slice fields used by agents before implementation
- require completed slices to include completion evidence and validation notes
- require dirty worktree changes to include an active exec plan update

## Harness Decision

No harness scenario is needed. This slice changes process documentation and a
repository-level guard test only; it does not change rendered UI, user input,
Git state semantics, or runtime behavior.

## Validation

```text
cargo fmt
cargo test --test exec_plan
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

## Completion Evidence

- replaced `docs/EXEC_PLAN.md` with an active-slice-only template
- documented historical planning notes under `docs/exec-plans/README.md`
- updated `AGENTS.md` and `docs/DEVELOPMENT_LOOP.md` so non-trivial work must
  update the active plan before implementation edits
- added `tests/exec_plan.rs` to require agent-readable plan fields, completed
  slice evidence, and active-plan updates alongside dirty worktree changes
- validation passed: `cargo fmt`
- validation passed: `cargo test --test exec_plan`
- validation passed: `cargo clippy --workspace --lib --bins -- -D warnings`
- validation passed: `cargo test`
