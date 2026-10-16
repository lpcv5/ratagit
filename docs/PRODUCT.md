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

Command workflows:

- `?` opens the command modal in navigation mode: `j/k` or arrows select,
  `/` enters search, `Enter` executes, and `Esc` closes. Search filters keys/labels;
  while editing, `j/k` are literal text and `Enter` or `Esc` returns to navigation
  with the filter retained, without executing. `/` resumes editing the query.
  Empty results are explicit; ordinary typing in navigation does not filter.
- `F` fetches all remotes with pruning outside commit lists. In main/branch
  commit lists, `F` creates a fixup commit; fetch remains in the command palette.
  Lowercase `f` retains direct commit fixup.
- Stash focus previews the selected stash; `Enter` focuses its read-only diff,
  `a` applies it without removal, `O` pops it, and `d` opens a drop confirmation.
  A conflicting pop refreshes Files to expose conflicts and keeps the stash.

---

## MVP v0 Scope

MVP v0 includes a left-nav workspace layout with six panels:

- Files: tree view, folder expand/collapse, stage/unstage toggle, path-limited stash, repository reset menu, multi-select, and search
- Branches: create branches from the selected local branch, checkout with
  optional auto-stash, delete local/`origin` branches, and rebase the current
  branch; Enter drills into the selected branch's commits and then that commit's
  file tree
- Commits: create commit, refresh list, squash/fixup/reword/delete commits, visual multi-select, detached checkout, and a commit Files subpanel
- Stash: entries, read-only diff preview, apply, pop, and confirmed drop
- Details:
  - Files focus projection: show native unstaged then staged diff text for the
    currently selected file/folder target, without synthetic section labels
  - Untracked text files render as new-file patches in the `unstaged` diff section
  - Files previews and interactive hunk selection share the same diff row
    component, with add/remove colors in both modes. Line numbers and their
    separator are omitted; only cursor/mark indicators precede native diff text.
  - Diff content preserves native `git diff` headers (`diff --git`, `index`,
    `---`/`+++`, modes and binary metadata), hunk headers and single-character
    prefixes. No extra space is inserted after `+`/`-`; indentation and blank
    context lines are retained. Tabs use eight-column stops within diff content.
  - The hunk counter and inline keyboard-help rows are removed; diff content
    uses the full panel height. Keyboard help remains in the shortcut bar.
  - History diff/log payloads preserve Git's native ANSI colors
  - While a selected file diff is loading, Details keeps the previous content visible until new content or an error arrives
  - Entering Details keeps Files at its expanded height. The Details content
    column stays fixed, and its existing preview/scroll remain visible while the
    reviewed patch loads; partial writes become available only after loading.
  - Branches focus projection: show the selected branch's native `git log --graph` output with Git's ASCII graph and ANSI colors preserved, limited to 50 commits
  - Commits focus projection: show the selected commit's header and a bounded patch diff preview
  - Commit Files subpanel projection: show the selected file or folder's patch within the selected commit
  - Stash projection: the selected stash's reviewed, read-only native patch,
    including saved untracked files within the shared preview limits
  - `Ctrl+U` / `Ctrl+D` scroll Details content up/down globally by 2/5 of the Details content height without changing the focused panel
- Log: show latest error, recent notices, and pending refresh/operation state
- Global sync: `p` runs `git pull`, `P` runs `git push`; if normal push is
  rejected because the remote branch has diverged, a confirmation modal asks
  whether to retry with force push using `--force-with-lease`

Common panel workflows (also available from `?`):

