# ratagit

Rust 2024 Git TUI built with ratatui and crossterm, aiming for lazygit-style
workflows. Follow [AGENTS.md](AGENTS.md) for repository rules and validation.

## Commands

```sh
cargo run --locked --bin ratagit
cargo check --locked --workspace
cargo fmt --check
cargo clippy --locked --workspace --lib --bins -- -D warnings
cargo test --locked --workspace
```

Run `cargo fmt` to fix formatting. During iteration, target the affected package
or integration test, for example:

```sh
cargo test --locked -p ratagit-ui --test details_entry
cargo test --locked -p ratagit-harness --test harness harness_status_refresh -- --exact
```

Verify filtered commands actually run the intended tests. Full workspace tests
already include snapshots and harness tests. Review intended snapshot changes;
use `cargo insta review` if cargo-insta is installed.

The app discovers a repository from the working directory and uses
`HybridGitBackend`; outside a repository it displays a mock fixture. Launch the
TUI in an interactive terminal. For runtime diagnostics, see
[observability](docs/OBSERVABILITY.md).

## Documentation

Read relevant sections on demand:

- [Architecture and ownership](ARCHITECTURE.md)
- [Product behavior](docs/PRODUCT.md) and [rendering design](docs/DESIGN.md)
- [Development workflow](docs/DEVELOPMENT_LOOP.md) and [testing](docs/TESTING.md)
- [Manual tooling and performance](docs/TOOLING.md)
