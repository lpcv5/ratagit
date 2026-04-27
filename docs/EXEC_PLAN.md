# EXEC_PLAN.md

## Current Slice

Rust abstraction reuse and module slimming.

## Goal

- keep behavior unchanged
- stay std-only and avoid new dependencies
- use Rust abstractions where they remove visible repetition
- keep rendering pure and AppState as the only source of truth
- preserve existing snapshots and harness behavior
- reduce oversized production files without changing architecture boundaries
- keep public APIs stable through facade modules and re-exports

## Vertical Slice

1. Backend abstraction cleanup
- add a boxed `GitBackend` blanket implementation
- return `Box<dyn GitBackend + Send>` from backend selection
- remove the root app backend enum and handwritten trait forwarding

2. Root app module slimming
- move key mapping, `KeyEffect`, and input mapping tests into `src/input.rs`
- keep `src/main.rs` focused on terminal setup, event loop, and backend selection

3. Core module slimming
- move action/result/command types and command metadata into a core action module
- move editor reducer helpers into a core editor module
- move commit rewrite and commit-files workflow into a focused commit workflow module
- add shared const-generic choice navigation for bounded enum choices
- keep all state in `AppState` and all side effects represented as `Command`

4. UI panels module slimming
- split panel line types, left-panel projections, details/log projections, scroll helpers, and formatters
- keep `panels.rs` as the facade for existing crate-internal call sites
- preserve deterministic rendering and existing snapshots

5. Validation
- run focused tests after each slice where useful
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
- report before/after Rust line counts and remaining oversized files