| Context | Keys | Behavior |
| --- | --- | --- |
| Normal mode | `m` | Continue, abort, or skip the active merge/rebase/cherry-pick/revert; abort requires confirmation. Merge does not support skip. From interactive Details, return with `Esc` or use `?`. |
| Files | `M` | Resolve selected conflicted files using Git's ours/theirs, or mark resolved after external editing with `e`. Replacing files requires confirmation; mark-resolved refuses remaining conflict markers, including configured `conflict-marker-size`. Modify/delete conflicts can resolve to deletion. |
| Files | `S` | Stash all (including untracked), only staged, or stash with `--keep-index`. Keep-index leaves staged changes in place and does not include untracked files; its stash also records the index snapshot according to Git semantics. |
| Files | `Ctrl+B` | Choose all/staged/unstaged/untracked/conflicted status filter. Filters change projection and folder targets, preserve complete repository data, and remain active across refresh. `a` continues to stage/unstage all repository status targets. |
| Files | backtick, `-`, `=` | Toggle flat/tree layout, collapse all, expand all. Flat mode displays full paths and preserves unknown untracked directory markers. |
| Files | `g` | Soft/mixed/hard reset of current branch to its upstream. Hard requires confirmation. |
| Files | `w` | Enter a message and commit staged changes with `--no-verify`; stage changes first. |
| Files | `C` | Edit a commit message externally, using `GIT_EDITOR`, `core.editor`, then `VISUAL`/`EDITOR`/platform fallback. Empty message cancels. The TUI suspends/restores around the editor; normal stage-all fallback applies. |
| Branches | `M` | Merge selected branch into current branch, or squash merge and leave changes staged for a separate commit. A clean worktree is required. |
| Branches | `R` | Rename selected local branch. |
| Branches | `u` | Set/unset selected branch upstream, fast-forward it, or confirm resetting current branch to its own upstream. |
| Branches | `f` | Fast-forward selected branch from its locally recorded upstream; fetch first for newer remote commits. Non-current updates preserve the checked-out worktree and refuse divergence. |
| Branches | `c`, `-` | Checkout by name or switch to previous branch; dirty worktrees use the existing auto-stash confirmation. |
| Branches | `Ctrl+O` | Copy selected branch name. |
| Branches | `]` / `Tab` | Browse fetched remote branches in a scrollable menu; choose a remote, enter a new local name, and checkout with tracking. Requires a clean worktree. `Esc` returns to local branches. |
| Main/branch commits | `C`, `V`, `Ctrl+R` | Copy selected commit(s), cherry-pick oldest first onto current branch, or clear copied selection. Failed picks retain the selection and expose conflicts. |
| Main/branch commits | `t` | Create a revert commit; a clean worktree is required. |
| Main/branch commits | `g` | Soft/mixed/hard reset current branch to selected commit; hard requires confirmation. |
| Main/branch commits | `n` | Create a branch from selected commit. Confirmed text search keeps `n` as next-match until cleared. |
| Main/branch commits | `Ctrl+O`, `y` | Copy full hash directly, or choose full/short hash, message, or author name. |
| Main/branch commits | `Ctrl+K/J` / `Alt+Up/Down` | Move one commit up/down in newest-first list using interactive rebase; confirmation and a clean linear history are required. |
| Main/branch commits | `F`, `S` | Create a `fixup!` commit from staged changes; confirm autosquashing fixups from selected base through HEAD. |
| Main/branch commits | `T` | Create a lightweight tag at selected commit. |

New workflow menus capture their target when opened. Arrows select menu choices;
prompts support Unicode and Left/Right/Home/End cursor editing. `Esc` cancels
without Git mutation. Failed conflict-producing operations refresh status and
remain recoverable with `M` and `m`. History moves/autosquash reject root/merge
boundaries rather than flattening merges. Conflict resolution operates on whole
files, with external editing available for finer decisions.
Automatic stash restores both staged and unstaged changes after checkout/rebase,
preserves existing stash entries, and keeps its temporary stash if restoration
fails. Confirmation dialogs show consequences before bounded target previews.

Navigation rules:

- `h` / `l` cycles only in left panels: Files -> Branches -> Commits -> Stash
- `1..6` focuses Files/Branches/Commits/Stash/Details/Log directly
- `Ctrl+U` / `Ctrl+D` scrolls the Details panel content by 2/5 of its content height regardless of the current focus
- `p` pulls and `P` pushes from normal mode regardless of focused panel; active
  text inputs and confirmation modals keep their own key handling
- `?` opens an executable command palette with `Local commands` and
  `Global commands` sections; `j/k` or arrows select, `/` starts search, `Enter`
  runs the selection, and `Esc` closes. In search, `Enter`/`Esc` finishes editing
  and retains the filter for navigation
- `p` and `P` are also available from the command palette's Global commands
  section, alongside refresh, focus navigation, Details scrolling, and quit
- `A` amends staged changes into `HEAD` by default; when the main Commits panel
  is focused, it amends staged changes into the selected commit and replays
  newer commits
- if a commit/amend operation needs staged changes and there are none, but file
  changes exist, a stage-all confirmation modal asks whether to stage every
  current file row and continue the original operation
- confirmation modals are used for operations that can discard unrecoverable
  working tree data, remove remote branches, force delete local branches, force
  push, or run a staged-required operation when nothing is staged
- all panel titles show numbered focus hints: `[1]..[6]`; in the terminal UI
  these hints render as badge-style reverse-video numbers
- top branch/focus/status summary is hidden to prioritize panels
- bottom keys row is unframed and shows only local shortcuts for the current
  focused panel plus `? commands`; global sync commands remain bound but are
  shown in the command palette instead of the bottom row
