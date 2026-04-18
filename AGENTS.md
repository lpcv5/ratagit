# Repository Guidelines

## Project Structure & Module Organization
- Core Rust code lives in `src/`.
- Entry point: `src/main.rs` (boots frontend and backend runtime).
- UI orchestration and rendering: `src/app.rs`.
- Async command/event backend loop: `src/backend.rs`.
- Git data access layer: `src/git/` (`repo.rs`, `mod.rs`).
- Shared UI component definitions: `src/components/`.
- CI configuration is in `.github/workflows/ci.yml`.
- Build artifacts are generated under `target/` and must not be edited manually.

## Build, Test, and Development Commands
- `cargo run` - run Ratagit in the current Git repository.
- `cargo build` - compile a debug build.
- `cargo check` - fast compile/type validation without producing binaries.
- `cargo test` - run unit/integration tests.
- `cargo fmt --check` - verify formatting (CI enforced).
- `cargo clippy --all-targets --all-features -- -D warnings` - lint and treat warnings as errors.

Run the full local gate before opening a PR:
`cargo fmt --check && cargo check && cargo test && cargo clippy --all-targets --all-features -- -D warnings`

## Coding Style & Naming Conventions
- Follow `rustfmt` defaults (4-space indentation, standard Rust layout).
- Use `snake_case` for functions/modules/files, `PascalCase` for types/enums, `SCREAMING_SNAKE_CASE` for constants.
- Keep modules focused by responsibility (UI in `app`, Git I/O in `git`, runtime wiring in `backend`).
- Prefer small, pure helpers for state transformations; keep side effects in backend/runtime boundaries.

## Testing Guidelines
- Put narrow unit tests next to code using `#[cfg(test)]`.
- Put cross-module behavior tests in `tests/` when introduced.
- Test names should describe behavior, e.g. `loads_diff_for_selected_file`.
- For backend changes, include success and error-path coverage.

## Commit & Pull Request Guidelines
- Commit messages in history favor concise, imperative summaries, often with prefixes like `feat:`, `refactor:`, `chore:`.
- Keep commits scoped to one logical change.
- PRs should include:
  - clear problem/solution summary,
  - validation steps and command output summary,
  - linked issue (if applicable),
  - screenshots or terminal captures for TUI-visible changes.
- Ensure CI passes on `dev`/`main` targets before requesting review.


## Reference
This project aim to rewrite lazygit to rust version, you can check how lazygit process the git and ui/ux from the local downloaded repo here D:\prj\lazygit