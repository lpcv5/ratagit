# Active development plan

## Active Slice

Name: Review and harden common panel workflows before committing to develop
Status: completed

## Problem

The user now requests a review, fixes for discovered issues, verification, and a
local commit on develop. Keep the full panel implementation in this commit;
preserve unrelated changes and do not push. Review covers modal key ownership,
Git argument semantics, editor lifecycle, status/projection accuracy, async reads
and backend capability behavior.

Primary risks are changing index/worktree data during failed mutations, launching
an interactive editor on a background worker, stale async results replacing a
newer dialog, and large selections hiding confirmation consequences. Regression
coverage checks these boundaries using isolated repositories and pure UI fixtures.

The user requested renaming the development branch and tracking origin/main,
then implementing the common lazygit panel gaps identified in chat.
The branch is now develop, tracking origin/main. Existing history is preserved.

## Smallest Slice

Implement in panel order, using typed operations and reducer-owned dialogs:
1. Files: conflict resolution and continue/abort/skip, stash options, status
   filtering, flat/tree and expand/collapse controls, upstream reset, skip-hook
   and external-editor commits.
2. Branches: merge/squash merge, rename, upstream, fast-forward, checkout by
   name/previous, copy name, remote branch browsing and tracking checkout.
3. Commits: cherry-pick selection/paste, revert, reset to selected commit,
   branch from commit, copy attributes, reorder, fixup/autosquash and tags.

Keep existing keys unless a contextual lazygit binding requires adjustment.
Make relevant operations available in main and branch commit lists.

## Non-Goals

No push, PR creation, promotion to main, PR hosting integration, bisect,
worktrees, or full lazygit parity. Conflict actions cover whole-file ours/theirs,
mark-resolved and the existing external editor; no conflict-hunk renderer.

## Expected Files

Core state/actions/workflows, GitBackend implementations, keyboard/desktop,
pure modal/panel rendering, harness/real-Git/UI tests, PRODUCT.md and DESIGN.md.

## Acceptance criteria

- develop tracks origin/main; existing history and user files are preserved.
- Keys and palette reach new workflows; menus capture the intended target.
- Cancellation has no Git effect; destructive actions require confirmation.
- Git uses validated refs/paths and argument arrays, with no shell interpolation.
- Conflict-producing failures refresh state and expose continue/abort/skip.
- Filters preserve complete repository data and coherent selection.
- Async ownership and exclusive mutation scheduling remain intact.

## Tests

Add reproductions for modal/palette C interception, dirty checkout of a file name,
autosquash requiring a message editor, editor precedence, partial staging,
configured conflict markers, large confirmation/menu sizes, and automatic stash
restoration with an existing stash. Run focused regressions, then the final gate.

Extend keyboard/reducer tests and reviewed deterministic snapshots. Harness
asserts UI, trace and resulting state. Real Git fixtures under tmp/git-tests
verify refs, index/worktree, conflicts, tracking, rewrites and stash boundaries.

## Harness Decision

New mutation flows require harness and real Git assertions. Add focused modules
instead of growing the monolithic scenario file; keep its manifest intact.

## Validation

Focused package checks during iteration. Final gate: cargo fmt --check;
cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace; git diff --check.

## Completion Evidence

Branch renamed to develop and upstream set to origin/main. All listed workflows
are implemented. Files gained independent staged/unstaged flags after real-Git
inspection exposed partial-staging ambiguity; status parsing, fallback, fixture
models, stage toggles and filter targets were updated together.

Focused validation: 14 isolated real Git workflow scenarios, five ownership,
confirmation and projection regressions, two input-routing tests, and 11 new
deterministic snapshots. Includes merge/rebase/cherry-pick conflicts, abort,
continue/skip, literal paths, stash boundaries, upstream/tracking, rename,
fast-forward divergence, commit reordering, fixup/autosquash and partial staging.
Existing global-palette default behavior was preserved after a harness regression.
Reviewed all 11 new snapshots and selected-row/cursor assertions. Final gate
passed: cargo fmt --check;
cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace (631 passed, zero failed/ignored);
git diff --check. Test log: tmp/workflows-workspace-test.log.
No commits or push requested. Validation ran on Windows; live external-editor
interaction and Linux execution were not manually exercised.

Review follow-up: reproduced and fixed modal/palette host-key leakage, ambiguous
checkout in the auto-stash path, autosquash message-editor invocation, and parsing
fallback editors before applying higher-precedence configuration. Also fixed
partial-staging summaries/mock commits, custom conflict-marker widths, overflowing
remote menu sizes, and batch confirmation paths hiding the consequence text.
Automatic stash previously compared moving stash selectors and lost index state;
it now compares object IDs and restores the exact stash with --index. A harness
regression proves successful and failed checkout preserve existing stashes,
partial staging and untracked files. Reviewed both changed/new confirmation
snapshots. Review fixes and a local commit are authorized; the prior no-commit
note records the earlier delivery.

Review gate passed on Windows: cargo fmt --check;
cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace (640 passed, zero failed/ignored);
git diff --check and git diff --cached --check. Full test log:
tmp/review-workspace-test.log. Nine additional regressions and extended existing
assertions cover the review findings. All 12 workflow snapshots were reviewed;
the review changed the single-file confirmation and added the batch confirmation.
Live external-editor interaction and Linux execution remain untested manually.
Delivery is a local commit on develop tracking origin/main; no push.
