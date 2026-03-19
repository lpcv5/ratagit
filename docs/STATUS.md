# Ratagit Status Board

> Last updated: 2026-03-20
> Purpose: fast project inspection and goal alignment.

## North Star

Ship a Rust implementation that can replace lazygit for core day-to-day and advanced Git workflows.

## Phase Status

| Phase | Status | Notes |
|------|------|------|
| Phase 1 (MVP Foundation) | Done | Core app loop, layout, status and diff base complete. |
| Phase 2 (Core Workflow) | In Progress | Read-side panels done; write-side workflows incomplete. |
| Phase 3 (Advanced Git) | Not Started | Only stash listing exists; actions missing. |
| Phase 4 (Parity/Reliability) | Not Started | Async, perf, and parity hardening pending. |
| Phase 5 (Release) | Not Started | CI, packaging, and release process pending. |

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

## Next Milestone (Phase 2 Exit)

- [x] Space on file toggles stage/unstage for current selection.
- [x] Commit popup/input supports create commit + validation.
- [x] Branch checkout/create/delete available with clear feedback.
- [x] Errors and success events shown in command log.
- [x] Tests for stage/unstage/commit happy path.

## Active Risks

- Advanced operations may exceed practical `git2` coverage for full UX parity.
- Current test coverage is low; regressions are likely during rapid feature work.
- Async command model exists but is not yet integrated in runtime.

## Mitigation Plan

- Keep `GitRepository` as stable boundary and add backend adapters as needed.
- Add workflow-focused integration tests using temp repositories early.
- Gate phase transitions by exit criteria, not by elapsed time.
