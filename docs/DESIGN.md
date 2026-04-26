# DESIGN.md

## MVP Design

ratagit MVP uses a left-nav workspace interface with six visible panels:

1. Left column (top -> bottom): Files, Branches, Commits, Stash
2. Right column (top -> bottom): Details, Log
3. Bottom row: unframed current-focused-panel Git operation shortcuts only

The focused panel is highlighted with a border/title accent. Selectable rows use
color-only cursor highlighting: the selected row is highlighted only inside the
focused selectable panel, and inactive panels render their selected row as plain
text. Left list panels keep deterministic selected row indexes. Right panels are
read-only views derived from `AppState`. The app does not render a top
branch/focus/status summary.
Panel titles include numbered focus hints `[1]..[6]`.

Focus model:

- default focus starts at `Files`
- `h` / `l` map to `FocusPrev` / `FocusNext` and cycle only left panels
- `FocusPanel` supports direct focus selection (`1..6` in app input map)
- `AppState.last_left_focus` tracks the last active left panel for `Details` projection
- `AppState.details` stores files-detail diff text, target paths, and detail-refresh errors
- `AppState.details` also stores a bounded files-detail diff cache keyed by the
  exact target path list
- `AppState.editor` stores active commit/stash editor modal state (type + fields + cursor indexes + scope)
- `AppState.reset_menu` stores whether the Files reset menu is active and which reset choice is selected
- `AppState.discard_confirm` stores whether the discard confirmation modal is active and the resolved target paths
- `AppState.branches` stores local branch rows plus active branch creation,
  delete menu, rebase menu, and auto-stash confirmation state
- `AppState.work` stores visible pending refresh/details/operation state and
  the last completed command label
- left-panel height baseline follows the Files/Branches/Commits/Stash ratio
- when `Stash` is unfocused it collapses to one content row and freed height is
  redistributed by ratio to Files/Branches/Commits
- when focus is in Files/Branches/Commits and content overflows, the focused
  panel borrows height from other left panels in round-robin order while keeping
  minimum readable rows

---

## Interaction Model

- Input is mapped to explicit `UiAction`.
- `update()` applies state transitions and emits `Command`.
- Command execution is delegated to `GitBackend`.
- The real TUI sends commands to a single background Git worker and receives
  `GitResult` values through a channel; the mock harness can still use the
  synchronous runtime for deterministic scenario tests.
- Runtimes coalesce redundant queued repository refresh and files-detail diff
  commands after the most recent mutation command.
- Backend output re-enters `update()` as `GitResult`.
- UI rendering reads only `AppState`.
- High-frequency side effects use runtime command debouncing keyed by
  `ratagit_core::debounce_key_for_command`, so rapid navigation can collapse to
  the latest command while keeping state transitions deterministic.

Files panel interaction:

- `AppState.files` stores tree expansion, visible-row selection, visual selection anchor, batch rows, and search state.
- File tree rows are derived from `RepoSnapshot.files`; no UI code reads external state.
- `AppState.files.tree_rows`, `row_descendants`, and `row_index_by_path`
  cache deterministic tree projection data after reducer-managed changes.
- Backend status collection uses full untracked-file expansion so untracked
  nested files appear as explicit file rows in the tree.
- Real backend status collection prefers Git CLI porcelain v1 `-z` output inside
  `GitBackend` for large-repository speed and falls back to git2 status if the
  CLI command fails.
- Directories are display targets only and resolve to descendant files from the current snapshot.
- `space` toggles stage state for the current target or visual-selected batch.
- `c` opens commit editor modal:
  - commit message + multiline body fields
  - active field cursor is stored in `AppState` and rendered with the terminal cursor
  - `Left` / `Right` / `Home` / `End` moves within the active field
  - `Tab` / `Shift+Tab` field switching
  - `Ctrl+J` inserts newline in body
  - `Enter` confirms, `Esc` cancels
- `s` opens stash editor modal:
  - normal files mode -> stash all changes, including untracked files
  - visual multi-select mode -> stash selected target paths only
  - title cursor is stored in `AppState` and rendered with the terminal cursor
  - `Left` / `Right` / `Home` / `End` moves within the title field
  - `Enter` confirms, `Esc` cancels
- `D` opens a repository reset modal:
  - choices are `mixed`, `soft`, `hard`, and `Nuke`
  - `j` / `k` or arrow keys select an option
  - the selected option controls the description rendered below the list
  - `Enter` confirms immediately and emits a reset or nuke command, `Esc` cancels
  - `Nuke` performs hard reset semantics followed by `git clean -fd`
