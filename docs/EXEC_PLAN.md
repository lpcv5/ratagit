# EXEC_PLAN.md

## Current Slice

Implement first-pass Details behavior with Files diff projection while preserving
the existing left/right workspace layout and pure rendering contract.

## Goal

- make `Files -> Details` show merged `unstaged` + `staged` diff for the
  current cursor target (file or folder descendants)
- color diff rows by semantic role (`section/meta/hunk/add/remove`)
- keep `Branches/Commits/Stash -> Details` as explicit placeholders with TODOs
- leave hunk editing / partial stage as deferred TODO without changing current
  `Enter` behavior

## Vertical Slice

1. Core + command pipeline
- add `AppState.details` as details source of truth
- add `Command::RefreshFilesDetailsDiff` and
  `GitResult::FilesDetailsDiff { paths, result }`
- trigger details refresh on:
  - `RefreshAll` completion
  - files cursor/visibility changes (`j/k`, directory toggle, search
    confirm/jump)
  - focus returning to Files

2. Git backend
- add `GitBackend::files_details_diff(&[String]) -> Result<String, GitError>`
- CLI backend returns merged sections in fixed order:
  - `### unstaged`
  - `### staged`
- mock backend returns deterministic pseudo diff and operation trace entries for
  harness assertions

3. UI projection + TODO anchors
- render Details from `AppState.details` instead of ad-hoc summary projection
- classify files diff lines into semantic roles for color rendering
- keep non-files Details as placeholders with TODO comments for future
  branch-log/commit/stash details
- add TODO comments for future hunk-selection model and `Enter` hunk-edit flow

4. Tests and harness
- core tests for new details command emission and details-result state updates
- git tests for command-to-backend diff flow
- UI tests for files diff projection and semantic color style assertions
- harness scenario verifying details follows files cursor and shows merged
  diff sections
- update terminal snapshots for new Details content

5. Quality gates
- run `cargo fmt`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
