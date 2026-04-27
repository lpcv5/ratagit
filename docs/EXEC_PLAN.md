# EXEC_PLAN.md

## Current Slice

Branches panel commits and commit-files subviews.

## Goal

- keep Branches subview rendering pure and deterministic
- keep Git-derived branch commits and commit-file rows in `AppContext` as the only source of truth
- let Enter drill from Branches into the selected branch commits and then into that commit's file tree
- reuse existing commit-row and commit-file tree projection/rendering components while keeping Branches subview state isolated from the main Commits panel

## Vertical Slice

1. Core state and projection
- add Branches-owned commits and commit-files repo state
- add Branches subview UI state for list, commits, and commit files
- reuse existing commit selection and commit-file tree helpers for subview projections

2. UI and input
- map Enter in Branches list to branch commits and Enter in branch commits to commit files
- render Branches subviews with existing commit-row and file-tree renderers
- keep Esc returning one subview level at a time

3. Tests and harness
- add unit coverage for subview entry, stale results, navigation, and details refresh
- add UI snapshots for branch commits and branch commit files subviews
- add a harness scenario asserting UI and Git state while drilling into a branch commit file

4. Documentation
- update `docs/PRODUCT.md`, `docs/DESIGN.md`, and architecture notes because behavior and GitBackend read operations change

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
