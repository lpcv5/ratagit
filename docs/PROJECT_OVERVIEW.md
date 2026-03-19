# Ratagit Project Overview

> Last updated: 2026-03-20

## Vision

Ratagit aims to become a Rust-native lazygit equivalent: fast, keyboard-first, and capable of handling both common and advanced Git workflows without leaving the terminal.

## Current Architecture

Ratagit currently uses a TEA-style core with modular UI and repository abstraction:

- `src/app`: state (`App`), events (`Message`), state transitions (`update`), command model.
- `src/git`: `GitRepository` trait and `Git2Repository` implementation.
- `src/ui`: panel-based rendering and widgets.
- `src/config`: keymap loading and action mapping.

This architecture is intentionally designed to support a mixed backend strategy for advanced Git features while keeping UI/business logic stable.

## Current Progress Snapshot

### Implemented
- TUI lifecycle and event loop.
- Multi-panel layout (Files/Branches/Commits/Stash + Diff + Command Log).
- Git status + tree rendering + diff preview.
- Branch/commit/stash listing.
- Keymap config file with defaults.

### In Progress
- Stage/Unstage wiring in end-user flow.
- Commit creation UX and execution path.
- Branch write operations.

### Pending
- Remote operations, rebase/cherry-pick workflows, robust conflict handling.
- Async heavy-operation runtime handling.
- Comprehensive testing and release automation.

## Documentation Map

- [ROADMAP.md](./ROADMAP.md): phase goals, scope, and exit criteria.
- [STATUS.md](./STATUS.md): live progress and parity scorecard.
- [ARCHITECTURE.md](./ARCHITECTURE.md): design rationale and module model.
- [DECISIONS.md](./DECISIONS.md): major technical decisions.

## How to Stay Aligned to the Goal

Use this cadence:

1. Update `STATUS.md` after each merged feature slice.
2. Re-check roadmap exit criteria before moving phases.
3. Prioritize feature slices that increase lazygit parity and workflow completeness.
4. Keep architecture boundaries strict (`UI -> Message -> Update -> GitRepository`).

If a feature is difficult in pure `git2` (for example interactive rebase), implement it behind the same repository boundary using a controlled backend path.
