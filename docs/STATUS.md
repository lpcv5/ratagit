# Ratagit Status Board

> Last updated: 2026-03-20
> Purpose: execution tracking for rolling milestones.

## Current Mode

- Development model: rolling milestones.
- Active milestone: `M2 Advanced Workflow Foundation`.
- Milestone doc: `TBD (next milestone doc to be created during M2 planning)`

## Milestone Progress

| Milestone | Status | Notes |
|------|------|------|
| M1 Core Workflow Hardening | Done | visual selection + batch stage/unstage + commit editor + guard |
| M2 Advanced Workflow Foundation | In Discussion | scope locking for stash/remote/history entry |
| M3 Parity and Reliability | Planned | async path, large repo behavior, search/filter |
| M4 Release Readiness | Planned | CI, packaging, release artifacts |

## M1 Checklist

- [x] Visual selection mode in Files (`v` enter/exit, `Esc` exit).
- [x] Range-based batch stage/unstage via `Space`.
- [x] Directory coverage behavior for full subtree selections.
- [x] Commit guard when no staged change.
- [x] Dedicated commit editor panel (`message + description`, `Tab` switch, multiline description).
- [x] Stable post-commit refresh/focus behavior.

## Feature Parity Scorecard (vs lazygit core usage)

Scoring: 0 = none, 0.5 = partial, 1 = usable.

| Capability | Score | Evidence |
|------|------|------|
| Repo status overview | 1.0 | Status lists + file tree + diff panel |
| File navigation | 1.0 | j/k + panel switching + tree expand/collapse |
| Stage/Unstage workflow | 1.0 | Space toggles stage/unstage on selected file |
| Commit creation | 0.8 | `c` opens input mode and creates commit |
| Commit browsing | 0.9 | Commit list + selected commit metadata and patch preview |
| Branch browsing | 0.8 | Branch list/current marker exists |
| Branch operations | 0.7 | checkout/create/delete implemented from Branches panel |
| Stash browsing | 0.7 | Stash list exists |
| Stash operations | 0.0 | create/apply/pop/drop missing |
| Remote operations | 0.0 | fetch/pull/push missing |
| Rebase/cherry-pick | 0.0 | not implemented |
| Conflict recovery UX | 0.0 | not implemented |

Current parity index (simple average): 0.58

## Active Risks

- Advanced operations may exceed practical `git2` coverage for full UX parity.
- Current test coverage is low; regressions are likely during rapid feature work.
- Async command model exists but is not yet integrated in runtime.
