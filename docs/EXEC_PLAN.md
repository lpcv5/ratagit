# EXEC_PLAN.md

## Current Slice

Speed up large-repository refresh and repeated files-detail work while keeping
complete first-load status data.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- keep first file status complete, including nested untracked files
- keep rendering pure and derived only from `AppState`
- use Git CLI porcelain status only inside `GitBackend`
- avoid slow git2 glob pathspec matching for literal repo paths
- avoid repeated details diff work for cached or stale selections
- coalesce redundant queued refresh/details commands without reordering mutations

## Vertical Slice

1. Backend status and diff path
- parse `git status --porcelain=v1 -z --untracked-files=all --ignored=no --ignore-submodules=all`
  into `FileEntry` values for the refresh snapshot file list
- fall back to the existing git2 status collection if the CLI status command fails
- keep git2 for head, recent commits, branches, stashes, file diffs, stage, and unstage
- set git2 diff pathspecs to literal matching after repo-relative validation

2. Cache and command behavior
- add a bounded AppState-owned files-details diff cache
- return cached details diffs without emitting Git work when selections repeat
- clear the details cache when a new snapshot or mutating Git result changes repository state
- coalesce queued `RefreshAll` and `RefreshFilesDetailsDiff` commands while preserving
  mutation ordering

3. Validation
- add parser, cache, pathspec, runtime coalescing, and harness tests
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
