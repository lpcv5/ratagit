# EXEC_PLAN.md

## Active Slice

Name: Remove obsolete and redundant documentation

Status: completed

## Problem

Completed migration records, a finished refactoring checklist, an empty archive
guide, and duplicate policy documents obscure the current project contract.
Some retained guides still describe behavior that has already been replaced.

## Smallest Slice

Delete completed and redundant documents, repair references, and correct stale
descriptions in the retained architecture, design, product, and agent guides.
Keep the active plan and test-maintained harness manifest.

## Non-Goals

Change application code, tests, runtime behavior, Git semantics, or tooling.

## Expected Files

Documentation only: root guides and docs/. No production-layer changes.

## Tests

Check deleted-document references, local Markdown links, and diff whitespace.
Run existing workspace checks, including active-plan and harness-manifest tests.
No new unit or snapshot tests are needed for documentation-only cleanup.

## Harness Decision

No new scenario: rendered behavior, input handling, and Git state are unchanged.
Retain the scenario index and terminal verification runner.

## Validation

Passed: cargo fmt; cargo clippy --workspace --lib --bins -- -D warnings;
cargo test (including snapshots, harness scenarios, and real Git tests);
git diff --check; deleted-document reference and local Markdown link checks.

## Completion Evidence

Removed nine completed, obsolete, or redundant documents. Retained the active
specifications, agent instructions, operational guides, and checked harness
manifest. Updated stale rendering/stash/reset descriptions and kept manual
terminal verification instructions in TESTING.md. All changes are Markdown only;
the standard formatting, strict production Clippy, and workspace tests passed.
