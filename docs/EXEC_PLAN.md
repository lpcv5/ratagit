# EXEC_PLAN.md

## Current Slice

Low-risk duplication cleanup and simplification.

## Goal

- keep behavior unchanged
- stay std-only and avoid new dependencies
- extract repeated code only where reuse is already visible or likely in near-term features
- keep rendering pure and AppState as the only source of truth
- preserve existing snapshots and harness behavior

## Vertical Slice

1. Core helpers
- extract shared Unicode text editing helpers for editor and branch-name input
- simplify shared file-tree projection code for Files and Commit Files
- move generic search reducer helpers out of the large core reducer
- reduce details-cache boilerplate with small bounded-cache helpers

2. UI and runtime helpers
- move reusable modal choice-list rendering into the modal system
- consolidate Git CLI command execution plumbing
- simplify harness command coalescing around shared command identity helpers

3. Validation
- run focused package tests after each slice where useful
- run `cargo fmt --check`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
