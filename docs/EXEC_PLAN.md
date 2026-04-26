# EXEC_PLAN.md

## Goal

Restructure the project into a standard Rust application workspace:

- make the repository root the runnable `ratagit` application package
- move internal libraries from `crates/` to `libs/`
- centralize shared package metadata and dependency declarations in the root
  `Cargo.toml`
- preserve independent library packages and their per-package tests

## Vertical Slice

1. Workspace layout
- move the app entrypoint to `src/main.rs`
- move `ratagit-core`, `ratagit-ui`, `ratagit-git`, `ratagit-observe`,
  `ratagit-testkit`, and `ratagit-harness` under `libs/`
- remove tracked harness target artifacts from source control

2. Cargo manifests
- make the root `Cargo.toml` both the app manifest and workspace manifest
- define workspace `default-members` so `cargo test` and clippy still cover the
  root app plus all libraries
- use workspace inheritance for version, edition, license, external dependency
  versions, and internal path dependencies
- keep each library `Cargo.toml` minimal while preserving crate boundaries

3. Documentation
- update `ARCHITECTURE.md` to describe the root app and `libs/` package layout
- do not update `PRODUCT.md` or `DESIGN.md` because this change has no product
  behavior or UI design impact

4. Tests
- run package-specific tests for core, UI snapshots, and harness scenarios
- run the full workspace quality gates

5. Quality gates
- run `cargo fmt`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
- run `cargo test snapshots -- --nocapture`
- run `cargo test harness -- --nocapture`
