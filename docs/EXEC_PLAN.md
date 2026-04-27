# EXEC_PLAN.md

## Current Slice

Add confirmation modals before Git operations that can discard unrecoverable
worktree data or remove remote branches.

## Goal

- keep confirmations in `AppContext.ui` as the only source of truth
- keep rendering pure and deterministic
- require explicit confirmation for `reset --hard`, `nuke`, and remote branch deletion
- preserve existing force push, force branch delete, discard, and auto-stash flows
- avoid extra confirmation for recoverable operations such as pull, normal push,
  checkout, rebase, and private commit rewrites

## Vertical Slice

1. Core state and reducer
- add reset danger confirmation state for `hard` and `Nuke`
- add branch delete confirmation state for remote and local+remote deletes
- confirm actions emit the existing Git commands; cancel actions leave Git state unchanged

2. UI and input
- render danger confirmation modals using existing modal primitives
- route Enter/Esc to confirm/cancel while a confirmation is active

3. Tests and harness
- add reducer coverage for confirm/cancel behavior
- add UI snapshot coverage for hard reset, nuke, remote delete, and local+remote delete confirmations
- add harness scenarios asserting no Git mutation before confirmation and mutation after confirmation

4. Documentation
- update `docs/PRODUCT.md` because behavior changes

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
