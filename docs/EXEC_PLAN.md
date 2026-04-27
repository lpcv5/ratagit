# EXEC_PLAN.md

## Current Slice

Unify Files and Commit Files tree rendering around explicit Git status markers.

## Goal

- keep Files and Commit Files rendering pure and deterministic
- keep Git-derived file status in `AppContext` as the only source of truth
- show status characters in both file trees instead of workspace file-state icons
- color all Commit Files status markers, not only added/deleted markers
- render staged workspace filenames in green and append `U` for conflicted files

## Vertical Slice

1. Core state and Git data
- extend workspace file entries with display status and conflict metadata
- parse porcelain and git2 status into explicit status values
- preserve existing staged/untracked booleans for staging and diff behavior

2. UI and input
- reuse one file-tree renderer for Files and Commit Files
- render workspace file markers as `A/M/D/R/C/T/?` status characters
- color staged workspace filenames and all Commit Files status markers
- append and color a `U` suffix for conflicted workspace files

3. Tests and harness
- add status parser and UI style coverage
- update UI snapshots for status-character file rows
- update harness scenarios affected by file marker text

4. Documentation
- update `docs/PRODUCT.md` and `docs/DESIGN.md` because behavior and design change

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
