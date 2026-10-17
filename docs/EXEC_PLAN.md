# Active development plan

## Active Slice

Name: Left-panel tabs for repository navigation
Status: complete; final regression gate passed

The pre-existing repository cleanup edits remain in the working tree. Preserve
those edits, including renamed Details snapshots and verification tooling.

## Problem

The left panels expose Files, local Branches, Commits and Stash but lack the
related repository views available through lazygit's panel tabs. Users need
within-panel navigation to submodules, worktrees, remotes, tags and reflog,
with useful repository data and a matching Details preview.

## Smallest Slice

Add the lazygit panel groups: Files / Submodules / Worktrees;
Branches / Remotes / Tags; Commits / Reflog. Stash remains a single view.
Inspect current keyboard/tree/modal contracts before choosing final mappings.
Use reducer-owned tab and selection state, backend-owned repository reads,
and immutable rendering. Preserve independent selection/scroll per tab.

Acceptance: tabs cycle in both directions and wrap; inactive tabs retain their
selection; repository-backed lists and empty/error/loading states render;
Details follows the active tab without applying stale results; existing local
file/branch/commit workflows still work. Narrow terminals remain usable.
Document actual shortcuts and available operations, and cover real repository
reads plus keyboard/reducer/rendered harness behavior.

## Non-Goals

Do not add unrelated Git mutations, dependencies, or rewrite history. Tab
navigation must not invoke hidden local Files/Branches/Commits mutations.
Do not discard or commit pre-existing working-tree changes.

## Expected Files

Core state/actions/reducers/selectors; Git read traits and implementations;
harness command execution/scheduling and fixtures; UI panel titles/list/details
projections; executable key mapping and command palette; focused tests and
snapshots; PRODUCT/DESIGN and scenario documentation as appropriate.

## Tests

Extend meaningful keyboard, core, UI and harness tests for tab cycling,
selection retention, empty/loading/error states, active-tab Details and
mutation isolation. Use isolated repositories under tmp/git-tests for Git
backend coverage. Review each intended snapshot change.

## Harness Decision

New navigation is an interaction flow. Add rendered harness scenarios that
assert active tabs, repository read traces and preserved Git state. Add async
ownership coverage where new reads participate in the runtime.

## Validation

Baseline workspace tests; focused checks during implementation; cargo fmt
--check; cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace; git diff --check. Use PowerShell for native
Windows commands. Review the final diff and preserve existing cleanup edits.

## Completion Evidence

Implemented Files/Submodules/Worktrees, Branches/Remotes/Tags and
Commits/Reflog groups. Left/Right and brackets wrap sibling tabs; h/l retains
panel navigation. Tab keeps tracking-checkout and custom-patch precedence.
Typed metadata lists load on demand, retain independent selection/scroll and
project Details synchronously. Explicit refresh reloads every displayed auxiliary
list, including while Details has focus; mutation completion does the same.
Request IDs and existing runtime generations reject superseded reads.

Added core tests for wrap, retention, search scroll reset, request ownership,
mutation isolation, global dialog interaction and preserved drilldown state;
keyboard tests; deterministic UI gallery/style/narrow/loading/error tests;
synchronous and async rendered harness scenarios; isolated real Git tests for
worktrees, submodules, remotes, lightweight/annotated tags, reflog and empty HEAD.
Annotated tag messages, symbolic remote HEAD, overlapping remote names and
truncated worktree records have explicit regressions. Pending custom-patch
Options can complete after switching tabs and remain dismissable.
Adversarial regressions also cover delayed unmerged-branch delete confirmation
on Remotes (both cancel and captured force retry), variant-qualified remote vs
remote-branch identities during search/refresh, and Details scroll resetting
when focus changes between cached metadata tabs.

Reviewed the new tab gallery. Existing snapshot changes are panel headers,
plus palette rows for the new tab commands; renamed Details snapshots and
pre-existing cleanup edits remain preserved. Focused tests and all final gates
pass: cargo fmt --check; cargo check --locked --workspace; strict production
Clippy; cargo test --locked --workspace (including the three adversarial
regressions); git diff --check. Final native test log is
`tmp/tabs-adversarial-final.log`; no pending snapshot files remain.
Verified native Cargo/Rust 1.99.0; no dependencies or management mutations added.
Before committing, the staged whitespace check identified terminal-padding spaces
in the new gallery snapshot. The gallery now trims line-end padding; reviewed
snapshot differences contain only that padding removal. Focused tab UI tests pass.
