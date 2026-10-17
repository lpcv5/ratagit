# DESIGN.md

## Left-panel tabs

The Files, Branches and Commits physical panels contain sibling repository tabs;
Stash stays single-view. Core owns the active tab and independent linear
selection/scroll for each list. ActiveLeftView and details_left_view resolve the
tab before any original list subview. New lists use typed RepositoryItem data,
loaded on demand through GitBackendRead, and synchronous metadata Details
projections. Pending list reads own monotonic request IDs; refresh and mutation
scheduling invalidate them, while runtime generations reject reads crossing a
write. A completed read may populate an inactive tab but cannot replace another
tab's Details. Search is scoped to a tab and cleared on switching.

Headers retain focus badges and distinguish the active tab with bold underline.
At widths that cannot fit all siblings, render the active title. Lists show
loading, errors and empty states explicitly; their caption states the read-only
1000-entry bound. The keyboard and reducer both exclude original-list actions
on metadata tabs. Arrows and brackets cycle siblings with wrap in normal input;
modal/editor/search/patch input retains precedence. Tab keeps its existing
tracking-checkout and commit-patch meanings.

## Common Git workflows

`ui.workflow` owns typed menu/prompt/confirmation state and copied commit IDs.
Menus capture operations/targets instead of looking up a changed cursor when
confirmed. Input translates keys into WorkflowAction; reducers handle contextual
intent, Unicode edits and confirmations. The pure workflow modal shares existing
geometry and styles. Clipboard and editors remain host effects; completion is
dispatched to core. External message files use unique create-new paths and RAII
cleanup. Git commits still run through GitBackend on the exclusive worker.
Modal and palette input own typed characters before host shortcuts. Editor
preferences are selected before parsing the winning command. Confirmations keep
consequence text ahead of bounded path previews, including for large selections.

Typed operations reuse RepositoryOperation metadata, results and mutation barrier.
Conflict-producing failures refresh without automatically aborting. `m` detects
the active Git sequence in the backend. Abort, hard reset, conflict replacement,
reorder and autosquash have captured confirmations. Reorder uses deterministic
rebase scripts and rejects root/merge boundaries. Cherry-pick reverses newest-first
selection to oldest-first application and retains copied IDs on failure.

Filters/flat layout change the cached Files projection, not repository rows.
FileEntry records staged and unstaged independently, so partially staged paths
appear in both filters and stage-all stages their remaining worktree changes.
Folder previews and writes resolve filtered descendants, including in large-repo
mode. Expand/collapse uses the existing tree. Remote browsing uses a read command,
request ID and cancel/focus checks; runtime generations reject stale reads crossing
mutations. Tracking checkout validates names and requires a clean worktree.
Non-current fast-forward uses Git's local fetch ref-update protections.
Automatic stash compares object IDs, restores the exact stash with `--index`,
and drops it only after successful restoration with a matching top entry.
Conflict-marker validation follows the path's `conflict-marker-size` attribute.

## Details and runtime design

The Details diff uses stable unnumbered diff content, `>` cursor and `┃` mark
gutters, distinct staged/unstaged add/remove backgrounds, cursor versus mark
colors. The hunk counter and inline keyboard-help rows are removed. The existing layout and
confirmation modal remain. Details uses explicit row styles so List cursor
highlighting cannot overwrite multiple selected lines or their add/remove colors.
Native source patch bytes are separate from combined display rows. A display
mapping never authorizes a write; missing/ambiguous mappings fall back to source
previews. Hunk/line selection, independent marks, visual anchors and side-specific
cursors/scroll offsets live in AppContext. Window resize supplies explicit
viewport data to the reducer.

Passive Files and Commit Files previews project bounded ANSI diff text into
read-only coordinates;
interactive patches project reviewed native rows. Both feed the same pure
`diff_rows` component for cursor/mark indicators and semantic colors. Old/new
line-number columns and their separator are omitted from both modes. Native file
headers are retained separately in `PatchFile.header`, including read-only and
truncated patches. The renderer preserves diff prefixes and content indentation;
only the gutter and semantic background colors are added. It expands tabs at
eight-column stops relative to the original diff content. Synthetic section
labels supply staged colors without becoming rendered rows. Core uses the same
read-only row projection for scroll bounds. Preview rows never receive writable
LineIds; source requests and reviewed-byte mutation checks stay unchanged.

When Files hands focus to Details, pure layout continues to allocate the Files
expansion using last_left_focus. Other left-panel focus changes retain their
existing layout rules. Details always reserves its full border, keeping diff
coordinates stable across focus changes. Entry preserves the current scroll;
while LoadPatch is pending without a reviewed patch, the renderer keeps the
existing Files preview. Success replaces it with reviewed rows; failure shows
the patch error. Only reviewed data can authorize partial writes.

