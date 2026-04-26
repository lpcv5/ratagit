# EXEC_PLAN.md

## Current Slice

Implement Files-panel commit/stash editor modals and unify real Git CLI tests
under workspace `tmp/`.

## Follow-up Hotfix

- fix Files tree rendering for untracked directory marker entries returned by
  `git status` (for example `libs/ratagit-git/tests/`)
- render them as directory nodes (`tests/`) instead of malformed file nodes
  showing full path text
- cover with core tree unit test + harness scenario

## Goal

- add a commit editor flow from Files (`c`) with:
  - subject + multiline body
  - `Tab` / `Shift+Tab` field switch
  - `Ctrl+J` newline in body
  - `Enter` confirm, `Esc` cancel
- add a stash editor flow from Files (`s`) with:
  - normal mode -> stash all changes
  - visual mode -> stash selected target paths
  - `Enter` confirm, `Esc` cancel
- keep commit/stash backend command interfaces stable
- ensure real Git integration tests run in `tmp/git-tests/*` repos only

## Vertical Slice

1. Core state and reducer
- add `AppState.editor` as editor source of truth
- add editor-focused `UiAction` variants (open/input/switch/newline/confirm/cancel)
- keep existing `Command`/`GitResult` types for commit/stash execution
- compose commit message in core (`subject + optional body`)
- freeze stash scope at modal open (`All` vs `SelectedPaths`)

2. Input mapping + UI modal overlay
- add editor-mode key mapping precedence in `src/main.rs`
- map Files `c/s` to open editor actions
- render centered commit/stash modal overlay in terminal renderer
- show editor-specific shortcuts while modal is active

3. Git behavior + tests
- mock backend commit summary uses first line for commit list projection
- add real CLI integration tests under `libs/ratagit-git/tests`:
  - multiline commit message
  - stash push all
  - stash files selected paths
- build temporary repos under `<workspace>/tmp/git-tests/<unique-case>`
- ignore `/tmp/` in git

4. Validation
- add/adjust unit tests (core/main) for editor state machine and key mapping
- add UI snapshot coverage for commit/stash modal states
- add harness scenarios for:
  - `files_commit_editor_multiline_confirm`
  - `files_stash_editor_all_mode`
  - `files_stash_editor_multiselect_mode`
- run `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, `cargo test`
