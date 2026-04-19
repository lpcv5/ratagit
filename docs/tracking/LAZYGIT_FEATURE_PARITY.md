# Lazygit Feature Parity Tracker

This document tracks ratagit feature parity against lazygit beyond raw keybinding lists.

Related docs:
- Interaction spec: `docs/tracking/UI_INTERACTIONS.md`
- Architecture: `docs/ARCHITECTURE.md`

## Status Legend

- Implemented?: `- [x]` yes, `- [ ]` no
- Have test?: `- [x]` yes, `- [ ]` no

---

## 1) Core Panel Workflows

### Files workflow
- [x] Implemented? Stage/unstage single file (`Space`)
- [x] Have test?
- [x] Implemented? Stage all (`a`)
- [x] Have test?
- [x] Implemented? Amend commit (`A`)
- [x] Have test?
- [x] Implemented? Discard selected (`d`)
- [x] Have test?
- [x] Implemented? Ignore selected (`i`)
- [x] Have test?
- [x] Implemented? Open commit dialog (`c`)
- [x] Have test?
- [x] Implemented? Reset menu (`D`)
- [x] Have test?
- [x] Implemented? Enter file detail (`Enter`)
- [x] Have test?
- [ ] Implemented? Refresh files (`r`)
- [ ] Have test?
- [x] Implemented? Toggle file tree view (flat/tree)
- [x] Have test?
- [x] Implemented? Collapse/expand all directories
- [x] Have test?
- [ ] Implemented? Open file in editor/default app
- [ ] Have test?

### Branches workflow
- [x] Implemented? Navigate branch list (`j/k`, arrows)
- [x] Have test?
- [x] Implemented? Open branch commits/detail (`Enter`)
- [x] Have test?
- [x] Implemented? Delete selected branch (`d`, delete-options menu)
- [x] Have test?
- [x] Implemented? Checkout selected branch with `Space`
- [x] Have test?
- [x] Implemented? Create branch (`n`)
- [x] Have test?
- [ ] Implemented? Rename branch (`R`)
- [ ] Have test?
- [ ] Implemented? Merge/rebase from branches panel
- [ ] Have test?

### Commits workflow
- [x] Implemented? Navigate commits (`j/k`, arrows)
- [x] Have test?
- [x] Implemented? Open commit files/details (`Enter`)
- [x] Have test?
- [ ] Implemented? Cherry-pick copy/paste flow (`C`/`V`)
- [ ] Have test?
- [ ] Implemented? Reword/edit/drop/squash/fixup actions
- [ ] Have test?
- [ ] Implemented? Tag/revert from commit panel
- [ ] Have test?
- [ ] Implemented? Open in browser/copy commit attributes
- [ ] Have test?

### Stash workflow
- [x] Implemented? Apply stash (`Space`)
- [x] Have test?
- [x] Implemented? Pop stash (`p`)
- [x] Have test?
- [x] Implemented? Drop stash (`d`)
- [x] Have test?
- [x] Implemented? Enter stash detail (`Enter`)
- [x] Have test?
- [ ] Implemented? New branch from stash (`n`)
- [ ] Have test?
- [ ] Implemented? Rename stash (`r`)
- [ ] Have test?

---

## 2) Main View Modes (Diff/Merge/Staging/Patch)

- [x] Implemented? Basic main view scroll (`j/k`, arrows)
- [x] Have test?
- [x] Implemented? Page scroll (`Ctrl+u`, `Ctrl+d`)
- [ ] Have test?
- [ ] Implemented? Search inside main view (`/`, next/prev)
- [ ] Have test?
- [ ] Implemented? Staging mode line/hunk toggling
- [ ] Have test?
- [ ] Implemented? Patch-building mode (custom patch)
- [ ] Have test?
- [ ] Implemented? Merge-conflict picking mode
- [ ] Have test?

---

## 3) Multi-Select / Range Select

(from lazygit `docs/Range_Select.md`)

- [x] Implemented? Sticky range select toggle (`v`)
- [ ] Have test?
- [ ] Implemented? Non-sticky range select (`Shift+Up/Down`)
- [ ] Have test?
- [x] Implemented? Apply actions to selected ranges (where supported)
- [ ] Have test?
- [ ] Implemented? Clear/reset selection semantics parity
- [ ] Have test?

---

## 4) Undo / Redo

(from lazygit `docs/Undoing.md`)

- [ ] Implemented? Reflog-driven undo (`z`)
- [ ] Have test?
- [ ] Implemented? Reflog-driven redo
- [ ] Have test?
- [ ] Implemented? Correct limitations messaging (working tree not covered)
- [ ] Have test?

---

## 5) Search / Filter

- [ ] Implemented? Generic panel filtering (`/`)
- [ ] Have test?
- [ ] Implemented? Commit log filter menu/options
- [ ] Have test?
- [ ] Implemented? Search prompt flow and cancellation semantics
- [ ] Have test?

---

## 6) Remote & Collaboration

- [ ] Implemented? Push/pull from UI
- [ ] Have test?
- [ ] Implemented? Fetch/fetch-prune flows
- [ ] Have test?
- [ ] Implemented? PR create/open/copy URL flows
- [ ] Have test?
- [ ] Implemented? Remote management panel actions
- [ ] Have test?

---

## 7) Worktrees / Submodules / Advanced Git

- [ ] Implemented? Worktree panel and switch/create/remove
- [ ] Have test?
- [ ] Implemented? Submodule panel actions
- [ ] Have test?
- [ ] Implemented? Reflog panel actions
- [ ] Have test?
- [ ] Implemented? Tags panel actions
- [ ] Have test?

---

## 8) Config & UX Parity

(reference: lazygit `docs/Config.md`)

- [ ] Implemented? Config file loading/overrides (global + repo local)
- [ ] Have test?
- [ ] Implemented? User keybinding overrides
- [ ] Have test?
- [ ] Implemented? Main/side panel layout tuning options
- [ ] Have test?
- [ ] Implemented? Diff/staging behavior toggles (whitespace, hunk mode, wrap)
- [ ] Have test?

---

## 9) Test Backlog Buckets

Use this section to link concrete test tasks/PRs.

### High-priority missing tests (existing features)
- [ ] Global input handler keys (`q`, `1-4`, `h/l`, `?`, `Ctrl+u/d`)
- [ ] Main view paging tests (`Ctrl+u/d`)
- [ ] Help panel navigation/execute tests
- [ ] Multi-select behavior tests in files/commits

### Feature tests for new parity work
- [ ] Search/filter flow tests
- [ ] Undo/redo reflog behavior tests
- [ ] Rename branch (`R`) tests
- [ ] Patch mode interaction tests

---

## 10) Suggested Document Set in `docs/tracking/`

Created now:
- `LAZYGIT_FEATURE_PARITY.md` (this file)

Recommended next additions:
- `TRACKING_INDEX.md` — one-page status dashboard linking all trackers
- `TEST_GAP_TRACKER.md` — per-feature test matrix with owners
- `ROADMAP_PARITY_PHASES.md` — phased execution order and acceptance criteria

---

## Update Rule

When a feature changes, update both:
1. `docs/tracking/UI_INTERACTIONS.md` (interaction-level truth)
2. `docs/tracking/LAZYGIT_FEATURE_PARITY.md` (parity + test tracking)
