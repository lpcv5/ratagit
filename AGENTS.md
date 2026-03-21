# Repository Guidelines

## Project Structure & Module Organization

`ratagit` is a single-crate Rust TUI app.

- `src/main.rs`: entry point, terminal setup, event loop.
- `src/app/`: TEA core (`App`, `Message`, `update`, `Command`).
- `src/git/`: Git abstraction and implementation (`GitRepository`, `Git2Repository`).
- `src/ui/`: layout, panels, widgets, and rendering.
- `src/config/`: keymap and config loading.
- `docs/`: architecture, roadmap, and design decisions.
- `target/`: build artifacts (generated).

Keep new features inside the existing layer boundaries (UI -> app messages -> git trait).

## Build, Test, and Development Commands

- `cargo check`: fast compile checks during iteration.
- `cargo build`: debug build.
- `cargo run`: run the TUI in the current Git repository.
- `cargo test`: run all tests.
- `cargo test test_name`: run a single test by name.
- `cargo fmt`: format code with rustfmt.
- `cargo clippy --all-targets --all-features -D warnings`: lint and fail on warnings.
- `cargo build --release`: optimized production build.

## Coding Style & Naming Conventions

- Follow Rust defaults: 4-space indentation, `snake_case` for functions/modules, `PascalCase` for types/enums, `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused and small; prefer adding methods to the relevant layer module.
- Route all Git operations through `GitRepository`; do not call `git2` directly from UI/app code.
- All source-code comments and doc comments must be in English.

## Testing Guidelines

- Prefer unit tests near implementation (`mod tests` in the same file).
- Use integration tests in `tests/` for cross-module workflows when added.
- Use `tempfile` to create temporary repositories for Git behavior tests.
- Name tests by behavior, e.g. `test_stage_file_updates_status`.

## Commit & Pull Request Guidelines

- Follow Conventional Commit style seen in history: `feat: ...` (also use `fix:`, `refactor:`, `test:`, `docs:`).
- Keep commits focused and compilable.
- Use GitHub CLI `gh` for PR creation/review/merge operations.
- PRs should include:
  - What changed and why.
  - Linked issue/task (if any).
  - Verification steps (`cargo check`, `cargo test`, `cargo clippy`).
  - Screenshots or terminal captures for UI changes.

## Branch Workflow

- Default development branch is `dev`.
- Implement and commit changes on `dev` unless explicitly instructed otherwise.
- Default primary branch is `main`.
- Open and merge PRs from `dev` into `main`.
- Sync `main` into `dev` using rebase flow (`git checkout dev` -> `git fetch origin` -> `git rebase origin/main`).
- Prefer rebase over merge commits when integrating `main` into `dev`.

## Agent-Specific Notes

- Do not revert unrelated working-tree changes.
- Prefer minimal, incremental patches and keep architecture docs in `docs/` in sync with behavior changes.