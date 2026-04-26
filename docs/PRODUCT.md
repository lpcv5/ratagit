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

- Files: tree view, folder expand/collapse, stage/unstage toggle, path-limited stash, multi-select, and search
- Branches: create branch and checkout selected branch
- Commits: create commit and refresh list
- Stash: stash push and stash pop selected entry
- Details:
  - Files focus projection: show merged `unstaged` then `staged` diff for the currently selected file/folder target
  - Branches/Commits/Stash projection: placeholder text for now (to be implemented in later slices)
- Log: show latest error and recent notices

Navigation rules:

- `h` / `l` cycles only in left panels: Files -> Branches -> Commits -> Stash
- `1..6` focuses Files/Branches/Commits/Stash/Details/Log directly
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
- folder operations apply to descendant files present in the tree model
- `space` stages unstaged targets or unstages targets when all selected targets are staged
- `s` stashes current or multi-selected targets
- `v` enters visual multi-select at the current row; `j` / `k` extends or shrinks the selected range
- `/` opens search input in the bottom bar; Enter confirms, Esc cancels or clears, `n` / `N` navigate matches
- discard backend support exists, but `d` is not mapped until the confirmation dialog is available
- `Enter` still toggles directory expand/collapse; hunk editing and partial-stage flow are explicitly deferred
- details-diff side effects for high-frequency files navigation are debounced to keep `j` / `k` scrolling smooth

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
