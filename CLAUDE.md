# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run                  # run in the current Git repo
cargo check                # fast type-check without building
cargo test                 # run all tests
cargo test <name>          # run a single test by name
cargo fmt --check          # verify formatting (CI enforced)
cargo clippy --all-targets --all-features -- -D warnings
```

Full local gate before a PR:
```bash
cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features -- -D warnings
```

## Architecture

Ratagit is a single-crate TUI Git client (ratatui + crossterm + git2 + tokio). All Git I/O is separated from the UI via bounded async channels (capacity 100).

```
main.rs
  ├── tokio::spawn(run_backend(cmd_rx, event_tx))   ← Git I/O on a background task
  └── App::new(cmd_tx, event_rx).run().await        ← UI on the main thread
```

**Channel protocol** — `CommandEnvelope` (UI → Backend) and `EventEnvelope` (Backend → UI), both carrying a `request_id: u64` for request tracking.

| Direction | Type | Key variants |
|-----------|------|-------------|
| UI → Backend | `BackendCommand` | `RefreshStatus`, `RefreshBranches`, `RefreshCommits`, `RefreshStashes`, `GetDiff`, `GetDiffBatch`, `GetCommitFiles`, `GetCommitDiff`, `GetBranchCommits`, `StageFile/s`, `UnstageFile/s`, `Quit` |
| Backend → UI | `FrontendEvent` | `FilesUpdated`, `BranchesUpdated`, `CommitsUpdated`, `StashesUpdated`, `DiffLoaded`, `CommitFilesLoaded`, `BranchGraphLoaded`, `BranchCommitsLoaded`, `ActionSucceeded`, `Error` |

**`App` (src/app/runtime.rs)** — main loop: drain backend events → draw → poll input (100 ms). Owns `AppState` and `RequestTracker`.

**`AppState` (src/app/state.rs)** — holds `UiState` (panel focus, scroll), `CachedData` (files, branches, commits, stashes, diffs), channel handles, and the log buffer.

**`RequestTracker` (src/app/request_tracker.rs)** — tracks in-flight request IDs; stale/duplicate responses are dropped in `handle_backend_event`.

**`run_backend` (src/backend/runtime.rs)** — opens `GitRepo::discover()` once, dispatches each `CommandEnvelope` to a `CommandHandler` impl (one per command type in `src/backend/handlers.rs`), sends `EventEnvelope` responses.

**`GitRepo` (src/backend/git_ops/repo.rs)** — wraps `git2::Repository`. Operations are split into focused modules: `status`, `branches`, `commits`, `stash`, `diff`, `commit_files`, `commit_diff`, `branch_graph`, `working_tree`.

**Component system (src/components/)** — UI panels implement the `Component` trait (`handle_event` → `Intent`, `render`). Components return `Intent` values; `App` executes them via `intent_executor.rs`. Core primitives live in `src/components/core/` (tree, selectable_list, simple_list, multi_select, theme).

**Layout** — two columns (34% / 66%). Left: Files, Branches, Commits, Stash panels. Right: Main View (diff/detail) + Log panel.

## Key conventions

- `Intent` (src/app/intent.rs) is the only way components communicate upward to `App` — no direct state mutation from components.
- `push_log` appends to the in-memory log (capped at 200 entries); use it for any backend response or user action worth surfacing.
- `sync_*_list_state` keeps `ListState` in bounds after data refreshes; always call after replacing a panel's data vec.
- Commit messages use imperative prefixes: `feat:`, `refactor:`, `chore:`, `fix:`.
- PRs for TUI-visible changes should include terminal captures.
