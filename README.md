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

Ratagit uses an event-driven architecture with clear separation between UI and Git operations:

- **Event Flow**: User Input → Component → AppEvent → Processor → Backend/State
- **Components** return `AppEvent` (no direct state mutation)
- **Processors** convert events to backend commands or state updates
- **Backend** executes Git operations asynchronously on a separate task

Key benefits:
- Unidirectional data flow (predictable state changes)
- Type-safe event handling (strongly typed enums)
- Responsive UI (Git I/O never blocks rendering)
- Easy testing (processors are simple, pure functions)

Read more in `docs/ARCHITECTURE.md`.

## Tech Stack

- `ratatui` and `crossterm` for terminal UI
- `git2` for Git operations (current backend)
- `tokio` for async runtime and task orchestration
- `thiserror` and `color-eyre` for robust error handling

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

## License

MIT