- `d` opens a file-targeted discard confirmation modal:
  - targets are resolved when the modal opens from the current cursor target or visual-selected rows
  - directory rows resolve to descendant files in the current snapshot
  - `Enter` emits `Command::DiscardFiles`, `Esc` cancels
  - confirmation text and path summary are rendered only from `AppState.discard_confirm`
- `v` enters visual multi-select at the current row; `j` / `k` updates the continuous anchor-to-cursor range.
- `/` switches the bottom keys area into search input until Enter or Esc.
- Long file lists keep a stable bottom-reserve viewport while reversing from
  downward movement; moving up does not jump to a top-reserve viewport.
- `RefreshAll` and files selection navigation emit `RefreshFilesDetailsDiff` so
  the Details panel follows the current files cursor.
- stale details-diff results are ignored when their path list no longer matches
  the current Files selection.
- while editor, reset, or discard modal is active, modal key handling has highest input priority
  over panel navigation mappings.
- Branches focus maps `space` to checkout, `n` to new branch, `d` to delete,
  and `r` to rebase.
- Branch creation opens an AppState-owned input modal and emits
  `Command::CreateBranch` with the selected branch as `start_point`.
- Branch checkout and rebase inspect current AppState file status; when dirty,
  they open an auto-stash confirmation modal before emitting commands with
  `auto_stash=true`.
- Branch deletion opens an AppState-owned select list for local, remote, or both:
  local deletion is blocked in the reducer when the selected branch is current,
  and `GitBackend` also blocks branches checked out in any worktree. The real
  backend first tries `git branch -d`; if Git reports the branch is not fully
  merged, the reducer stores a force-delete confirmation modal in `AppState`.
  Confirming emits the same deletion command with `force=true`, which maps local
  deletion to `git branch -D`.
- Branch rebase options are simple, interactive, and `origin/main`; simple and
  interactive rebase the current branch onto the selected branch.
- Files Details projection renders merged `unstaged` and `staged` diff sections
  for current file/folder targets from `GitBackend`.
- Repeated Files Details selections reuse the AppState-owned diff cache without
  emitting a new Git command; the cache is cleared after snapshot refreshes and
  successful mutating Git operations.
- Branches/Commits/Stash Details projections are placeholders in this slice and
  intentionally marked for follow-up implementation.

---

## Error Presentation

- Git failures never crash the app.
- Errors are stored in `AppState.status.last_error`.
- The `Log` panel displays the latest error.
- The `Log` panel displays pending refresh and Git operation state while work is running.
- The Files details projection displays a loading row while its diff command is pending.
- Empty-state placeholders such as `<empty>` / `<none>` are not rendered; empty
  views remain visually blank.

---

## Visual Theme

- The UI assumes Unicode/Nerd Font support.
- Panel titles include semantic icons and never use `*` to show focus.
- File rows use icons for folders, files, staged files, untracked files,
  batch membership, and search matches.
- Visual-selected file rows use a color distinct from cursor selection.
- Visible cursor markers such as `>` are not rendered; selection is tested
  through buffer styles.
- Modal overlays use the internal `ratagit-ui` modal system for deterministic
  centering, sizing, borders, inner padding, input blocks, and action footers.
- Modal tones are semantic and text-backed: editor modals use an info accent,
  repository reset uses a warning accent, and discard confirmation uses a danger
  accent while still rendering explicit warning text.
- Branch creation uses the info modal tone, branch delete uses danger, and
  branch force-delete confirmation uses danger; branch rebase plus auto-stash
  confirmation use warning.
- Files diff rows in Details are color-coded by semantics:
  - section headers (`### ...`)
  - diff metadata (`diff --git`, `---`, `+++`, `index`)
  - hunk headers (`@@`)
  - additions (`+`)
  - removals (`-`)

---

## Snapshot and Harness Design

- UI panel unit tests assert pure panel projections from `AppState`.
- Full-screen UI tests render fixed terminal sizes through `render_terminal`
  and `ratatui::TestBackend`.
- Style-sensitive assertions inspect terminal buffer cells instead of relying on
  text snapshots for invisible color state.
- Harness scenarios drive action sequences and assert:
  - real terminal screen text
  - selected-row style when needed
  - backend operation trace
  - final mock Git state
- On failure, harness writes artifacts:
  - compatibility text buffer
  - real terminal screen
  - AppState dump
  - git operation trace
  - final mock Git state
  - input sequence