Executable contextual help is integrated into the command palette with
text filtering.
Editor and clipboard services stay in the application layer; pure input derives
host requests, terminal ownership stays on the main thread, and completion is
dispatched to core. Git path resolution remains in ratagit-git.

Text caches retain at most 16 entries and 8 MiB of owned value/key capacity per
kind (files, branches, commits), below a combined 24 MiB payload budget. Cache
hits promote recency. Refreshes and mutations invalidate mutable cached data.

## MVP Design

ratagit MVP uses a left-nav workspace interface with six visible panels:

1. Left column (top -> bottom): Files, Branches, Commits, Stash
2. Right column (top -> bottom): Details, Log
3. Bottom row: unframed current-focused-panel local shortcuts plus `? commands`

The focused panel is highlighted with a border/title accent. Selectable rows use
color-only cursor highlighting: the selected row is highlighted only inside the
focused selectable panel, and inactive panels render their selected row as plain
text. Left list panels keep deterministic selected row indexes. Right panels are
read-only views derived from `AppContext`. The app does not render a top
branch/focus/status summary.
Panel titles include numbered focus hints `[1]..[6]`; the terminal UI renders
the numbers as reverse-video badges.

Focus model:

- default focus starts at `Files`
- `h` / `l` map to `FocusPrev` / `FocusNext` and cycle only left panels
- `FocusPanel` supports direct focus selection (`1..6` in app input map)
- `AppContext` is pure categorized state: `repo` stores Git/backend-derived
  data, `ui` stores focus/search/selections/scroll/modals/projection caches,
  and `work` stores pending command/loading state
- `AppContext.ui.last_left_focus` tracks the last active left panel for `Details` projection
- `AppContext.repo.details` stores files-detail diff text, target paths, and detail-refresh errors
- `AppContext.repo.details` also stores a bounded files-detail diff cache keyed by the
  exact target path list
- `AppContext.repo.details` stores commit-detail diff text, selected commit target,
  detail-refresh errors, and a bounded commit-detail cache keyed by full commit id
- Details text is stored as backend output. Files and Commit Files diffs project
  into read-only rows for the shared diff component; full-commit history and
  branch-log text retain the pure ANSI-to-span projection.
- `AppContext.ui.details.scroll_offset` stores the global Details viewport offset
  used by `Ctrl+U` / `Ctrl+D`; the action carries terminal-size-derived scroll
  and visible-line counts
- `AppContext.ui.editor` stores active commit/stash editor modal state (type + fields + cursor indexes + scope)
- `AppContext.ui.reset_menu` stores whether the Files reset menu is active and which reset choice is selected
- `AppContext.ui.discard_confirm` stores whether the discard confirmation modal is active and the resolved target paths
- `AppContext.ui.stage_all_confirm` stores the active staged-required fallback:
  the original operation intent and deterministic file paths that will be staged
  before continuing
- `AppContext.ui.search` stores generic panel-scoped search input, query, matches,
  and current match index for searchable left panels and subpanels
- `AppContext.ui.command_palette` stores the active executable command palette
  state and selected command index; command entries are derived
  deterministically from the current `AppContext`
- `AppContext.repo.commits.items` stores commit rows while
  `AppContext.ui.commits` stores cursor selection, visual selection anchor, and
  selected visual range
- `AppContext.repo.commits.files.items` stores Commit Files backend rows while
  `AppContext.ui.commits.files` stores the active subpanel, selected commit id,
  tree projection, and cursor scroll state
- `AppContext.repo.branches.items` stores local branch rows while
  `AppContext.ui.branches` stores active branch creation, delete menu, rebase
  menu, and auto-stash confirmation state
- `AppContext.repo.branches.commits` and
  `AppContext.repo.branches.commit_files` store Branches subview data separately
  from the main Commits panel; `AppContext.ui.branches` stores the active
  subview and reuses commit-list and commit-file tree UI state for isolated
  Branches interactions
- `AppContext.work` stores visible pending refresh/details/operation state and
  the last completed command label through typed refresh, details, mutation,
  pagination, and Commit Files substates
- `AppContext.work.details` stores the active Details request id and target so
  stale async Details results cannot clear or replace the latest request
- `AppContext.repo.status` stores Git status performance metadata:
  `index_entry_count`, `large_repo_mode`, `status_truncated`,
  `status_scan_skipped`, and `untracked_scan_skipped`
- `AppContext.repo.details.files_diff_truncated_from` stores the original target count
  when Files Details diff output is limited to the first 100 targets
- left-panel height baseline follows the Files/Branches/Commits/Stash ratio
- reserve three content rows per left panel before distributing remaining height
  by ratio; when `Stash` is unfocused it keeps those three rows and freed height
  is redistributed by ratio to Files/Branches/Commits
