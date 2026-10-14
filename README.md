# Ratagit

[![CI](https://github.com/lpcv5/ratagit/actions/workflows/ci.yml/badge.svg)](https://github.com/lpcv5/ratagit/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Stars](https://img.shields.io/github/stars/lpcv5/ratagit?style=social)](https://github.com/lpcv5/ratagit/stargazers)

Ratagit is a fast, keyboard-first Git terminal UI built with Rust and `ratatui`.
It is designed for developers who want a responsive Git workflow without leaving the terminal.

## Why Ratagit

- Fast navigation across Git status, diffs, branches, commits, and stashes
- Predictable unidirectional data flow (Flux-style runtime)
- Strong typing and clear architecture for safe feature growth
- Extensible Git backend via the `GitRepository` abstraction

## Current Capabilities

- Multi-panel TUI with focused keyboard workflow
- Repository status view (unstaged/staged/untracked) with tree navigation
- Diff preview with smooth scrolling
- Branch, commit, and stash listing panels
- Custom keymap support via `~/.config/ratagit/keymap.toml`

## Quick Start

### Prerequisites

- Rust (stable toolchain)
- A local Git repository to open in Ratagit

### Build

```bash
cargo build
```

### Run

Run Ratagit from inside any Git repository:

```bash
cargo run
```

## Default Keybindings

| Key | Action |
| --- | --- |
| `q` | Quit |
| `h` / `←` | Previous panel |
| `l` / `→` | Next panel |
| `1`-`4` | Jump to panel |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` | Stage/unstage selected file (Files panel) |
| `Enter` | Expand/collapse directory (Files panel) |
| `-` / `=` | Collapse/expand all directories |
| `Ctrl+U` / `Ctrl+D` | Scroll diff up/down |
| `r` | Refresh |

## Architecture

Ratagit follows a layered runtime design:

- UI -> `Action` -> `Dispatcher`/`Stores` -> Effect Runtime -> `GitRepository`
- Reducers remain pure; Git I/O is executed in the effect runtime
- UI renders from immutable snapshots, keeping rendering decoupled from mutation logic

Read more in:

- [Application architecture, module boundaries and refactoring roadmap](src/app/README.md)

## Tech Stack

- `ratatui` and `crossterm` for terminal UI
- `git2` for Git operations (current backend)
- `tokio` for async runtime and task orchestration
- `thiserror` and `color-eyre` for robust error handling

### Ratatui version and practices

Ratagit requires Ratatui **0.30.2 or newer within 0.30.x**. Ratatui 0.30.2
requires Rust 1.88 or newer; use the stable toolchain for this project and its
other dependencies.

- Enable only `crossterm_0_29`, `layout-cache`, and `underline-color`. This keeps
  the backend aligned with the direct Crossterm 0.29 dependency and retains
  layout caching without pulling in unused calendar widgets or macros.
- Keep the main `ratatui` crate for this application. The split `ratatui-core`
  crate is primarily useful for reusable widget libraries.
- Initialize application state before entering terminal mode. The async runtime
  uses `try_init` / `try_restore` and `DefaultTerminal`; synchronous applications
  can use `ratatui::run`. Install color-eyre before Ratatui so panic diagnostics
  appear after terminal restoration. Clean up mouse capture separately.
- Render complete frames from immutable snapshots when state changes, use
  `Frame::area()` for layout, and keep Git I/O outside rendering. Continue using
  `TestBackend` and golden buffers for layout, Unicode, styles, and cursor tests.

Features worth considering for future UI work:

- **0.30.1:** `Block::shadow` for modal overlays, multi-column table cells, and
  `Fill` for styled regions. Shadows would require updating overlay geometry
  and golden tests before enabling them.
- **0.30.2:** fixes wide-character cell cleanup, restores widgets' conditional
  `Send` / `Sync` guarantees, and fixes scrollbar thumbs at the end of a track.
  The new optional Termina backend is available, while this app uses Crossterm.
- `serde` can support saved themes if theme configuration is added. Enable
  optional or unstable features only when a concrete use case needs them.

References: [0.30.2 release highlights](https://ratatui.rs/highlights/v0302/),
[0.30.1 release highlights](https://ratatui.rs/highlights/v0301/),
[initialization APIs](https://docs.rs/ratatui/0.30.2/ratatui/init/index.html),
[feature definitions](https://github.com/ratatui/ratatui/blob/ratatui-v0.30.2/ratatui/Cargo.toml).

## Project Status

Ratagit is under active development.
Milestone execution is tracked in `.track/`.
If this project is useful to you, consider starring the repository to support visibility.

## Contributing

Contributions are welcome.
If you plan to contribute, start with the docs in `docs/` and open an issue/PR with clear reproduction and verification steps.

Useful local checks:

```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

UI regression tests use Ratatui's in-memory backend and checked-in golden
buffers. Run `cargo test ui::` to compare layouts, cell styles, and input cursors.
See [UI golden buffers](src/ui/goldens/README.md) for coverage and baseline updates.

## License

MIT
