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

- Files: stage and unstage selected file
- Branches: create branch and checkout selected branch
- Commits: create commit and refresh list
- Stash: stash push and stash pop selected entry
- Details: show summary for the currently selected left panel item
- Log: show latest error and recent notices

Navigation rules:

- `h` / `l` cycles only in left panels: Files -> Branches -> Commits -> Stash
- `1..6` focuses Files/Branches/Commits/Stash/Details/Log directly
- top branch/focus/status summary is hidden to prioritize panels
- bottom shortcut bar shows only Git operation shortcuts for the current focused panel

All features are keyboard-driven and deterministic.

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
