# EXEC_PLAN.md

## Current Slice

Show selected-branch log graph output in Details while keeping rendering pure
and Git access behind `GitBackend`.

## Goal

- preserve `GitBackend` and `AppState` boundaries
- keep rendering pure and derived only from `AppState`
- display the selected branch's native `git log --graph` output in Details
- preserve Git's original ASCII graph text and ANSI colors
- limit branch details log output to 50 commits
- keep high-frequency branch navigation deterministic through AppState-owned
  details cache

## Vertical Slice

1. Branch details state
- add AppState-owned branch details target, raw ANSI log output, error state,
  and a bounded per-branch log cache
- request branch details when Branches gains focus or the branch cursor moves
- ignore stale branch-log results for branches that are no longer selected

2. Git and rendering
- extend `GitBackend` with a read-only branch log method
- real backend runs `git log --graph --color=always -n 50 <branch>`
- render Branches Details by parsing ANSI SGR into ratatui spans while keeping
  plain text snapshots deterministic

3. Validation
- add core reducer, mock Git, real Git CLI, UI snapshot, color-style, and harness
  tests
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, and `cargo test`
