# EXEC_PLAN.md

## Current Slice

Compact single-child directory chains in Files and Commit Files tree rows.

## Goal

- keep Files and Commit Files rendering pure and deterministic
- keep Git-derived file paths and tree projection rows in `AppContext` as the only source of truth
- collapse directory chains that contain only one child at each level into one displayed row
- preserve real row paths for selection, search, details diffs, staging, and Commit Files pathspecs

## Vertical Slice

1. Core state and projection
- derive compact display names while building tree rows
- keep row keys and descendants based on the real full paths
- apply the same compaction to Files and Commit Files projections

2. UI and input
- render compact directory names through the existing shared file-tree renderer
- keep expand/collapse, multi-select, search, Details, and Git operations path-based

3. Tests and harness
- add unit coverage for compact paths and non-compact branching directories
- update UI snapshots affected by compact directory names
- add or update a harness scenario asserting UI and Git state for compact paths

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