- when focus is in Files/Branches/Commits and content overflows, the focused
  panel borrows height from other left panels in round-robin order, preserving
  at least three content rows in every donor
- complete left-panel frames with this reserve need 20 body rows (21 terminal
  rows including shortcuts); smaller terminals reduce the common content reserve
  evenly, conserving available height without overlap
- Details/Log retain their 70/30 split, clamped to three content rows per panel
  whenever both fully bordered panels fit

---

## Interaction Model

- Input is mapped to explicit `UiAction`.
- The input layer derives an explicit `InputMode` from `AppContext` before
  routing keys. Global `Ctrl+U` / `Ctrl+D` are handled first, modal/editor/search
  modes keep priority over panel shortcuts, and confirmed search queries can
  fall through to panel shortcuts only after search-specific keys are checked.
- `?` maps to `OpenCommandPalette` from normal panel and confirmed-search-query
  flow. The palette owns navigation and search-input modes before panel shortcuts.
  Navigation uses `j` / `k` or arrows to select, `/` to enter search, `Enter` to
  execute the selected command, and `Esc` to close. Search input accepts literal
  characters (including j/k), Backspace, and arrow selection; `Enter` or `Esc`
  finishes editing and retains the filter without executing. `/` in navigation
  resumes editing the retained query. Other ordinary navigation-mode characters
  are ignored, including `q`; selecting Quit explicitly or `Ctrl+C` quits.
- `Ctrl+U` and `Ctrl+D` map to global Details scroll actions before
  mode-specific key handling. The input layer computes the step as
  `max(1, details_content_height * 2 / 5)` from the current terminal layout.
- `update()` applies state transitions and emits `Command`.
- Command execution is delegated to `GitBackend`.
- The real TUI sends read-only commands to a fixed background Git worker pool
  and mutating commands to one exclusive write worker, then receives
  `GitResult` values through a channel; the mock harness can still use the
  synchronous runtime for deterministic scenario tests.
- Refresh-all fan-outs to independent Files/status, Branches, Commits, and
  Stash read commands. Each panel applies its own result into `AppContext` as soon
  as a worker finishes, so slow file status collection does not block the other
  left panels from rendering available data.
- The async runtime defers new read commands while a mutation is in flight and
  drops stale read results that were started before a queued mutation.
- Runtimes coalesce redundant queued repository refresh and files-detail diff
  commands after the most recent mutation command.
- Staged-required operations share one fallback model: if no files are staged
  and current file rows exist, `update()` opens an AppContext-owned stage-all
  confirmation; confirming emits a compound mutating command that stages those
  paths through `GitBackend` and then runs the original operation.
- Sync and async runtimes share the same scheduler implementation. The scheduler
  uses `Command` metadata for mutating barriers, debounce keys, and refresh
  coalescing keys instead of duplicating command matches in each runtime.
- Backend output re-enters `update()` as `GitResult`.
- UI rendering reads immutable `AppContext`, terminal size, and explicit
  `RenderContext` inputs, including the resolved theme and spinner frame.
- High-frequency side effects use runtime command debouncing keyed by
  `ratagit_core::debounce_key_for_command`, so rapid navigation can collapse to
  the latest command while keeping state transitions deterministic.

Search interaction:

- `/` activates `AppContext.ui.search` for the current Files, Branches, Commits,
  Stash, or Commit Files scope.
- Search input replaces the bottom shortcuts with `search: <query>` until
  `Enter` confirms or `Esc` cancels.
- Normal bottom shortcuts list only panel-specific common actions; baseline
  navigation/search keys and global sync keys are omitted, including the generic
  `v` visual-mode entry key. The final normal shortcut is always `? commands`.
- Bottom shortcut keys render as reverse-video badges, omit the old
  `keys(panel):` prefix, and use spaces instead of pipe separators.
- The command palette renders as an executable modal with `Local commands` and
  `Global commands` sections. Section labels are not selectable; command rows
  are selectable, viewported around the selected command when the list is taller
  than the modal content, and execute existing reducer actions rather than
  introducing separate command semantics. Palette search state belongs to
  `AppContext.ui.command_palette`, independently of panel search. Opening the
  palette clears its query and starts navigation; hints switch between
  `j/k ↑/↓ select`, `/ search`, `Enter run`, `Esc close` and search-editing help.
  The filter row is visible during search even before any text is entered, and
  stays visible for a nonempty confirmed query.
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

- `AppContext.repo.files.items` stores Git-derived file rows while
  `AppContext.ui.files` stores tree expansion, visible-row selection, visual
  selection anchor, and batch rows.
