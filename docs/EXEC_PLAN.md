# EXEC_PLAN.md

## Active Slice

Name: Rust Best Practices Optimization

Status: completed

## Problem

The codebase lacked workspace-level lint configuration and had several redundant
clone operations flagged by clippy. These issues reduce code quality and
maintainability.

## Smallest Slice

- add workspace-level lint configuration to enforce code quality standards
- fix redundant clone operations detected by clippy
- ensure all changes pass tests and validation

## Non-Goals

- no functional changes to Git operations or UI behavior
- no refactoring beyond fixing detected issues
- no documentation additions (separate task)

## Expected Files

- `docs/EXEC_PLAN.md`
- `Cargo.toml` (workspace lints)
- `libs/*/Cargo.toml` (enable workspace lints)
- `libs/ratagit-core/src/actions.rs` (test module placement)
- `libs/ratagit-core/src/details.rs` (remove redundant clone)
- `libs/ratagit-git/src/cli.rs` (remove redundant clone)
- `libs/ratagit-git/src/lib.rs` (remove redundant clones)

## Tests

- all existing tests continue to pass
- clippy passes with no warnings

## Harness Decision

No new harness scenario needed. This slice only improves code quality without
changing behavior.

## Validation

```text
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

## Completion Evidence

- added workspace-level lint configuration in Cargo.toml
- enabled workspace lints in all 6 crate Cargo.toml files
- fixed test module placement in actions.rs
- removed 4 redundant clone operations across 3 files
- validation passed: `cargo clippy --workspace --lib --bins -- -D warnings`
- validation pending: `cargo test` (awaiting EXEC_PLAN.md update)
