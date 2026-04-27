# EXEC_PLAN.md

## Current Slice

Git worker pool runtime.

## Goal

- keep rendering pure and AppState as the only source of truth
- keep side effects represented as `Command` plus `GitBackend`
- run real TUI read-only Git work across a fixed worker pool
- keep mutating Git work serialized through one exclusive worker
- prevent stale read results from applying after a queued mutation
- stay std-only and avoid new dependencies
- preserve synchronous harness determinism and existing UI snapshots

## Vertical Slice

1. Runtime API
- change `AsyncRuntime` construction to accept a backend factory
- create one backend per read worker plus one backend for the write worker
- keep the default read worker count fixed at 4

2. Worker pool dispatch
- classify commands with `Command::is_mutating()`
- route read commands to read workers by round-robin
- route mutating commands to the exclusive write worker
- keep debounce and command coalescing on the runtime thread before dispatch

3. Mutation barrier
- increment a runtime repository generation as soon as a mutation is queued
- defer new read commands while any mutation is in flight
- tag read results with their generation and ignore stale read results
- flush deferred reads after mutation results are reduced

4. Backend selection
- make the real TUI build workers from a backend factory
- open a fresh `HybridGitBackend` per real worker
- use a shared mock backend wrapper for non-repo demo mode

5. Validation
- add async runtime tests for read worker distribution, write serialization,
  deferred reads, and stale read result dropping
- update existing async runtime tests for factory construction
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