- Workspace file rows carry both action metadata (`staged`, `untracked`) and
  display metadata (`status`, `conflicted`) so rendering can show Git status
  characters without rediscovering status outside `AppContext`.
- File tree rows are derived from `RepoSnapshot.files`; no UI code reads external state.
- `AppContext.ui.files.tree_rows`, `row_descendants`, and `row_index_by_path`
  cache deterministic tree projection data after reducer-managed changes.
- `AppContext.ui.files.tree_index` and `AppContext.ui.commits.files.tree_index` share the
  same deterministic parent/child tree index. Expanding or collapsing one
  directory rebuilds visible rows from cached children instead of rescanning
  every file path.
- Tree indexes sync item changes by removing, adding, or metadata-updating
  changed source paths. When the path topology is stable, status-only refreshes
  update node metadata without rebuilding child relationships.
- Files Details target resolution is bounded during navigation: file rows resolve
  directly from the selected tree row, and directory rows use cached tree index
  source counts to collect only the first 100 diff targets before reporting the
  original total.
- Files and Commit Files compact single-child directory chains into one display
  row, such as `src/a/b/c/`, while keeping the row's real path and descendant
  mapping in `AppContext` for selection, search, Details, and Git commands.
- Files and Commit Files share one tree row renderer. Directory rows keep their
  expand/collapse icons; file rows render Git status markers (`A/M/D/R/C/T/?`),
  workspace conflict rows append `U`, and staged workspace file names use the
  staged-file style.
- Backend status collection uses full untracked-file expansion in small
  repositories so untracked nested files appear as explicit file rows in the
  tree.
- When the backend reports large repo fast mode, the Files tree initializes
  collapsed and uses lightweight projection data to avoid constructing
  `row_descendants` for every path during initial load or folder toggles.
- Large repo fast mode skips full untracked expansion. The Log panel renders a
  notice for the skipped untracked scan and a manual Git config tip rather than
  changing repository configuration automatically.
- When the backend reports metadata-only huge repo mode, the Files tree remains
  empty for that refresh and the Log panel renders a deterministic file-scan
  skipped notice. No Files Details diff command is emitted for the empty
  selection.
- Status output is bounded by backend limits. `AppContext.repo.status.status_truncated`
  drives a deterministic Log notice when the result was capped.
- Real backend status collection prefers Git CLI porcelain v1 `-z` output inside
  `GitBackend` for large-repository speed and falls back to git2 status if the
  CLI command fails.
- Directories are display targets only and resolve to descendant files from the current snapshot.
- `space` toggles stage state for the current target or visual-selected batch.
- `c` opens commit editor modal:
  - commit message + multiline body fields
  - active field cursor is stored in `AppContext.ui.editor` and rendered with the terminal cursor
  - `Left` / `Right` / `Home` / `End` moves within the active field
  - `Tab` / `Shift+Tab` field switching
  - `Ctrl+J` inserts newline in body
  - `Enter` confirms, `Esc` cancels
- `s` opens stash editor modal:
  - normal files mode -> stash all changes, including untracked files
  - visual multi-select mode -> stash selected target paths only
  - title cursor is stored in `AppContext.ui.editor` and rendered with the terminal cursor
  - `Left` / `Right` / `Home` / `End` moves within the title field
  - `Enter` confirms, `Esc` cancels
- `D` opens a repository reset modal:
  - choices are `mixed`, `soft`, `hard`, and `Nuke`
  - `j` / `k` or arrow keys select an option
  - the selected option controls the description rendered below the list
  - `Enter` emits mixed/soft reset commands immediately; hard reset and Nuke
    require danger confirmation. `Esc` cancels the menu or confirmation.
  - `Nuke` performs hard reset semantics followed by `git clean -fd`
- `d` opens a file-targeted discard confirmation modal:
  - targets are resolved when the modal opens from the current cursor target or visual-selected rows
  - directory rows resolve to descendant files in the current snapshot
  - `Enter` emits `Command::DiscardFiles`, `Esc` cancels
  - confirmation text and path summary are rendered only from `AppContext.ui.discard_confirm`
- `v` enters visual multi-select at the current row; `j` / `k` updates the continuous anchor-to-cursor range; `Esc` exits visual multi-select.
- `/` switches the bottom keys area into search input until Enter or Esc using
  the generic search model.
- Long file lists keep a stable bottom-reserve viewport while reversing from
  downward movement; moving up does not jump to a top-reserve viewport.
- `RefreshAll` and files selection navigation emit `RefreshFilesDetailsDiff` so
  the Details panel follows the current files cursor.
- Files Details commands carry deterministic `FileDiffTarget` values
  (`path`, `untracked`, `is_directory_marker`) instead of asking the backend to
  rediscover status metadata.
