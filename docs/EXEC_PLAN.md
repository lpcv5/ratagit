# EXEC_PLAN.md

## Current Slice

Polish Files-panel commit/stash editor modals and add real editor cursor
support.

## Follow-up Hotfix

- fix Files tree rendering for untracked directory marker entries returned by
  `git status` (for example `libs/ratagit-git/tests/`)
- render them as directory nodes (`tests/`) instead of malformed file nodes
  showing full path text
- cover with core tree unit test + harness scenario

## Goal

- improve commit/stash editor modal readability with form-style fields
- keep editor cursor state in `AppState`
- render a real terminal cursor for the active editor field
- support `Left` / `Right` / `Home` / `End` cursor movement
- keep commit/stash backend command interfaces stable

## Vertical Slice

1. Core state and reducer
- store commit subject/body cursor indexes and stash title cursor index in `AppState.editor`
- add editor cursor movement `UiAction` variants
- insert characters, newlines, and Backspace at the active field cursor
- preserve UTF-8 char boundaries for cursor movement and deletion

2. Input mapping + UI modal overlay
- map `Left` / `Right` / `Home` / `End` before other modes while editor is active
- replace `>>` line prefixes with form-style input blocks
- set ratatui frame cursor to the active editor field position
- keep body viewport derived from the cursor line

3. Validation
- add/adjust unit tests for cursor movement, insertion, deletion, unicode, and key mapping
- update commit/stash editor snapshots
- assert terminal cursor position for commit subject/body and stash title
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
