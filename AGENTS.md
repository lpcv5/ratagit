# Repository Guidelines

## Project Structure & Module Organization

Ratagit is a Rust 2021 Git terminal UI using `ratatui`, `crossterm`, and Tokio. `src/main.rs` is the entry point. Application state and runtime helpers live in `src/app/`; actions, dispatching, reducers, and effects live in `src/flux/`. `src/git/repository.rs` defines the Git backend abstraction. Terminal components, panels, widgets, and themes live in `src/ui/`; keymap configuration lives in `src/config/`.

Unit tests sit alongside implementation code, including dedicated `update_tests.rs` and `highlight_tests.rs` modules. Binary process tests live in `tests/terminal_lifecycle.rs`; `tests/support/` supplies isolated subprocess fixtures shared with `src/process_tests.rs`. CI is defined in `.github/workflows/ci.yml` and runs on Ubuntu and Windows.

## Build, Test, and Development Commands

Use the stable Rust toolchain and Cargo:

- `cargo build`: build the development binary.
- `cargo run`: launch the TUI from a Git repository in an interactive terminal.
- `cargo build --release`: build the optimized release binary.
- `cargo fmt`: apply standard Rust formatting.
- `cargo fmt --check`: verify formatting without modifying files.
- `cargo check`: check compilation.
- `cargo test`: run all tests; use `cargo test test_stage_unstage_roundtrip` to filter.
- `cargo clippy --all-targets --all-features -- -D warnings`: lint with warnings treated as errors.

Run the four CI checks (format, check, test, Clippy) before submitting.

## Coding Style & Architecture

Follow rustfmt defaults, including four-space indentation. Use `snake_case` for modules and functions, `PascalCase` for types and traits, and `SCREAMING_SNAKE_CASE` for constants. Write comments and documentation in English; use `thiserror` for custom errors.

Preserve Flux flow: actions pass through the dispatcher and stores; reducers remain pure. Access state through `StateAccess`, perform Git I/O through effects and `GitRepository`, and render immutable snapshots. Avoid direct `git2` calls in stores or UI components.

## Testing Guidelines

Use Rust's built-in `#[test]` framework within `#[cfg(test)]` modules. Name tests `test_<behavior>` and use `tempfile` repositories for Git operations. Exercise input and actions through existing test dispatch/runtime helpers. Add regression tests for changed behavior. Existing guidance describes milestone coverage goals, but CI currently enforces no numeric coverage threshold.

Use `ProcessFixture` for process tests so each child has its own Git config, home/config directories, temporary files, and bounded lifetime. Never mutate the parent test process's environment or libgit2 search paths in parallel tests. Configuration scenarios inject a file path because Windows Known Folders do not follow an `APPDATA` environment override. Keep Unix PTY tests Unix-only; they do not verify Windows console cleanup. Golden regeneration remains a reviewed local action.

## Commit & Pull Request Guidelines

Follow the observed commit pattern: `refactor: ...`, `perf: ...`, `docs: ...`, or `chore: ...`, with concise imperative summaries. Keep changes focused. PRs should explain the problem, resulting behavior, reproduction steps, and verification; link relevant issues and include terminal screenshots for visible UI changes. CI runs for PRs targeting `dev` and `main`.
