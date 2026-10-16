# Active development plan

## Active Slice

Name: Resolve main integration conflicts and restore PR validation
Status: completed

## Problem

PR lpcv5/ratagit#10 merges develop into main. Main adopted the architecture
through a squash merge while develop retains its original history, causing merge
conflicts and preventing pull-request merge validation. The user requests conflict
resolution and CI fixes, including committing and pushing the repaired branch.

## Smallest Slice

Merge origin/main into develop without rewriting history. Resolve conflicts using
the current main tree and preserve newer develop workflow features, fixes, and
repository guidance. Run the required Windows validation, fix failures caused by
the integration, and push. Use the desktop PR status/monitor for CI state.

## Non-Goals

No PR merge, force push, unrelated product features, or history rewrite. Do not
change the separate main checkout. Persistent Auto-fix was denied by automatic
approval review, so do not retry that setting or bypass the denial.

## Expected Files

Merge conflicts may span application, core, Git, UI, tests, and documentation due
to the squash history. Prefer preserving identical current trees; inspect every
conflicted path and keep the final delta focused on the newer workflow changes.
Update CI or affected tests/code only when validation provides evidence.

## Tests

Run formatting, production Clippy, compile check, full workspace tests (including
snapshots and harness), and diff whitespace validation. Inspect real failures;
review snapshot changes individually if any arise. Existing workflow real-Git,
reducer, input, UI, and harness coverage should exercise preserved behavior.

## Harness Decision

Full workspace tests include the harness and real-Git scenarios. No new product
flow is planned; add a focused regression only for an uncovered integration bug.
Real-Git tests must remain isolated under workspace tmp/git-tests/.

## Validation

rtk is unavailable in this shell; use the underlying commands:
cargo fmt --check
cargo clippy --locked --workspace --lib --bins -- -D warnings
cargo check --locked --workspace
cargo test --locked --workspace
git diff --check

## Completion Evidence

Resolved all 44 conflicts after verifying origin/main and earlier develop commit
50e9f39 have identical tree fc83da325c1c7ed9c3d2dddba9846ac93df85870. Preserving
current develop therefore retains the exact main baseline plus all subsequent
workflow and guidance changes. No source, test, dependency, or CI changes were
needed for conflict resolution; no unresolved entries or conflict markers remain.

Passed on Windows: cargo fmt --check; production Clippy with -D warnings;
cargo check --locked --workspace; working-tree and staged diff whitespace checks.
Full workspace tests passed: 640 tests, zero failures or ignored tests, including
UI snapshots, harness, isolated tracing, and real-Git workflows. Full log:
tmp/main-integration-workspace-test.log. No snapshot files changed; earlier
workflow snapshot reviews are retained in Git history. Linux CI remains to be
checked after push; live external-editor interaction was not manually verified.
Automatic approval review denied persistent PR Auto-fix.
