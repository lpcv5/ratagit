# EXEC_PLAN.md

## Active Slice

Name: Restore PR #8 Details diff and verify real keyboard operations

Status: completed

Tracking: docs/PR8_DETAILS_VALIDATION.md, extending docs/PR8_MIGRATION.md

## Problem

The initial migration simplified PR #8's Details diff and forced visual
selection into line mode. The user makes PR #8 authoritative for the right
Details diff and requires actual Files -> Enter -> Details testing for single
hunks, independent multiple hunks and visual hunk ranges.

## Smallest Slice

- Restore Hunk/Line modes, visual ranges, independent marks and clear/back.
- Port dual line numbers, cursor/mark gutters, staged diff colors and status.
- Port reviewed-byte cursor relocation after hunk writes and whole-block scrolling.
- Correct viewport height to match the actual ratatui layout.
- Add the real keyboard/Git matrix and independently exercise the compiled TUI.

Cross-layer changes are required for input -> pure reducer -> Command ->
GitBackend -> immutable UI. AppContext remains the sole source of truth.

## Non-Goals

- Do not transplant old Flux stores or replace the surrounding newarch panels.
- Perform test writes only in isolated fixtures, never in the user's repository.
- Do not push or merge remote branches, or claim local Unix PTY coverage.

## Expected Files

Core partial state/selection/position/viewport/row modules and actions; application
input; pure Details projection/rendering and viewport metadata; core, input,
snapshot and real Git keyboard tests; terminal verification runner; product,
design, architecture and tracking documentation.

## Tests

- Visual ranges preserve mode, freeze on second v and coexist with independent marks.
- Escape clears marks before leaving; mode/side/document changes clear stale marks.
- Line navigation includes context while writes apply changed source lines only.
- Applied cursors follow reviewed bytes to the destination side; unmarked cursors
  survive removal of earlier hunks. Clamped movement preserves manual scrolling.
- Complete replacement blocks fit before context padding; terminal heights 14–39
  show both removed and added lines.
- Snapshots cover marks, hunk/line ranges, staged colors, loading and narrow panes.
- Three selection strategies x stage/unstage/discard assert exact index/worktree
  bytes and an unrelated staged sentinel; an extra case covers staged-side d.

## Harness Decision

The production key mapper drives Runtime + HybridGitBackend with isolated real
Git repositories. Scenarios assert rendered UI and complete Git contents, saving
screen, AppContext, key sequence and Git state artifacts, including on failure.
Separate Windows ConPTY runs exercise the compiled binary through Files/Enter.

## Validation

All four CI commands passed: cargo fmt --check, cargo check --locked,
cargo test --locked, cargo clippy --locked --all-targets --all-features -- -D warnings.
Each was invoked with +stable to select the verified Windows toolchain.
Full tests: tmp/pr8-details-final-tests.log. No local Unix execution is claimed.

## Completion Evidence

589 workspace tests passed, zero failures/ignored; strict Clippy, compilation,
formatting and git diff --check passed. Ten real Git keyboard scenarios include
the complete nine-cell matrix plus staged-side d. Two independent nine-case
Windows ConPTY runs passed, including a rerun after cursor/viewport fixes.
Six new and six intentionally updated snapshots were reviewed.
PR8_DETAILS_VALIDATION.md records keys, assertions, artifacts and portability
decisions. The surrounding newarch UI and GitBackend boundaries remain in use.