- when background work is pending, the bottom keys row prefixes shortcuts with
  an animated `/|\\-` spinner and loading type such as `loading: refresh`;
  without pending work, the row remains unchanged
- bottom shortcut keys render as badge-style reverse-video labels, without a
  `keys(panel):` prefix and without pipe separators
- baseline navigation/search keys are intentionally omitted from normal bottom
  shortcuts; while search input is active, the bottom row becomes
  `search: <query>`
- focused panels are indicated by a colored border/title accent, not by `*`
- every panel renders its own complete rounded border, including inactive panels;
  focus changes the border/title color without changing the content inset
- cursor rows are indicated by color only and only in the focused selectable panel
- rows selected for batch operations use a separate batch color; every text
  fragment uses the selection foreground, including authors, hashes, file statuses
  and search matches, to preserve readability
- empty panels render blank content lines instead of `<empty>` / `<none>` placeholders
- Stash panel keeps three content rows when unfocused and restores its weighted
  height when focused
- focused Files/Branches/Commits panels can grow dynamically when content overflows
  by borrowing height evenly from other left panels, preserving three content rows
  plus each panel's own borders
- the three-row reserve fits all left panels at terminal heights of 21 rows or
  more, including shortcuts; smaller terminals reduce the reserve evenly within
  the available height; Details and Log also keep three content rows when possible
- left-panel subviews keep the same height as their parent panel view, so entering a short subview does not resize surrounding panels

Files panel rules:

- file rows come from Git status data only; the app does not scan the working tree separately
- small repositories request untracked entries with full file granularity
  (equivalent to `git status --untracked-files=all`) so nested untracked files
  appear as file rows
- repositories with at least 100,000 index entries automatically enter large
  repo fast status mode: tracked/index changes are shown, full untracked
  expansion is skipped, and the Log panel reports that untracked scanning was
  skipped
- repositories with at least 1,000,000 index entries automatically enter
  metadata-only status mode: the Files panel does not auto-load file status
  rows, the Log panel reports that file scanning was skipped, and Commits,
  Branches, Stash, and bounded Details previews can still load
- status results are capped at 50,000 file entries or 64 MiB of status stdout;
  if the cap is hit, the Log panel reports that status was truncated
- folder operations apply to descendant files present in the tree model
- Files and Commit Files share cached file-tree rows and parent/child indexes
  in `AppContext`, so rendering and folder toggles do not rebuild tree topology
  every frame
- Files and Commit Files compact directory chains that only lead to one child at
  each level into one row, such as `src/a/b/c/`, while folder operations still
  target the real descendant files
- Files and Commit Files render file rows with explicit Git status characters
  instead of file-state icons. Workspace rows use `A/M/D/R/C/T/?`, append `U`
  for conflicted files, and render staged file names in green.
- in large repo fast mode, the Files tree starts collapsed and uses a lightweight
  projection and cached child index instead of precomputing every directory
  descendant set or rescanning every file path on folder expand/collapse
- in Commit Files, Git status letters use `A/M/D/R/C/T/?`; status colors apply
  to the letter only and file names stay in the default foreground
- repeated file-detail diffs are cached in `AppContext` and reused when the same
  target path list is selected again
- `space` stages unstaged targets or unstages targets when all selected targets are staged.
  Unstaging a staged rename restores both the old and new index paths while
  preserving the renamed worktree file and any later edits.
- `a` stages all visible status targets, or unstages all when every target is staged
- `Enter` expands directories or opens a reviewed file patch in Details
- In Details, `j/k` navigates contiguous changes (Hunk mode), `a`
  toggles Hunk/Line mode, `v` selects a range in the current mode, and a second
  `v` freezes that range. `x` marks independent hunks/lines; marked targets take
  precedence over the cursor. `Esc` clears marks/range before returning to Files.
  Line mode can traverse context, but operations apply changed lines only.
  `Tab` switches independent staged/unstaged cursors, `Space`
  applies stage/unstage, `d` confirms discard on unstaged changes or unstages
  staged changes, and `n/N` selects adjacent files. After hunk writes, the cursor
  follows the reviewed change to its destination side, allowing immediate reversal.
- staged and unstaged edits share a native combined preview when every source
  edit maps safely; overlapping edits retain separate source previews
- binary, truncated, renamed, mode-changing, and read-only patches cannot be
  partially applied; every write rechecks the exact reviewed native bytes
- `e` edits the selected file using `VISUAL`, then `EDITOR`, then the platform
  default; the TUI suspends and restores the terminal around the editor
