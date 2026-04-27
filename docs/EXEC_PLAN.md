# EXEC_PLAN.md

## Current Slice

Details diff row semantics.

## Goal

- color every patch row with deterministic Details semantics
- keep the smallest useful step toward future hunk-level staging without changing
  the Git backend or introducing hidden UI state
- keep rendering pure and AppState as the only source of truth
- keep UI assertions deterministic through fixtures, unit tests, snapshots, and harness scenarios

## Vertical Slice

1. Details row semantics
- classify extended patch metadata such as file modes, rename/copy headers,
  similarity headers, binary patch headers, and no-newline markers as diff
  metadata
- keep hunk headers, additions, removals, and section headers unchanged

2. UI snapshots and docs
- add style-sensitive terminal buffer assertions for extended diff metadata
- document the semantic row set and note that future hunk staging should build
  on an AppState-owned structured diff model

3. Harness
- keep existing Details scenarios asserting UI text and Git state; no backend
  command shape changes are required for this color-only slice

4. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
