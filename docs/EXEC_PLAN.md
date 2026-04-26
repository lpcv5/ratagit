# EXEC_PLAN.md

## Goal

Complete the Files panel vertical slice:

- render Git status entries as a deterministic file tree
- support visible tree folder expand/collapse
- support `space` stage/unstage toggling for current or selected targets
- support `s` path-limited stash for current or selected targets
- support `v` multi-select mode for batch operations
- support `/` search input with `n` / `N` match navigation
- implement backend-only discard plumbing for later confirmation UI

## Vertical Slice

1. Core state/action update
- store files tree UI state in `AppState.files`
- derive file tree rows only from `RepoSnapshot.files`
- resolve folder operations to descendant files from the visible snapshot model
- add search input state, match navigation, and multi-select selection state

2. Git command/backend layer
- add batch stage, unstage, stash, and discard commands
- stash selected files with `git stash push -u -- <paths>`
- implement discard backend behavior without mapping `d` in the TUI yet

3. UI render and app input
- render files as an indented tree with directory expansion markers
- show untracked files distinctly
- show multi-select and search-match markers deterministically
- replace the shortcut bar with `search: <query>` while `/` input is active

4. Tests
- unit tests for tree building, target resolution, search, and batch decisions
- snapshot tests for tree, multi-select, and search input rendering
- harness scenarios for expand/collapse, space toggle, multi-select stash, and search navigation

5. Docs and quality gates
- update `PRODUCT.md` and `DESIGN.md`
- run `cargo fmt`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
