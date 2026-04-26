# EXEC_PLAN.md

## Current Slice

Add a Files subpanel inside Commits.

## Goal

- enter the subpanel from the selected commit with `Enter`
- keep commit files and commit-file diffs inside `AppState`
- load changed files only through `GitBackend`
- reuse the Files tree projection and rendering shape
- show changed-file status markers (`A/M/D/R/C/T`)
- make Details follow the selected commit file/folder with path-limited patches
- keep panel height stable while commit files are loading
- allow commit-file folder rows to expand/collapse
- defer commit-files local shortcuts beyond navigation and `Esc` return

## Vertical Slice

1. State and input
- add commit-files substate under `CommitsPanelState`
- add open/close actions and commit-files commands/results
- map `Enter` from Commits to open, and `Esc` from Commit Files to return
- map `Enter` inside Commit Files to directory expand/collapse

2. Rendering
- render Commit Files in the Commits panel frame
- render commit-file status markers in the reused file-tree row format
- render commit-file loading, empty, error, and diff states in Details
- keep Commit Files loading height based on the parent Commits list to avoid a
  one-frame layout jump

3. Git and validation
- add backend methods for changed files and selected file/folder commit diff
- cover reducer behavior, snapshots, harness scenario, and real backend output
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
