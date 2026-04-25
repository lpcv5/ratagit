## Goal

ratagit aims to replicate lazygit UX with a Rust + ratatui implementation.

---

## Core Features

- status view
- file staging
- commits
- branches
- stash

---

## MVP v0 Scope

MVP v0 includes all five panels with minimum write capabilities:

- Status: refresh repository data
- Files: stage and unstage selected file
- Commits: create commit and refresh list
- Branches: create branch and checkout selected branch
- Stash: stash push and stash pop selected entry

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
