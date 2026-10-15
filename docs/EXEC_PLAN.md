# EXEC_PLAN.md

## Active Slice

Name: Migrate PR #8 onto feature/newarch

Status: completed

Tracking: docs/PR8_MIGRATION.md

## Problem

PR #8 was developed on main while feature/newarch is the intended product.
Its useful features and fixes must use newarch's AppContext, GitBackend,
workspace boundaries, pure UI, and visual design.

## Smallest Slice

- Audit all 15 PR commits against newarch; retain existing architecture equivalents.
- Add reviewed partial patches behind GitBackend; test native bytes before UI work.
- Integrate block/hunk/line-range workflows into existing Details and confirmation UI.
- Add missing file, stash, fetch, and filtered command workflows.
- Port cache budgets, request/shutdown hardening, terminal/process regressions, and CI.
- Verify every migrated behavior and review snapshots against the newarch baseline.

Cross-layer changes are necessary for input -> reducer -> Command -> GitBackend
-> result -> immutable UI. Each implementation slice has focused tests before
the next slice.

## Non-Goals

- Do not merge the old src/app + Flux architecture wholesale.
- Do not replace newarch's layout, theme, editor or branch/commit rewrite workflows.
- Do not alter the original dirty checkout or its uncommitted branch-management work.
- Do not publish, push, or merge remote branches.

## Expected Files

Core patch/selection/cache/action modules; GitBackend patch and file services;
runtime ownership; application input/desktop/terminal; pure Details rendering;
core/backend/harness/process tests; snapshots; CI and product/design docs.

## Tests

- Lossless partial stage/unstage/discard, stale/wrong-direction rejection,
  untracked/literal paths, CRLF, EOF, binary/truncated/rename/deletion cases.
- Combined preview mappings and overlapping-edit fallback.
- Selection modes, ranges, independent side viewports, request ownership.
- Real Git UI flows for partial operations, stage-all, ignore, stash preview/apply/drop.
- Filtered/no-result palette; native editor argument boundaries; modal precedence.
- Cache byte budgets/recency; blocked-worker shutdown; terminal startup/isolation.
- Existing newarch tree, branch/rewrite, scroll, stale-read, and snapshot suites.

## Harness Decision

New real Git flows assert rendered UI and actual Git state. Their failures write
screen, AppContext, input sequence and Git state artifacts, with a focused test
for artifact generation. Existing mock harness scenarios retain coverage of
unchanged newarch behavior. Unix-only PTY coverage stays Unix-only.

## Validation

Standard commands: cargo fmt --check, cargo check --locked, cargo test --locked,
and cargo clippy --locked --all-targets --all-features -- -D warnings.
The explicit +stable invocations below select the verified toolchain.

```text
cargo +stable fmt --check
cargo +stable check --locked
cargo +stable test --locked
cargo +stable clippy --locked --all-targets --all-features -- -D warnings
```

Use verified Windows stable Cargo and the existing build cache. Ubuntu/Windows
CI now runs all four checks. Local WSL has Ubuntu 24.04 and C tooling but no Rust
toolchain; do not claim local Unix test execution.

## Completion Evidence

All four checks passed, including 567 workspace tests, zero failures/ignored.
Seven new snapshots and one intentional command-modal update were reviewed;
the baseline theme/layout/modal/editor files remain identical to newarch.
A Windows PTY smoke session exercised patch entry, line range selection,
command filtering, escape/back and normal quit with alternate-screen restoration.
The migration is committed locally and delivered by fast-forwarding feature/newarch;
the original dirty checkout and remote branches remain unchanged.
See PR8_MIGRATION.md for source-to-target decisions and practical limits.
