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
- `AppState.editor` stores active commit/stash editor modal state (type + fields + scope)
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
- Backend output re-enters `update()` as `GitResult`.
- UI rendering reads only `AppState`.
- High-frequency side effects use runtime command debouncing keyed by
  `ratagit_core::debounce_key_for_command`, so rapid navigation can collapse to
  the latest command while keeping state transitions deterministic.

Files panel interaction:

- `AppState.files` stores tree expansion, visible-row selection, visual selection anchor, batch rows, and search state.
- File tree rows are derived from `RepoSnapshot.files`; no UI code reads external state.
- Backend status collection uses full untracked-file expansion so untracked
  nested files appear as explicit file rows in the tree.
- Directories are display targets only and resolve to descendant files from the current snapshot.
- `space` toggles stage state for the current target or visual-selected batch.
- `c` opens commit editor modal:
  - commit message + multiline body fields
  - `Tab` / `Shift+Tab` field switching
  - `Ctrl+J` inserts newline in body
  - `Enter` confirms, `Esc` cancels
- `s` opens stash editor modal:
  - normal files mode -> stash all changes
  - visual multi-select mode -> stash selected target paths only
  - `Enter` confirms, `Esc` cancels
- `v` enters visual multi-select at the current row; `j` / `k` updates the continuous anchor-to-cursor range.
- `/` switches the bottom keys area into search input until Enter or Esc.
- `d` discard is intentionally not mapped to input until the reusable confirmation dialog exists.
- Long file lists keep a stable bottom-reserve viewport while reversing from
  downward movement; moving up does not jump to a top-reserve viewport.
- `RefreshAll` and files selection navigation emit `RefreshFilesDetailsDiff` so
  the Details panel follows the current files cursor.
- while editor modal is active, editor key handling has highest input priority
  over panel navigation/search mappings.
- Files Details projection renders merged `unstaged` and `staged` diff sections
  for current file/folder targets from `GitBackend`.
- Branches/Commits/Stash Details projections are placeholders in this slice and
  intentionally marked for follow-up implementation.

---

## Error Presentation

- Git failures never crash the app.
- Errors are stored in `AppState.status.last_error`.
- The `Log` panel displays the latest error.
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
