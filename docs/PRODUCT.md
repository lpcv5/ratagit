## Goal

ratagit aims to replicate lazygit UX with a Rust + ratatui implementation.

---

## Core Features

- file staging
- commits
- branches
- stash
- details view
- log view

---

## MVP v0 Scope

MVP v0 includes a left-nav workspace layout with six panels:

- Files: tree view, folder expand/collapse, stage/unstage toggle, path-limited stash, repository reset menu, multi-select, and search
- Branches: create branches from the selected local branch, checkout with
  optional auto-stash, delete local/`origin` branches, and rebase the current
  branch
- Commits: create commit, refresh list, squash/fixup/reword/delete commits, visual multi-select, and detached checkout
- Stash: stash push and stash pop selected entry
- Details:
  - Files focus projection: show merged `unstaged` then `staged` diff for the currently selected file/folder target
  - Untracked text files render as new-file patches in the `unstaged` diff section
  - While a selected file diff is loading, Details shows a deterministic loading row instead of blocking input
  - Branches focus projection: show the selected branch's native `git log --graph` output with Git's ASCII graph and ANSI colors preserved, limited to 50 commits
  - Commits/Stash projection: placeholder text for now (to be implemented in later slices)
  - `Ctrl+U` / `Ctrl+D` scroll Details content up/down globally by 2/5 of the Details content height without changing the focused panel
- Log: show latest error, recent notices, and pending refresh/operation state

Navigation rules:

- `h` / `l` cycles only in left panels: Files -> Branches -> Commits -> Stash
- `1..6` focuses Files/Branches/Commits/Stash/Details/Log directly
- `Ctrl+U` / `Ctrl+D` scrolls the Details panel content by 2/5 of its content height regardless of the current focus
- all panel titles show numbered focus hints: `[1]..[6]`
- top branch/focus/status summary is hidden to prioritize panels
- bottom keys row is unframed and shows only Git operation shortcuts for the current focused panel
- focused panels are indicated by a colored border/title accent, not by `*`
- cursor rows are indicated by color only and only in the focused selectable panel
- files selected for batch operations use a separate batch color
- empty panels render blank content lines instead of `<empty>` / `<none>` placeholders
- Stash panel shows one content row when unfocused and restores default height when focused
- focused Files/Branches/Commits panels can grow dynamically when content overflows by borrowing height evenly from other left panels

Files panel rules:

- file rows come from Git status data only; the app does not scan the working tree separately
- untracked entries are requested with full file granularity (equivalent to
  `git status --untracked-files=all`) so nested untracked files appear as file rows
- folder operations apply to descendant files present in the tree model
- file-tree rows and descendant targets are cached in `AppState` after status
  refresh or tree/search changes, so rendering does not rebuild them every frame
- repeated file-detail diffs are cached in `AppState` and reused when the same
  target path list is selected again
- `space` stages unstaged targets or unstages targets when all selected targets are staged
- `c` opens a commit editor modal from Files focus
  - `message` and `body` fields are editable
  - the active field shows a real terminal cursor
  - `Left` / `Right` / `Home` / `End` moves the cursor within the active field
  - `Tab` / `Shift+Tab` switches active field
  - `Ctrl+J` inserts a newline in body
  - `Enter` confirms, `Esc` cancels
- `s` opens a stash editor modal from Files focus
  - normal mode stashes all current changes, including untracked files
  - `v` multi-select mode stashes only selected target paths
  - the title field shows a real terminal cursor
  - `Left` / `Right` / `Home` / `End` moves the cursor within the title
  - `Enter` confirms, `Esc` cancels
- `D` opens a repository reset menu from Files focus
  - choices are `mixed`, `soft`, `hard`, and `Nuke`
  - `j` / `k` or arrow keys move the menu selection
  - `Enter` immediately confirms, `Esc` cancels
  - reset choices target `HEAD`
  - `Nuke` runs hard reset semantics and then removes untracked files/directories with `git clean -fd`
