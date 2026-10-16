# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

ratagit is a Rust 2024 Git TUI built with ratatui and crossterm, aiming for lazygit-style workflows. Read [AGENTS.md](AGENTS.md) for repository invariants and delivery rules; behavior is documented in [docs/PRODUCT.md](docs/PRODUCT.md).

## Commands

Run from the workspace root with stable Rust and Git installed. Commands use the repository's `rtk` prefix; if the wrapper is unavailable, run the underlying command and report that limitation.

```bash
# Build / compile check
rtk cargo build --locked --workspace
rtk cargo check --locked --workspace

# Launch the TUI in an interactive terminal (explicit binary avoids helper binaries)
rtk cargo run --locked --bin ratagit

# Format / check formatting
rtk cargo fmt
rtk cargo fmt --check

# Strict lint: production libraries and binaries; test-only lint is advisory
rtk cargo clippy --locked --workspace --lib --bins -- -D warnings

# Full test gate: unit, integration, UI snapshots, and harness
rtk cargo test --locked --workspace

# One crate / one integration test target / one exact test
rtk cargo test --locked -p ratagit-ui
rtk cargo test --locked -p ratagit-ui --test details_entry
rtk cargo test --locked -p ratagit-ui --test details_entry entry_keeps_expanded_files_and_previous_preview_until_patch_arrives -- --exact

# Harness only / one mock scenario
rtk cargo test --locked -p ratagit-harness
rtk cargo test --locked -p ratagit-harness --test harness harness_status_refresh -- --exact

# Review pending snapshot differences (requires cargo-insta)
rtk cargo insta review

# Whitespace validation
rtk git diff --check
```

The app discovers a repository from the process working directory and uses `HybridGitBackend`; outside a repository it displays a mock dirty-repository fixture. The executable owns keyboard mapping, terminal lifecycle, clipboard, and external editors.

Ensure a filtered test command actually runs the intended tests. CI runs formatting, strict production Clippy, compile checks, and the full workspace tests on Ubuntu and Windows; a full test run already includes snapshots and harness tests.

## Development workflow

Follow [docs/DEVELOPMENT_LOOP.md](docs/DEVELOPMENT_LOOP.md): small documentation or bounded local changes need only a goal and validation note in the conversation. Before substantial, ambiguous, cross-layer, Git mutation, or async/runtime work, update [docs/EXEC_PLAN.md](docs/EXEC_PLAN.md) with scope, risks, acceptance criteria, and verification, preserving its required headings.

For code, dependency, build, or CI changes, the final gate is formatting, strict production Clippy, and full workspace tests above. Review intended snapshot diffs rather than bulk-accepting mismatches. Documentation-only changes need content/link review and `rtk git diff --check`.

## Architecture

The executable composes six workspace libraries under `libs/`. The dependency boundaries and state ownership are specified in [ARCHITECTURE.md](ARCHITECTURE.md); rendering and interaction design are in [docs/DESIGN.md](docs/DESIGN.md).

- **ratagit-core** owns the pure state machine and shared Git models, with no external dependencies. `AppContext` separates Git-derived `repo` data, `ui` state/projections, and pending/loading `work` state. Inputs become `UiAction`; `update(&mut state, Action)` emits `Command` values. Backend `GitResult` values re-enter the same reducer and can emit follow-up commands such as refreshes after mutations.
- **ratagit-git** depends on core and exposes `GitBackend`, composed of read, write, and history-rewrite traits. `HybridGitBackend` combines libgit2 operations with a private `GitCli` helper; `MockGitBackend` and `SharedMockGitBackend` support tests/demo state. Git access stays behind this boundary.
- **ratagit-harness** depends on core, git, and ui and supplies both the synchronous `Runtime` for deterministic scenarios and the production `AsyncRuntime`. The latter uses standard threads/channels, a read worker pool, and a serial mutation worker. A shared scheduler coalesces refreshes and debounces Details requests. Mutations advance a generation so stale reads are dropped; new reads defer until mutations finish. Already-running reads may overlap writes. Details results must also match their request id, target, and current selection.
- **ratagit-ui** depends on core and projects immutable `&AppContext` into ratatui output, without state mutation or Git access. Files, Branches, Commits, and Stash occupy the left column; Details and Log occupy the right. The focused left panel borrows height from others, and Stash collapses when unfocused. Files previews and reviewed native patches share a diff-row renderer; history/log text uses ANSI-to-span projection. Core owns tree projections and bounded detail text caches; rendering contracts belong in DESIGN.md.
- **ratagit-observe** configures file-based tracing. Use `RATAGIT_LOG=debug` or `RATAGIT_LOG_PATH` when diagnosing runtime behavior; see [docs/OBSERVABILITY.md](docs/OBSERVABILITY.md).
- **ratagit-testkit** depends on core and supplies shared deterministic fixtures for tests and the application's demo state.

## Testing contracts

See [docs/TESTING.md](docs/TESTING.md) for coverage selection and failure artifacts.

- UI tests render the real terminal renderer with `ratatui::TestBackend` and deterministic fixtures. Insta snapshots live under `libs/ratagit-ui/tests/snapshots/`; assert selection/cursor styles separately because text snapshots do not encode colors.
- Interaction flows and Git mutations need harness coverage of rendered UI, operation trace, and resulting Git state. The mock scenario index [docs/harness/SCENARIOS.md](docs/harness/SCENARIOS.md) must track `fn harness_*` definitions in `libs/ratagit-harness/tests/harness.rs`; real-Git smoke tests are separate.
- Real-Git tests use isolated repositories under workspace `tmp/git-tests/`, never the checkout itself. Scheduler tests use injected time; async tests use explicit wait helpers or channel barriers instead of bare sleeps.
- Large synthetic repositories and performance suites are manual tools, outside normal tests/CI. Commands and report locations are in [docs/TOOLING.md](docs/TOOLING.md).
