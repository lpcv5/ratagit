# EXEC_PLAN.md

## Current Slice

Complete the first practical Branches panel workflow while keeping the panel
local-branch-only for now.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- keep rendering pure and derived only from `AppState`
- replace placeholder branch shortcuts with focused branch operations
- protect dirty working trees with explicit auto-stash confirmation
- protect current/worktree-occupied branches from local deletion
- keep remote deletion scoped to `origin/<selected-local-branch>`

## Vertical Slice

1. Branch state and input
- add AppState-owned Branches modal state for branch creation, deletion, rebase,
  and auto-stash confirmation
- map Branches focus shortcuts to `space` checkout, `n` new branch, `d` delete,
  and `r` rebase
- create new branches from the selected local branch start point

2. Git commands
- extend `GitBackend` with create-from-start-point, auto-stash checkout,
  branch delete, and rebase commands
- implement local deletion with `git branch -d`
- when safe local deletion reports an unmerged branch, open a force-delete
  confirmation before retrying with `git branch -D`
- implement remote deletion with `git push origin --delete <branch>`
- block local deletion when the branch is checked out in any worktree

3. Validation
- add core reducer, mock Git, real Git worktree, UI snapshot, and harness tests
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