- Files Details tracked patches come from bounded `git diff --color=always`
  output; untracked file patches remain generated by the backend's safe
  formatter because plain Git diff does not include untracked contents.
- Folder or visual multi-select Details diffs are limited to the first 100
  resolved targets in core before the command is emitted. The Details and Log
  panels render the original total from `AppContext.repo.details.files_diff_truncated_from`.
- Unknown untracked directory markers do not emit a Git diff command when large
  repo mode skipped untracked scanning; Details renders a skip message from
  `AppContext.repo.details`.
- stale details-diff results are ignored when their target list and truncation
  metadata no longer match the current Files selection or active request id.
- while editor, reset, or discard modal is active, modal key handling has highest input priority
  over panel navigation mappings.
- Branches focus maps `space` to checkout, `v` to enter visual multi-select,
  `n` to new branch, `d` to delete, `r` to rebase, and `Enter` to a commits
  subview for the selected branch.
- Branch Commits reuses the main commit-row formatter and commit selection
  helpers against Branches-owned state; `Enter` opens Branch Commit Files and
  `Esc` returns to the branch list.
- Branch Commit Files reuses the existing Commit Files tree projection,
  renderer, and Details patch logic against Branches-owned commit-file state;
  `Enter` expands/collapses directories and `Esc` returns to Branch Commits.
- Branch visual multi-select is AppContext-owned and follows the same continuous
  anchor-to-cursor model as Files and Commits; `Esc` exits the mode.
- Branch creation opens an AppContext-owned input modal and emits
  `Command::CreateBranch` with the selected branch as `start_point`.
- Branch checkout and rebase inspect current AppContext file status; when dirty,
  they open an auto-stash confirmation modal before emitting commands with
  `auto_stash=true`.
- Branch deletion opens an AppContext-owned select list for local, remote, or both:
  local deletion is blocked in the reducer when the selected branch is current,
  and `GitBackend` also blocks branches checked out in any worktree. The real
  backend first tries `git branch -d`; if Git reports the branch is not fully
  merged, the reducer stores a force-delete confirmation modal in `AppContext`.
  Confirming emits the same deletion command with `force=true`, which maps local
  deletion to `git branch -D`.
- Branch rebase options are simple, interactive, and `origin/main`; simple and
  interactive rebase the current branch onto the selected branch.
- Branches search selects matching branches and refreshes the branch details log.
- Commits focus maps `A` to amend staged changes into the selected commit, `s`
  to squash, `f` to fixup, `r` to reword, `d` to delete, `space` to detached
  checkout, `v` to enter visual multi-select, and `Enter` to open Commit Files
  for the selected commit.
- Commit visual multi-select is AppContext-owned and follows the same continuous
  anchor-to-cursor model as Files visual selection; `Esc` exits the mode.
- Commit rewrite commands can operate on linear pushed or unpushed history and
  are executed only through `GitBackend`; squash, fixup, reword, and delete run
  scripted Git interactive rebase and require a clean working tree, while amend
  requires staged changes with no unstaged or untracked changes unless the
  shared stage-all fallback is confirming an operation that started with no
  staged files. Merge commits are rejected in this slice.
- Squash/fixup reject root-parent targets in this slice because the interactive
  rebase path does not use `git rebase --root`.
- Commit reword reuses the existing commit editor modal with a reword intent
  stored in `AppContext.ui.editor`; confirming emits `Command::RewordCommit`.
- Commits search selects matching loaded commits and refreshes the commit diff;
  it does not request additional pages.
- Detached commit checkout uses `Command::CheckoutCommitDetached`; when the
  working tree is dirty it opens the shared auto-stash confirmation modal before
  dispatching with `auto_stash=true`.
