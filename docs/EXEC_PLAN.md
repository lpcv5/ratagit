# EXEC_PLAN.md

## Current Slice

Million-file repository Git performance.

## Goal

- keep startup responsive when Git index entry count is very large
- avoid full untracked expansion for repositories with at least 100,000 index entries
- cap status and details-diff work so million-file repositories cannot flood AppState, logs, or Git workers
- keep rendering pure and AppState as the only source of truth
- keep UI assertions deterministic through fixtures, unit tests, snapshots, and harness scenarios

## Vertical Slice

1. Core state and command model
- add status performance metadata to `FilesSnapshot` / `AppState.status`
- carry deterministic `FileDiffTarget` values for Files Details diff commands
- limit Details diff commands to the first 100 resolved targets and keep the
  truncation notice in `AppState`

2. Git backend
- select `StatusMode::LargeRepoFast` when index entry count is at least 100,000
- use `git status --untracked-files=no` in large mode and
  `--untracked-files=all` in full mode
- cap parsed status entries at 50,000 and stdout at 64 MiB
- set `GIT_OPTIONAL_LOCKS=0` for read-only Git CLI commands
- keep all Git access behind `GitBackend`

3. Files tree and Details
- initialize large-repo Files trees collapsed with lightweight row projection
- avoid cached `row_descendants` construction during large-repo initial load
- skip unknown untracked directory marker diffs when untracked scanning was skipped

4. UI snapshots and docs
- show Log notices for large repo fast status, skipped untracked scan, status
  truncation, details diff limits, and optional Git config tips
- add terminal snapshots and harness scenarios for large repo status and large
  directory details limits

5. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
