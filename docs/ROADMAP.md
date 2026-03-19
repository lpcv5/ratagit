# Ratagit Roadmap

> Last updated: 2026-03-20
> North Star: Build a Rust-first lazygit equivalent with predictable UX, safe Git workflows, and strong performance.

## Current State (Code-Verified)

### Completed
- Core TUI event loop and terminal lifecycle.
- TEA skeleton (`App`, `Message`, `update`, `Command`).
- Split-panel layout with Files / Branches / Commits / Stash / Diff / Command Log.
- Git repository abstraction (`GitRepository`) with `git2` backend.
- Status, branch list, commit list, stash list loading.
- File-tree expansion/collapse and diff scrolling.
- Configurable keymap in `~/.config/ratagit/keymap.toml`.

### Partially Completed
- Stage/Unstage exists in backend and app methods, but no complete end-user workflow keybinding/action path.
- Commit/Branch/Stash are currently read-oriented UI; write operations are missing.
- Async `Command::Async` exists as API shape but runtime handling is not wired.

### Not Completed
- Interactive rebase and advanced history editing.
- Remote workflows (fetch/pull/push).
- Production test coverage, CI/CD, release packaging.

---

## Phase Plan (Re-baselined)

## Phase 2: Core Workflow Completion
Target: 2-3 weeks

### Scope
- [x] Wire Stage/Unstage to user actions in Files panel.
- [x] Implement Commit flow (input, validation, execute, refresh).
- [x] Implement branch actions (checkout/create/delete).
- [x] Improve commit detail preview in Diff panel.
- [x] Add visible status/error feedback in Command Log.

### Exit Criteria
- [x] A developer can complete a daily local workflow without leaving Ratagit: stage -> commit -> checkout branch.
- [ ] No blocking crash in 30-minute manual smoke test.
- [x] `cargo check`, `cargo test`, `cargo clippy` all pass in CI/local baseline.

## Phase 3: Advanced Git Operations
Target: 3-5 weeks

### Scope
- [ ] Stash actions: create/apply/pop/drop.
- [ ] Remote basics: fetch/pull/push with progress and conflict messaging.
- [ ] Cherry-pick (single and multi-commit).
- [ ] Rebase support plan:
  - [ ] Start with non-interactive/basic rebase workflow.
  - [ ] Add interactive rebase UI/state machine (pick/squash/fixup/drop/reorder).
  - [ ] Continue/abort/skip and conflict recovery UX.

### Exit Criteria
- [ ] End-to-end branch cleanup flow works (stash/pull/rebase/cherry-pick/push).
- [ ] Conflict states are recoverable from UI.

## Phase 4: Parity and Reliability
Target: 3-4 weeks

### Scope
- [ ] Feature parity checklist against core lazygit workflows.
- [ ] Async execution path for heavy git operations.
- [ ] Search/filter in lists and commits.
- [ ] Better rendering behavior for large repositories.
- [ ] Error taxonomy cleanup and user-friendly messages.

### Exit Criteria
- [ ] Core parity score >= 80% (see `docs/STATUS.md`).
- [ ] Manual tests pass on Windows/Linux/macOS terminals.

## Phase 5: Quality Gate and Release
Target: 2-3 weeks

### Scope
- [ ] Integration tests for Git workflows using temp repositories.
- [ ] CI: build, test, clippy, formatting checks.
- [ ] Release pipeline and binary artifacts.
- [ ] User-facing docs and contributor guide consistency.

### Exit Criteria
- [ ] Stable beta release candidate published.
- [ ] No critical bug in one-week dogfooding cycle.

---

## Implementation Strategy for Complex Git Operations

For advanced workflows (interactive rebase, history rewriting):

- Keep `GitRepository` as the single app-facing boundary.
- Use mixed backend strategy where needed:
  - Prefer `git2` API when capabilities are complete and safe.
  - Use controlled `git` CLI delegation for operations where libgit2 support is incomplete or too costly.
- Normalize all results/errors into app-level message types.

This keeps UI architecture stable while enabling full lazygit-class features.

---

## Weekly Alignment Checklist

Every week, update and review:

- [ ] Completed items moved in `docs/STATUS.md`.
- [ ] New blockers/risks logged with owner and mitigation.
- [ ] Next 1-2 week goals narrowed to shippable slices.
- [ ] Any roadmap changes reflected here with date.
