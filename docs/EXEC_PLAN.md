# EXEC_PLAN.md

## Current Slice

Continue low-risk duplication cleanup and reducer simplification.

## Goal

- keep behavior unchanged
- stay std-only and avoid new dependencies
- extract repeated code only where reuse is already visible or likely in near-term features
- keep rendering pure and AppState as the only source of truth
- preserve existing snapshots and harness behavior
- reduce `ratagit-core` reducer size without changing architecture boundaries

## Vertical Slice

1. Command metadata helpers
- move command debounce keys, mutation classification, and pending-operation labels onto `Command`
- keep `debounce_key_for_command` as a compatibility wrapper
- update runtime coalescing to use command metadata methods

2. Core reducer modules
- move mutating operation result handling into a private operations module
- move details refresh, result application, cache, and scroll helpers into a private details module
- keep all state in `AppState` and all side effects represented as `Command`

3. UI choice metadata
- generate branch delete/rebase modal choice rows from existing enum option arrays
- keep reset choice rendering behavior unchanged

4. Test fixture cleanup
- extract only repeated, domain-named test fixture helpers
- keep important test setup visible at each assertion site

5. Validation
- run focused package tests after each slice where useful
- run `cargo fmt --check`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