- `Ctrl+o` copies the selected relative path; `i` adds an anchored literal ignore
  rule for an untracked file or directory without replacing existing rules
- `c` opens a commit editor modal from Files focus
  - `message` and `body` fields are editable
  - the active field shows a real terminal cursor
  - the active field is highlighted by its border, not by changing the text
    background
  - `Left` / `Right` / `Home` / `End` moves the cursor within the active field
  - `Tab` / `Shift+Tab` switches active field
  - `Ctrl+J` inserts a newline in body
  - `Enter` confirms, `Esc` cancels

  - if no files are staged but file changes exist, confirming the commit opens a
    stage-all confirmation modal; `Enter` stages all current file rows and then
    creates the commit, while `Esc` cancels the fallback
- `s` opens a stash editor modal from Files focus
  - normal mode stashes all current changes, including untracked files
  - visual multi-select mode stashes only selected target paths, treating names
    literally and excluding unrelated staged files from both saved snapshots
  - the title field shows a real terminal cursor
  - the title field is highlighted by its border, not by changing the text
    background
  - `Left` / `Right` / `Home` / `End` moves the cursor within the title
  - `Enter` confirms, `Esc` cancels
- `D` opens a repository reset menu from Files focus
  - choices are `mixed`, `soft`, `hard`, and `Nuke`
  - `j` / `k` or arrow keys move the menu selection
  - `Enter` immediately runs `mixed` or `soft`; `hard` and `Nuke` open a danger
    confirmation modal before running
  - `Esc` cancels the menu or confirmation
  - reset choices target `HEAD`
  - `Nuke` runs hard reset semantics and then removes untracked files/directories with `git clean -fd`
- `d` opens a discard confirmation modal for the current Files target
  - normal mode targets the current file row or all descendant files for the current directory row
  - visual multi-select mode targets the selected visual range
  - `Enter` discards tracked changes and removes selected untracked targets, `Esc` cancels
  - discard and unstage targets are literal paths; Git wildcard characters in a
    filename do not select other files
- `v` enters visual multi-select at the current row; `j` / `k` extends or shrinks the selected range, and `Esc` exits multi-select
- `/` opens search input in the bottom bar; Enter confirms, Esc cancels or clears, `n` / `N` navigate matches
- the same search model applies to Branches, Commits, Stash, and Commit Files,
  scoped to the currently focused list or subpanel
- search highlights the matching visible characters instead of styling the
  whole row
- `Enter` toggles directories or opens partial staging for a file
- details-diff side effects for high-frequency files navigation are debounced to keep `j` / `k` scrolling smooth
- Files Details automatically limits folder or multi-select diffs to the first
  100 resolved file targets and shows a deterministic "first 100 of N files"
  notice when results are truncated
- unknown untracked directory markers are not diffed when large repo mode has
  skipped untracked scanning; Details shows a deterministic skip message instead
- queued refresh/details work is coalesced so stale duplicate details commands do
  not delay the latest selection
- Files Details tracked diffs, automatic Commits Details, and Commit Files patch
  previews use bounded native Git output with ANSI colors preserved; previews
  show a deterministic truncation notice when the cap is reached, and the
  Commit Files subpanel remains the path-focused way to inspect individual
  files in a large commit
- branch-details log graph output is cached in `AppContext` and reused when the
  same branch is selected again during the current snapshot
- real TUI read-only Git work runs on a fixed background worker pool so initial
  refresh, details, log, and pagination work do not block drawing or keyboard
  input; mutating Git work remains serialized through one exclusive worker
- initial and manual refresh load Files/status, Branches, Commits, and Stash as
  independent read-worker jobs, so a slow Files status scan does not hold back
  the other left panels
- real backend file status refresh uses Git porcelain status inside `GitBackend`;
  read-only Git CLI commands use `GIT_OPTIONAL_LOCKS=0`, and Log suggests
  optional manual Git configuration such as `untrackedCache`, `fsmonitor`, and
  `splitIndex` for very large repositories
- the TUI writes low-noise file logs by default and supports `RATAGIT_LOG=debug`
  for Git backend performance diagnostics, including command timings, status
  parse/sort timings, async worker queue delay, and truncation/fallback signals
  without logging Git stdout or diff payloads

Branches panel rules:

- the Branches list shows local branches; `]` / `Tab` opens fetched remote branches
- `space` checks out the selected branch
  - if the working tree has uncommitted changes, an auto-stash confirmation
    modal opens first
  - confirming stashes changes, checks out the branch, then restores the stash
  - cancelling leaves the repository unchanged
- `v` enters visual multi-select at the current branch; `j` / `k` updates the
  continuous selected range, and `Esc` exits multi-select
