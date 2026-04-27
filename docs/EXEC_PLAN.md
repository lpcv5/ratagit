# EXEC_PLAN.md

## Current Slice

Million-index repository pressure reduction.

## Goal

- avoid automatic whole-working-tree status scans when the index is at or above
  1,000,000 entries
- keep commits, branches, stash, and existing bounded Details previews usable
  while the Files panel declines to load a massive status result
- keep rendering pure and `AppState` as the only source of truth
- avoid logging Git stdout, diff text, or commit message payloads

## Vertical Slice

1. Backend behavior
- add a metadata-only status mode for million-index repositories
- skip `git status` file collection in that mode after counting the index
- surface deterministic status metadata through `FilesSnapshot`

2. UI and state
- store skipped status-scan metadata in `AppState.status`
- render a Log notice explaining that file scanning was skipped
- leave Files Details empty instead of emitting file-diff commands

3. Tests and harness
- add unit coverage for the huge-repo threshold and snapshot metadata
- add UI snapshot coverage for the Log notice
- add a harness scenario that asserts UI, Git operations, and Git state

4. Documentation
- update product/design/architecture docs for metadata-only huge-repo status

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
