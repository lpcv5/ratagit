# EXEC_PLAN.md

## Current Slice

Left-panel subview height stability and non-flickering Details refresh.

## Goal

- keep left-panel subviews at the same height as their parent panel view
- prevent Commit Files from shrinking the Commits panel when a commit has only a
  few changed files
- keep Details content stable while new details commands are pending
- update Details only when new content or an error arrives
- preserve pure rendering and `AppState` as the only source of truth

## Vertical Slice

1. Layout
- derive active subview height from the parent panel's main content length
- keep Commit Files rendering scrollable inside that stable panel height

2. Details refresh behavior
- stop rendering transient loading placeholders in Details
- keep prior Details content while details commands are pending
- avoid clearing commit-file details on target changes before the new diff
  arrives

3. Tests
- add focused UI/core coverage for stable Commit Files height calculation
- update snapshot coverage for pending Details rendering
- add or update harness coverage that asserts UI and Git state

4. Documentation
- update PRODUCT behavior notes for stable subview height and non-flickering
  Details refresh

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`

## Latest Validation

- `cargo fmt`
- `cargo clippy --workspace --lib --bins -- -D warnings`
- `cargo test`