- `n` opens a branch-name input modal
  - the new branch is created from the selected branch as the start point
  - `Enter` creates the branch, `Esc` cancels
- `d` opens a branch delete menu
  - choices are local, remote, and local plus remote
  - local deletion uses safe `git branch -d`
  - remote deletion and local plus remote deletion open a danger confirmation
    before mutating Git state
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
- `/` searches branch names; confirming or navigating matches updates the
  selected branch and refreshes Branches Details
- `Enter` opens a commits subview for the selected branch using `git log <branch>`
  semantics
  - commit rows use the same rendering as the main Commits panel
  - Details follows the selected branch commit with the same bounded commit diff preview
  - `Enter` on a branch commit opens a Commit Files subview for that commit
  - Branch Commit Files rows use the same tree display and Details patch preview
    as the main Commit Files subpanel
  - `Esc` returns from Branch Commit Files to Branch Commits, then from Branch
    Commits to the branch list

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
- `v` enters visual multi-select at the current commit; `j` / `k` updates the continuous anchor-to-cursor range, and `Esc` exits multi-select
- `A` amends currently staged changes into the selected commit, requiring the
  selected commit and any newer replayed commits to be non-merge
- `s` squashes the selected commit or visual-selected commits into their parent lineage
- `f` fixups the selected commit or visual-selected commits into their parent lineage
- `r` opens the commit message modal prefilled with the selected commit message and rewords one commit
- `d` deletes the selected commit or visual-selected commits
- squash/fixup/reword/delete run through Git interactive rebase and require a
  clean working tree; amend normally
  requires only staged changes with no unstaged or untracked changes, but when
  nothing is staged it can ask to stage all current file rows and continue; these
  rewrite actions can modify pushed or unpushed linear history and reject merge
  commits in this slice. Pushing rewritten public history is handled separately
  by the normal push flow and may require the existing force-push confirmation
- squash/fixup reject commits whose parent is the root commit in this slice
- `space` checks out the selected commit as detached HEAD; dirty worktrees use the same explicit auto-stash confirmation as branch checkout
- `Enter` opens a Files subpanel for the selected commit
  - rows use the same tree display as the Files panel
  - file rows show changed-file status markers: `A`, `M`, `D`, `R`, `C`, `T`,
    or `?`
  - the subpanel keeps the parent Commits panel height even when the selected commit has only a few changed files
  - folder rows can be expanded/collapsed with `Enter`
  - `j` / `k` move the file cursor, and Details follows with the selected file's patch in that commit
  - selecting a folder shows a combined diff for descendant files in that commit
    without expanding the folder into a long path argument list
  - `v` enters visual multi-select at the current file or folder row; `j` /
    `k` updates the selected range, Details follows the selected file/folder
    pathspecs, and `Esc` exits multi-select before closing the subpanel
  - `Esc` returns to the parent Commits panel and restores the selected commit diff
  - additional commit-files local shortcuts are deferred
- `/` searches loaded commit rows by visible row identity: short hash, author
  initials, and summary/message first line; confirming or navigating matches
  refreshes Commits Details
- inside Commit Files, `/` searches changed-file paths; confirming or
  navigating matches refreshes the selected file/folder diff

Stash panel rules:

- `/` searches stash id plus summary and updates only the selected Stash row;
  Stash Details remain a placeholder in this slice

All features are keyboard-driven and deterministic.

The visual theme is Unicode/Nerd Font first. Panel titles and file, branch, and
status rows use a compact semantic icon set while preserving deterministic text
layout. Every terminal panel uses rounded corners and its own complete border;
adjacent panels do not share edges.

Themes are selected by a JSON configuration loaded once before terminal setup.
The `default` and `tokyo-night` presets support partial semantic color overrides
for panels, selections, badges, commits, files, diffs, and modals. Cursor and
batch selection replace all text colors within selected rows; modal lists,
including the command palette, share a full-width selection pair. Native Git
ANSI history and log colors remain intact.

`RATAGIT_CONFIG` selects an explicit file, which must exist. Otherwise the app
uses `%APPDATA%\ratagit\config.json` on Windows, or
`$XDG_CONFIG_HOME/ratagit/config.json` on Unix with a
`$HOME/.config/ratagit/config.json` fallback. A missing implicit file uses the
default theme; invalid configuration stops startup with a path-bearing error.
Fixed RGB selection and diff pairs are contrast-validated; terminal-defined
colors are not guaranteed by that validation. See [Themes](THEMES.md) for the
exact JSON shape, role reference, contrast constraints, and a runnable example.

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
