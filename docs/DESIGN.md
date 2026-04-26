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
- `AppState.details` stores commit-detail diff text, selected commit target,
  detail-refresh errors, and a bounded commit-detail cache keyed by full commit id
- `AppState.details.scroll_offset` stores the global Details viewport offset
  used by `Ctrl+U` / `Ctrl+D`; the action carries terminal-size-derived scroll
  and visible-line counts
- `AppState.editor` stores active commit/stash editor modal state (type + fields + cursor indexes + scope)
- `AppState.reset_menu` stores whether the Files reset menu is active and which reset choice is selected
- `AppState.discard_confirm` stores whether the discard confirmation modal is active and the resolved target paths
- `AppState.search` stores generic panel-scoped search input, query, matches,
  and current match index for searchable left panels and subpanels
- `AppState.commits` stores commit rows, cursor selection, visual selection anchor, and selected visual range
- `AppState.commits.files` stores the active Commit Files subpanel state,
  including selected commit id, changed-file rows, tree projection, and cursor
  scroll state
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
- `Ctrl+U` and `Ctrl+D` map to global Details scroll actions before
  mode-specific key handling. The input layer computes the step as
  `max(1, details_content_height * 2 / 5)` from the current terminal layout.
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

Search interaction:

- `/` activates `AppState.search` for the current Files, Branches, Commits,
  Stash, or Commit Files scope.
- Search input replaces the bottom shortcuts with `search: <query>` until
  `Enter` confirms or `Esc` cancels.
- Normal bottom shortcuts list only panel-specific common actions; baseline
  navigation/search keys are omitted.
- Matches are case-insensitive and deterministic:
  - Files and Commit Files match full paths
  - Branches match branch names
  - Commits match visible row identity: short hash, author initials, and
    summary/message first line
  - Stash matches stash id plus summary
- Search styling is character-level: visible matching substrings receive the
  search style while the rest of the row keeps its normal semantic styling.
  File-tree rows still show the search marker when the full path matches but
  the matching path segment is not visible in the compact row label.
- `n` / `N` navigate confirmed matches in the active scope. Files, Branches,
  Commits, and Commit Files refresh their Details projection after selection
  changes; Stash only updates its selected row in this slice.

Files panel interaction:

- `AppState.files` stores tree expansion, visible-row selection, visual selection anchor, and batch rows.
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
- `/` switches the bottom keys area into search input until Enter or Esc using
  the generic search model.
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
- Branches search selects matching branches and refreshes the branch details log.
- Commits focus maps `s` to squash, `f` to fixup, `r` to reword, `d` to delete,
  `space` to detached checkout, `v` to visual multi-select, and `Enter` to
  open Commit Files for the selected commit.
- Commit visual multi-select is AppState-owned and follows the same continuous
  anchor-to-cursor model as Files visual selection.
- Commit rewrite commands require a clean working tree, reject merge commits in
  this slice, only accept unpushed commits, and are executed only through
  `GitBackend`.
- Squash/fixup reject root-parent targets in this slice because the replay path
  amends the selected commit into its parent.
- Commit reword reuses the existing commit editor modal with a reword intent
  stored in `AppState.editor`; confirming emits `Command::RewordCommit`.
- Commits search selects matching loaded commits and refreshes the commit diff;
  it does not request additional pages.
- Detached commit checkout uses `Command::CheckoutCommitDetached`; when the
  working tree is dirty it opens the shared auto-stash confirmation modal before
  dispatching with `auto_stash=true`.
- Commit Files is an AppState-owned subpanel under Commits:
  - opening emits `Command::RefreshCommitFiles` for the selected commit
  - rows reuse the Files tree projection/rendering shape but use commit
    changed-file status markers (`A/M/D/R/C/T`) instead of working-tree status
  - `j` / `k` move the Commit Files cursor through the shared panel navigation
    action path
  - `Enter` toggles the selected directory row, while file rows leave a notice
  - Details follows the selected file or folder via
    `Command::RefreshCommitFileDiff`
  - directory selections resolve to descendant commit files and request one
    combined path-limited patch
  - stale commit-files and commit-file-diff results are ignored when the user
    has moved to another commit or file
  - `Esc` closes the subpanel and restores the selected commit diff
  - while changed files are loading, dynamic height calculations use the parent
    Commits list length to avoid a one-frame panel jump
  - `/` searches changed-file paths and refreshes the selected file/folder diff
  - additional local commit-files shortcuts are intentionally deferred
- Files Details projection renders merged `unstaged` and `staged` diff sections
  for current file/folder targets from `GitBackend`.
- Commits Details projection renders the selected commit's header and patch diff
  from `GitBackend`.
- Commit Files Details projection renders the selected commit file or folder's
  patch from `GitBackend`.
- Files, Branches, and Commits Details projections apply the AppState-owned
  `scroll_offset`; loading, empty, and error rows ignore the offset.
- Details scroll resets when the selected details target changes or accepted
  details content refreshes.
- Details downward scroll clamps `scroll_offset` to the last visible page
  (`content_len - visible_lines`) to avoid hidden overscroll at the bottom.
- Repeated Files Details selections reuse the AppState-owned diff cache without
  emitting a new Git command; the cache is cleared after snapshot refreshes and
  successful mutating Git operations.
- Repeated Commit Details selections reuse the AppState-owned diff cache without
  emitting a new Git command; the cache is cleared after snapshot refreshes and
  successful mutating Git operations.
- Stash Details projection is a placeholder in this slice and intentionally
  marked for follow-up implementation.

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
- Commit rows use fixed graph/hash/author/message columns; the graph is a `●`
  placeholder until a later topology pass, hash colors represent main/upstream
  reachability, and author initials use deterministic per-author colors.
- Files and Commits share the same AppState-owned scroll direction/origin helper
  so list windows move only after the cursor crosses the three-row top/bottom
  reserve.
- Commit pagination is AppState-owned: refresh stores the first 100 commits,
  Commits navigation prefetches the next 100 when the cursor enters the last 20
  loaded commits, and successful pages append to `AppState.commits.items`.
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
