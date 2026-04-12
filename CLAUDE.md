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

Ratagit is a single-crate TUI Git client (ratatui + crossterm + git2 + tokio). The design separates all Git I/O from the UI via async channels.

```
main.rs
  ├── tokio::spawn(run_backend(cmd_rx, event_tx))   ← Git I/O on a background task
  └── App::new(cmd_tx, event_rx).run().await        ← UI on the main thread
```

**Channel protocol** — unbounded mpsc, two directions:

| Direction | Type | Variants |
|-----------|------|----------|
| UI → Backend | `BackendCommand` | `RefreshStatus`, `RefreshBranches`, `RefreshCommits { limit }`, `RefreshStashes`, `GetDiff { file_path }`, `Quit` |
| Backend → UI | `FrontendEvent` | `StatusUpdated`, `BranchesUpdated`, `CommitsUpdated`, `StashesUpdated`, `DiffLoaded`, `Error` |

**`App` (src/app.rs)** owns all UI state: panel focus (`Panel` enum), four `ListState`s, cached Git data (`files`, `branches`, `commits`, `stashes`), and the main-view/log scroll positions. The render loop is: drain backend events → draw → poll input (100 ms timeout).

**`run_backend` (src/backend.rs)** opens `GitRepo::discover()` once, then loops on `cmd_rx.recv()`, dispatching to `GitRepo` methods and sending results back as `FrontendEvent`s.

**`GitRepo` (src/git/repo.rs)** wraps `git2::Repository`. All Git operations live here: `get_status_files`, `get_branches`, `get_commits`, `get_stashes`, `get_diff`. Note: `get_stashes` requires `&mut self` due to the git2 callback API.

**Layout** — two columns (34% / 66%). Left column: Files, Branches, Commits, Stash panels stacked vertically. Right column: Main View (diff/detail/overview) + Log panel.

**Panel navigation** — `Tab`/`Shift+Tab`/`h`/`l` cycle panels; `j`/`k` navigate within a panel; selecting a list item triggers `update_main_view_for_active_panel`, which either sends a `GetDiff` command (Files panel) or renders detail text inline (other panels).

## Key conventions

- Side effects belong in `backend.rs`; `App` methods stay pure where possible.
- `push_log` appends to the in-memory log (capped at 200 entries) — use it for any backend response or user action worth surfacing.
- `sync_list_state` keeps `ListState` in bounds after data refreshes; always call it when replacing a panel's data vec.
- Commit messages use imperative prefixes: `feat:`, `refactor:`, `chore:`, `fix:`.
- PRs for TUI-visible changes should include terminal captures.