- Commit Files is an AppContext-owned subpanel under Commits:
  - opening emits `Command::RefreshCommitFiles` for the selected commit
  - rows reuse the shared Files tree projection/rendering shape but use commit
    changed-file status markers (`A/M/D/R/C/T`) instead of working-tree status
  - status markers are colored by Git status while file names keep the default
    foreground
  - `j` / `k` move the Commit Files cursor through the shared panel navigation
    action path
  - `Enter` toggles the selected directory row or opens a file's native commit
    patch through `Command::LoadPatch` / `PatchRequest::CommitFiles`. The request
    captures the commit OID and literal new/old paths, including rename sides
  - Details follows the selected file or folder via
    `Command::RefreshCommitFileDiff`
  - directory selections request one combined patch using the directory
    pathspec, avoiding large descendant path argument lists
  - stale commit-files and commit-file-diff results are ignored when the user
    has moved to another commit or file
  - `v` enters AppContext-owned visual multi-select for the changed-file tree;
    `j` / `k` updates the continuous range, and Details uses the selected
    file/folder pathspecs
  - `Esc` exits visual multi-select first; outside multi-select it closes the
    subpanel and restores the selected commit diff
  - dynamic height calculations use the parent Commits list length so the
    subpanel keeps the same height even when a commit has only a few changed
    files
  - `/` searches changed-file paths and refreshes the selected file/folder diff
  - `workflow::commit_files` resolves selected files/directory descendants and
    visual ranges before opening dialogs. Checkout and original-commit removal
    use captured `Workflow::Operation` targets through `RepositoryOperation`
  - `Space` toggles persistent inclusion through `custom_patch`. Immutable
    native source files live in `repo.custom_patch`, per-file whole/line sets
    and the source OID in `ui.custom_patch`, and a serial intent queue plus
    request-ID/target ownership in `work.custom_patch`. IDs within selections
    are local hunk/line coordinates, independent of visible file indices
  - `LoadCustomPatch` and `RenderCustomPatch` are read-only commands independent
    of Details loading/debouncing. Rapid file/hunk toggles retain input order;
    options await preceding toggles. Reset, source replacement and mutations
    invalidate queued/pending reads while completed inclusion survives refresh
  - file toggles promote partial to whole, then remove whole selections. Hunk
    toggles use the first selected changed line's membership to choose add or
    remove for the full range/marks. Whole files demote to line sets when hunks
    are removed. These sets are separate from temporary visual anchors/marks
  - Ctrl+P builds the aggregate preview/copy bytes and captures all selected
    files for confirmed apply/reverse/history-removal workflow operations.
    Native bytes are validated again against the immutable source before one
    combined Git apply --check/application. Binary/rename/mode/empty-file changes
    retain whole native patches; partial text reuses the native line builder
  - source changes ask before replacing existing inclusion. Navigation/refresh
    do not discard it; successful history removal clears it. Preview is read-only
    and bounded to 5000 rendered lines, independently of complete output. The
    builder rejects incomplete loads and more than 1024 files/16 MiB of native
    source data. Clipboard output is offered only when patch bytes are UTF-8
  - historical patches start in Hunk mode and reuse block/line selection,
    ranges, independent marks, cursor scrolling and file navigation. Contextual
    help exposes historical actions; editor/copy routes retain modal ownership
    Mode and current file are projected into the Details bottom border, using
    the same caption chrome as read-only custom patch preview metadata. Cycling
    modes updates that caption without adding content rows. Passive main/Branch
    Commit Files diffs share the hunk/aggregate preview renderer's three-column
    gutter, hunk color and add/remove backgrounds, replacing raw Git ANSI colors
  - checkout and worktree patch writes refresh Files while retaining the commit
    file context. Successful history rewrites close that context and refresh all
    repository panels. Failed history preparation retains the context
  - checkout uses literal NUL-delimited pathspec files, preserves the index,
    and rejects changed selected paths or ignored/untracked obstructions
  - original-commit removal restores parent tree entries for whole files,
    including binary/mode/rename data. Partial removal applies reviewed native
    bytes in reverse to a temporary index. Replacement commits and descendants
    are prepared in memory, preserving authors/messages. Merge ranges, foreign
    commits, detached HEAD and replay conflicts are rejected before updating the
    branch. Checkout preflight also protects ignored obstructions; Git reset
    --keep performs the final index/worktree/branch update and records a reflog
  - persistent patch inclusion is available in both commit-file subviews and
    visible in file/diff gutters, including unfocused panels. Partial binary/
    rename/type/mode/truncated patches remain non-writable; native preview caps
    do not affect the separate whole-file tree-removal operation
  - patch presentation reuses the shared file-tree, diff, shortcut-badge and
    command-menu row components. The reserved tree separator carries a thin
    whole-file rail or dotted partial rail without shifting status/name columns;
    batch and search gutters remain separate. Theme info colors persist in
    inactive rows; cursor/batch row styles retain precedence. Included source
    lines use a thin rail; temporary marks retain their heavy rail and the
    existing diff backgrounds. Accumulation never replaces styled shortcuts
    with plain text. PanelProjection supplies optional bottom-border captions:
    Commit Files shows the patch count; Custom Patch preview shows count, short
    source, read-only status and any 5000-line display cap. The diff body has no
    custom header/help rows. Options share the palette's aligned key/label rows
- Files Details projection renders merged `unstaged` and `staged` diff sections
  for current file/folder targets from `GitBackend`.
- Commits Details projection renders the selected commit's header and bounded
  patch diff preview from `GitBackend`; automatic full-commit previews are
  capped at 1 MiB and include a deterministic truncation notice when capped.
- Commit Files Details projection renders the selected commit file or folder's
  patch from `GitBackend`; backend output is capped at the same 1 MiB preview
  limit used by automatic commit previews.
