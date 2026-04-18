# Tracking Documentation Index

This directory contains feature parity and test coverage tracking documents for ratagit.

## Documents

### 1. [LAZYGIT_FEATURE_PARITY.md](./LAZYGIT_FEATURE_PARITY.md)
Comprehensive feature-by-feature comparison between ratagit and lazygit with implementation/test status checkboxes.

**Covers:**
- Core panel workflows (Files, Branches, Commits, Stash)
- Main view modes (Diff, Merge, Staging, Patch)
- Multi-select / Range select
- Undo/Redo (reflog-driven)
- Search/Filter
- Remote & Collaboration
- Worktrees/Submodules/Advanced Git
- Config & UX parity
- Test backlog buckets

### 2. [UI_INTERACTIONS.md](./UI_INTERACTIONS.md)
Detailed keybinding specification with per-key implementation/test status.

**Covers:**
- Global keybindings
- List panel navigation
- Panel-specific keybindings (Files, Branches, Commits, Stash, Main View)
- Modal dialogs
- Multi-select mode
- Coverage summary and implementation priorities

## Update Protocol

When implementing features or adding tests:

1. **Before starting work:**
   - Check both `LAZYGIT_FEATURE_PARITY.md` and `UI_INTERACTIONS.md`
   - Identify which checkboxes will be affected

2. **During implementation:**
   - Update `docs/tracking/UI_INTERACTIONS.md` for keybinding changes
   - Update `LAZYGIT_FEATURE_PARITY.md` for feature-level changes

3. **After completion:**
   - Mark `Implemented?` checkboxes as `[x]`
   - Add tests and mark `Have test?` checkboxes as `[x]`
   - Update both documents in the same commit

## Recommended Future Additions

- `TEST_GAP_TRACKER.md` — Per-feature test matrix with owners
- `ROADMAP_PARITY_PHASES.md` — Phased execution order and acceptance criteria
- `TRACKING_INDEX.md` — One-page status dashboard (this file can evolve into that)

## Quick Status

Run these commands to get current status:

```bash
# Count implemented features
grep -c "\[x\] Implemented?" docs/tracking/LAZYGIT_FEATURE_PARITY.md

# Count tested features
grep -c "\[x\] Have test?" docs/tracking/LAZYGIT_FEATURE_PARITY.md

# Count implemented keybindings
grep -c "| ✅ |" docs/tracking/UI_INTERACTIONS.md

# Count tested keybindings
grep -c "| ✅ | ✅ |" docs/tracking/UI_INTERACTIONS.md
```

## Related Documentation

- [ARCHITECTURE.md](../ARCHITECTURE.md) — System architecture and event-driven design
- [CLAUDE.md](../../CLAUDE.md) — Development guidelines and conventions