- `d` opens a discard confirmation modal for the current Files target
  - normal mode targets the current file row or all descendant files for the current directory row
  - `v` multi-select mode targets the selected visual range
  - `Enter` discards tracked changes and removes selected untracked targets, `Esc` cancels
- `v` enters visual multi-select at the current row; `j` / `k` extends or shrinks the selected range
- `/` opens search input in the bottom bar; Enter confirms, Esc cancels or clears, `n` / `N` navigate matches
- `Enter` still toggles directory expand/collapse; hunk editing and partial-stage flow are explicitly deferred
- details-diff side effects for high-frequency files navigation are debounced to keep `j` / `k` scrolling smooth
- queued refresh/details work is coalesced so stale duplicate details commands do
  not delay the latest selection
- branch-details log graph output is cached in `AppState` and reused when the
  same branch is selected again during the current snapshot
- real TUI Git work runs on a single background worker so initial refresh and
  long operations do not block drawing or keyboard input
- real backend file status refresh uses Git porcelain status inside `GitBackend`
  for large repositories while preserving full untracked-file expansion

Branches panel rules:

- the Branches panel lists local branches only in this slice
- `space` checks out the selected branch
  - if the working tree has uncommitted changes, an auto-stash confirmation
    modal opens first
  - confirming stashes changes, checks out the branch, then restores the stash
  - cancelling leaves the repository unchanged
- `n` opens a branch-name input modal
  - the new branch is created from the selected branch as the start point
  - `Enter` creates the branch, `Esc` cancels
- `d` opens a branch delete menu
  - choices are local, remote, and local plus remote
  - local deletion uses safe `git branch -d`
  - if Git reports the branch is not fully merged, a force-delete confirmation
    modal opens so the user can decide whether to delete with `git branch -D`
  - remote deletion targets `origin/<selected-local-branch>`
  - deleting the current local branch is blocked with a notice
  - deleting a branch checked out by a worktree is blocked by `GitBackend`
- `r` opens a rebase menu
  - simple rebase rebases the current branch onto the selected branch
  - interactive rebase runs Git interactive rebase onto the selected branch
  - origin/main rebase rebases the current branch onto `origin/main`
  - dirty rebase uses the same explicit auto-stash confirmation as checkout

Commits panel rules:

- commit rows render four columns: Unicode graph placeholder, hash, two-letter author, and message
- initial refresh loads the first 100 commits; moving into the last 20 loaded commits prefetches the next 100 commits
- commit list scrolling keeps the visible window still while the cursor remains inside the middle rows; it scrolls only after crossing the top or bottom three-row reserve
- the graph column uses a deterministic `●` placeholder in this slice
- author initials are derived from the author name and colored deterministically per author
- commit hashes are colored by reachability:
  - green when reachable from local `main`
  - yellow when not on `main` but reachable from the current branch upstream
  - red when not reachable from the upstream or when upstream/main data is unavailable
- `v` enters visual multi-select at the current commit; `j` / `k` updates the continuous anchor-to-cursor range
- `s` squashes the selected commit or visual-selected commits into their parent lineage
- `f` fixups the selected commit or visual-selected commits into their parent lineage
- `r` opens the commit message modal prefilled with the selected commit message and rewords one commit
- `d` deletes the selected commit or visual-selected commits
- rewrite actions require a clean working tree, only operate on red/unpushed commits, and reject merge commits in this slice
- squash/fixup reject commits whose parent is the root commit in this slice
- `space` checks out the selected commit as detached HEAD; dirty worktrees use the same explicit auto-stash confirmation as branch checkout

All features are keyboard-driven and deterministic.

The visual theme is Unicode/Nerd Font first. Panel titles and file, branch, and
status rows use a compact semantic icon set while preserving deterministic text
layout.

---

## UX Rules

- keyboard-driven
- predictable navigation
- minimal latency
- consistent layout

---

## Non-Goals (initial)

- full git parity
- plugin system