- Files, Branches, and Commits Details projections apply the
  `AppContext.ui.details.scroll_offset`; empty and error rows ignore the offset.
- Pending Details refreshes keep the previous Details content visible until new
  content or an error result arrives; rendering does not replace content with a
  transient loading row.
- Details scroll resets when the selected details target changes or accepted
  details content refreshes.
- Details downward scroll clamps `scroll_offset` to the last visible page
  (`content_len - visible_lines`) to avoid hidden overscroll at the bottom.
- Repeated Files Details selections reuse the `AppContext.repo.details` diff cache without
  emitting a new Git command; the cache is cleared after snapshot refreshes and
  successful mutating Git operations.
- Repeated Commit Details selections reuse the `AppContext.repo.details` diff cache without
  emitting a new Git command; the cache is cleared after snapshot refreshes and
  successful mutating Git operations.
- Stash Details projects the selected stash's read-only native patch;
  request ownership prevents stale results from replacing the current selection.

Panel projection and row source:

- Every rendered panel is projected through a shared descriptor containing panel
  focus identity, focused state, title metadata, legacy text title, and content
  rows.
- `PanelLine` stores styled spans as the canonical row representation; plain text
  snapshots derive text from those spans so terminal rendering and compatibility
  text tests share one row source.
- Files, commits, search-highlighted rows, and selected rows build their plain
  text from the same span-backed formatters used by the terminal renderer.

---

## Error Presentation

- Git failures never crash the app.
- Errors are stored in `AppContext.repo.status.last_error`.
- The `Log` panel displays the latest error.
- Git failures carry typed failure kinds plus user-facing messages. Reducers use
  the kind for semantic recovery flows such as force-push and force-delete
  confirmations while preserving the displayed message.
- Commands expose centralized metadata for logging labels, mutating
  classification, pending operation labels, debounce keys, and refresh
  coalescing keys.
- The `Log` panel displays pending refresh and Git operation state while work is running.
- Details refresh progress is reflected in `AppContext.work.details` for command
  tracking, not as a transient Details panel row.
- Empty-state placeholders such as `<empty>` / `<none>` are not rendered; empty
  views remain visually blank.

---

## Visual Theme

- The UI assumes Unicode/Nerd Font support.
- Every terminal panel owns a complete rounded border with all four edges,
  regardless of focus. Adjacent panels occupy separate rectangles and never
  share or overwrite border cells; focus changes border/title color only.
- Panel titles include semantic icons and never use `*` to show focus.
- Terminal panel titles render the numbered focus hint as a reverse-video badge
  before the semantic icon and label.
- Directory rows use expand/collapse icons; file rows use Git status markers.
  Batch membership and search matches use distinct styles.
- Visual-selected file rows use a color distinct from cursor selection.
- Commit rows use fixed graph/hash/author/message columns; the graph is a `●`
  placeholder until a later topology pass, hash colors represent main/upstream
  reachability, and author initials use deterministic per-author colors, with a
  bright RGB blue replacing dark ANSI blue for readability on dark terminals.
- Files and Commits share the same AppContext-owned scroll direction/origin helper
  so list windows move only after the cursor crosses the three-row top/bottom
  reserve.
- Commit pagination is AppContext-owned: refresh stores the first 100 commits,
  Commits navigation prefetches the next 100 when the cursor enters the last 20
  loaded commits, successful pages append to `AppContext.repo.commits.items`,
  and loading intent stays in `AppContext.work`.
- Visible cursor markers such as `>` are not rendered; selection is tested
  through buffer styles.
- Modal overlays use the internal `ratagit-ui` modal system for deterministic
  centering, sizing, borders, inner padding, input blocks, and action footers.
- Business modal renderers call the shared modal shell with a `ModalSpec`,
  action rows, and an AppContext-derived content renderer; the shell owns the
  title, scrim, content area, footer separator, and action row rendering.
- Modal overlays render above a deterministic scrim, use rounded borders,
  centered placement, a shared footer separator, and responsive width clamping.
  Standard terminals keep compact 60% sizing; wide terminals use roughly 70% of
  the terminal width up to the shared modal maximum. Modal outer frames target
  three fifths of the terminal height while preserving small-terminal minimum
  and maximum clamps.
- `ratagit-ui` uses a shared `UiTheme` for panels, rows, loading text, commits,
  file statuses, diffs, and modals. The executable loads JSON configuration once
  before terminal setup, resolves `default` or `tokyo-night` plus semantic color
  overrides, and passes an immutable theme through `RenderContext` alongside the
  spinner frame. Rendering is pure: it receives `AppContext`, terminal size, and
  explicit render inputs rather than reading configuration or environment.
  See [Themes](THEMES.md) for discovery, JSON fields, and the complete role list.
