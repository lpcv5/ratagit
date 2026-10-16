# Active development plan

## Active Slice

Name: Resolve main integration conflicts and restore PR validation
Status: in progress

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

Pending integration and validation. The earlier workflow slice recorded 640
passing Windows tests, reviewed workflow snapshots, formatting, and production
Clippy before this merge; those results are historical and do not validate the
merged tree. Linux execution and live external-editor interaction were not
manually verified. Automatic approval review denied persistent PR Auto-fix.
