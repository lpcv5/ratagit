# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
# Build
rtk cargo build
rtk cargo check

# Lint (production targets only — test code exempt)
rtk cargo clippy --workspace --lib --bins -- -D warnings

# Format
cargo fmt

# Test
rtk cargo test
rtk cargo test -p ratagit-ui   # single crate

# Review snapshot mismatches
cargo insta review
```

All commands should be prefixed with `rtk` (see global CLAUDE.md).

## Development Loop

Before any code edit, update `docs/EXEC_PLAN.md` with: problem, smallest slice, non-goals, expected files, tests, harness decision, and validation commands. See `docs/DEVELOPMENT_LOOP.md` for sizing rules (tiny / lightweight / full plan).

Exit conditions — do not commit if any of these fail:
- `cargo test`
- snapshot tests (`cargo insta review`)
- harness (`cargo test -p ratagit-harness`)

## Architecture

Layered workspace — each layer only depends on layers below it:

```
ratagit (binary)
├── ratagit-ui       — ratatui rendering, panels, modals, layout
├── ratagit-harness  — async runtime, background Git worker pool
├── ratagit-core     — pure state machine (no I/O, no git2)
├── ratagit-git      — GitBackend trait + CLI/hybrid/mock impls
├── ratagit-observe  — tracing/logging setup
└── ratagit-testkit  — shared test helpers
```

### State model (`ratagit-core`)

`AppContext` is the single pure state container:
- `repo` — Git-derived data (status, commits, branches, files, details, stash)
- `ui` — focus, search, selections, scroll, modals, projection caches
- `work` — pending command / loading state

Interaction loop: input → `UiAction` → `update()` → emits `Command` → dispatched to `GitBackend` → result updates `AppContext`.

### UI (`ratagit-ui`)

6 panels: left column (Files, Branches, Commits, Stash) + right column (Details, Log) + bottom shortcut bar. Dynamic left-panel heights: Stash collapses when unfocused; focused panel borrows height from others.

Details diff/log stored as raw backend output, rendered via ANSI-to-span projection (no re-parsing). Bounded caches for file-detail and commit-detail diffs.

### Git backend (`ratagit-git`)

`GitBackend` trait with three impls: `CliGitBackend`, `HybridGitBackend`, `MockGitBackend`. Read-only commands run on a background worker pool; mutations run on a dedicated serial worker.

### Testing

- Unit tests live alongside source or in `tests/` per crate.
- Snapshot tests (`insta`) live in `libs/ratagit-ui/tests/snapshots/`.
- Harness scenarios live in `libs/ratagit-harness/tests/` and `docs/harness/SCENARIOS.md`.
- `ratagit-testkit` provides shared fixtures.