- The default palette uses terminal-default foreground/background, yellow focus
  and cursor selection, blue batch selection, dark-gray muted chrome, and ANSI
  warning/danger/success/info colors. Tokyo Night supplies a dark RGB base and
  modal palette. Both presets share initial commit, file-status, and diff colors.
- Cursor and batch selection apply their foreground/background to every text
  span, overriding author/hash/status/search colors. Search-hit substrings retain
  underline inside these highlights. Unselected rows retain semantic colors;
  full-commit history and branch-log ANSI spans retain their Git colors;
  Commit Files diffs use the shared semantic diff colors.
- Diff rows pair tinted staged/unstaged normal, marked, and cursor backgrounds
  with explicit RGB add/remove foregrounds. Highlighted context and cursor/mark
  gutters use `diff.context.foreground`. Validation checks these text/background
  pairs at 4.5:1 or above; `diff.gutter` colors gutters without a highlighted
  background. Cursor state takes precedence over marked state.
- Overrides to cursor, batch, badge, modal-selection, and diff roles require RGB.
  Cursor, batch, and badge pairs require both final channels to be RGB and at
  least 4.5:1, except unchanged legacy default ANSI pairs. Base and effective
  modal-body pairs, plus semantic accents, file statuses, hashes, authors and
  normal diff gutters against their corresponding backgrounds, are checked at
  4.5:1 when both channels are RGB. Identical non-reset values are rejected even
  for ANSI colors. Muted chrome only rejects identical non-reset values.
  Terminal-default and named/indexed ANSI colors cannot be contrast-verified;
  validation does not guarantee their appearance in a terminal.
- Modal scrims clear underlying selection/diff backgrounds before applying muted
  text, so background highlights do not bleed through the overlay.
- Modal tones are semantic and text-backed: editor modals use an info accent,
  repository reset uses a warning accent, and discard confirmation uses a danger
  accent while still rendering explicit warning text.
- Input modal focus is border-only: the active input field uses the modal tone
  for its border and title, while input text keeps the effective modal
  foreground/background.
- Confirmation modals share the `Confirm` title with the tone icon, center a
  primary question plus secondary consequence text, and render compact
  AppContext-derived details below the message.
- Modal select lists (including reset, rebase, command palette and workflow
  menus) use `modal.selection.foreground` and `modal.selection.background`.
  The default pair is RGB dark text `#142238` on pale blue `#B4D7FF`; Tokyo Night
  uses `#C0CAF5` on `#24283B`. Validation requires both channels to be RGB with
  at least 7:1 text contrast, including custom overrides. Selection overrides
  warning, danger and muted label colors; unselected choices retain their
  semantic styles. Indicators and trailing row padding use the same selection
  pair, with one full-width highlight per list.
- Choice-menu modals reuse a shared body shape: intro text, choice list,
  description label, and description text. The shared choice list derives its
  viewport from the available choices, sizes the modal shell to fit that
  viewport when the terminal allows, and caps visible rows at ten items.
- Branch creation uses the info modal tone, branch delete uses danger, and
  branch force-delete confirmation uses danger; branch rebase plus auto-stash
  confirmation use warning.
- Files diff rows in Details are color-coded by semantics:
  - diff metadata (`diff --...`, `---`, `+++`, `index`, mode changes,
    rename/copy headers, similarity headers, binary patch headers, submodule
    headers, and no-newline markers)
  - hunk headers (`@@`)
  - additions (`+`)
  - removals (`-`)
- Interactive partial staging uses AppContext-owned files, hunks, and lossless
  source lines. Selection never derives applicable patch data from terminal text.

---

## Snapshot and Harness Design

- UI panel unit tests assert pure panel projections from `AppContext`.
- Full-screen UI tests render fixed terminal sizes through `render_terminal`
  and `ratatui::TestBackend`.
- Style-sensitive assertions inspect terminal buffer cells instead of relying on
  text snapshots for invisible color state.
- Harness scenarios drive action sequences and assert:
  - real terminal screen text
  - selected-row style when needed
  - backend operation trace
  - final mock Git state
- `docs/harness/SCENARIOS.md` indexes harness scenarios by scanning
  `fn harness_*` definitions. The manifest is checked by tests so added or
  renamed scenarios remain discoverable.
- On failure, harness writes artifacts:
  - compatibility text buffer
  - real terminal screen
  - AppContext dump
  - git operation trace
  - final mock Git state
  - input sequence
  - structured `failure_report.json` for agent-readable triage
- The manual performance suite includes pure UI operations for status rendering,
  search rendering, and Details scroll rendering in addition to backend and tree
  operations.
