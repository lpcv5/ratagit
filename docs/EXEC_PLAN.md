# EXEC_PLAN.md

## Active Slice

Name: Isolate backend tracing regression before main promotion

Status: completed

## Problem

The user designated feature/newarch as authoritative for main. Integration
preparation passed 606 local tests, but PR CI exposed a flaky log-capture test:
one Ubuntu run captured no command-start event while another passed on the same
head. The test shares process-wide tracing callsites with parallel unit tests.

## Smallest Slice

Move the existing log-capture regression into its own integration-test binary,
retain every logging/privacy assertion, and keep its real repository under
workspace tmp/ with cleanup on success or panic. Revalidate and deliver PR #9.

## Non-Goals

Change production logging, weaken assertions, serialize unrelated tests, restore
retired main files, rewrite history, or alter another worktree's checkout.

## Expected Files

docs/EXEC_PLAN.md, libs/ratagit-git/src/hybrid/tests.rs, and a dedicated backend
tracing integration test. No production-layer changes.

## Tests

Run the isolated logging test, the parallel backend unit suite, formatting,
locked compilation, strict Clippy, and locked workspace tests. Require Ubuntu
and Windows PR CI to pass before merging the validated head.

## Harness Decision

No new scenario: only test-process isolation changes. Existing assertions still
check command metadata, timings, status parsing, and absence of Git payloads.

## Validation

Passed: cargo fmt; cargo fmt --check; cargo check --locked;
cargo clippy --workspace --lib --bins -- -D warnings;
cargo clippy --locked --all-targets --all-features -- -D warnings;
cargo test --locked; git diff --check; dedicated tracing and backend unit tests.
PR CI and final delivery are tracked in the pull request.

## Completion Evidence

The dedicated tracing test and all 52 parallel backend unit tests passed.
Formatting, locked compilation, both strict Clippy checks, and all 606 workspace
tests passed with no failures or ignored tests. Log:
tmp/newarch-isolated-tracing-tests.log. All seven logging/privacy assertions
remain; the fixture uses workspace tmp/ with RAII cleanup. Integration merge
a5248df records old main as an ancestor while preserving the feature tree.
PR #9 tracks Ubuntu/Windows CI and final integration.
