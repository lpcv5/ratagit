# EXEC_PLAN.md

## Current Slice

Fix file details diff rendering for untracked files in the hybrid Git backend.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- keep tracked unstaged and staged diffs on git2 diff APIs
- synthesize deterministic patch text for selected untracked text files
- keep untracked file patches under the existing `### unstaged` section

## Vertical Slice

1. Backend behavior
- detect selected untracked files from git2 status
- render each readable untracked file as a new-file patch from `/dev/null`
- skip staged new files because they are already covered by the staged diff

2. Validation
- add a real temp-repo integration test for untracked file details diff
- add a harness scenario asserting untracked file diff appears in the UI
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
