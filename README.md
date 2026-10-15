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
| `?` | Open contextual keybindings help |
| `h` / `←` | Previous panel |
| `l` / `→` | Next panel |
| `1`-`4` | Jump to panel |
| `j` / `↓` | Move down |
| `k` / `↑` | Move up |
| `Space` | Stage/unstage selected file (Files panel) |
| `a` | Stage all changes, or unstage all when everything is staged (Files panel) |
| `A` | Amend HEAD with staged changes, preserving its message and author (Files panel) |
| `e` | Edit the selected file in the configured Git editor (Files panel) |
| `Ctrl+O` | Copy the selected repository-relative path (Files panel) |
| `i` | Add the selected untracked file/directory to the root `.gitignore` (Files panel) |
| `Enter` | Open a file's diff, or expand/collapse a directory (Files panel) |
| `Tab` / `Shift+Tab` | Next/previous side panel; `Tab` switches staged/unstaged sides within a diff |
| `-` / `=` | Collapse/expand all directories |
| `Ctrl+U` / `Ctrl+D` | Scroll diff up/down |
| `r` | Refresh |

Press `?` for the keybindings menu. Local lists commands for the current panel
or diff, Global lists shared commands and navigation, and Custom lists built-in
commands whose bindings you have changed in `keymap.toml`. Only effective,
currently available bindings are shown; conflicting and unbound shortcuts are
omitted. Custom shell commands are not currently supported.

In help, use `j`/`k` or arrows to select, `PageUp`/`PageDown` to jump,
`Home`/`End` to reach either end, and `Enter` or `Space` to execute the selected
command. `/` filters descriptions, command names and keys. `Esc` clears an active
filter first, then closes help; `?` or `q` also close it when not filtering.
Closing help preserves the underlying selection and diff focus. The help key
can be changed with `[global] help = ["F1"]` in `keymap.toml`; text editors
continue to accept `?` as text.

Editing follows Git's editor selection (`GIT_EDITOR`, `core.editor`, `VISUAL`,
`EDITOR`, then Git's default). Editor commands support quoted executable paths
and arguments, such as `"C:/Program Files/Editor/editor.exe" --wait`; file paths
are passed as separate arguments. Use the editor's wait option for GUI editors.
The TUI releases the terminal while the editor runs, resumes afterward, and
refreshes file status. Shell pipelines and substitutions in editor commands are
not supported.

Ignoring adds an anchored literal rule, leaves existing rules intact, and does
not stage `.gitignore` or untrack files. Editing, copying and ignoring operate on
a single selection; directory editing is unavailable. Clipboard access requires
a desktop clipboard service and reports an error if one is unavailable.

Files and diff navigation follow Lazygit's key conventions. Press `Enter` on a
file to focus its diff, starting in hunk mode. `0` also focuses the main diff,
including directory previews. Inside the diff, `j`/`k` (or up/down) select the
next/previous hunk, `h`/`l` (or left/right) also jump between hunks, and `a` switches between
line and hunk selection. `v` starts/finishes a range in either mode. `Space`
immediately stages an unstaged selection or unstages a staged selection;
without a range it acts on the cursor. The cursor stays on the changed hunk
after staging or unstaging, so pressing `Space` again toggles it back. In line
mode, `j`/`k` move through individual lines. Each path appears once in the file list,
and its staged and unstaged hunks share one diff, ordered by position. `Tab`
jumps between staged and unstaged hunks. `n`/`N` move to the next/previous visible
file while keeping diff focus, and `Esc` returns to the file list and clears the
selection.

The diff shows old/new line numbers, with a blank on the side where a line does
not exist. `>` marks the cursor and a blue rail marks the selected range.
Hunk selection highlights only added and removed lines. Staged changes use
stronger green/red shading on those lines; headers and unchanged context keep
their normal colors, without staged/unstaged badges. Mixed staged and unstaged
edits retain the native Git diff hunk layout and HEAD/worktree line numbers
when their changes can be represented together. The header shows the file and change
counts, while the bottom border shows selection mode,
position and range status. Line numbers hide automatically in narrow panes.
Hunk navigation scrolls the full selected change block into view when it fits;
larger blocks start at the top of the pane. Staging and unstaging preserve the content's
screen position, including manual scrolling inside a large hunk and adjacent
hunks merging after a write.
Selection in hunk mode uses individual contiguous change blocks. A replacement's
removed and added lines form one block; unchanged context separates the next
block, even inside the same Git hunk. `j`/`k`, highlighting, ranges and `Space`
operate on these blocks, and the cursor marks the first changed line. Staging
one block leaves its neighbors unstaged.

Older generated default bindings are migrated when loaded, including the former
`Tab` focus, `Enter` side switch and `s`/`u` staging keys. Custom bindings remain
intact, and loading preserves the configuration file and its comments. New
configurations include `schema_version = 1`; setting this explicitly also allows
intentional overrides that match a former default.

Within the diff, `d` discards only the selected unstaged lines or hunks, leaving
the index and other changes intact. On the staged side, `d` only unstages the
selection and preserves the working file. A directory diff can select changes
across files, but each operation must use one source (staged or unstaged).
Historical, binary, truncated, rename, symlink and mode-changing patches remain
read-only. Before applying, the backend checks the reviewed patch against fresh
repository state; stale selections fail and trigger a refresh.

## Architecture

`src/main.rs` composes startup. `src/terminal.rs` owns terminal lifecycle and
restoration; `src/runtime/` owns channels, task scheduling, input/render loops,
shutdown and telemetry. Application state stays in the dispatch loop, and UI
rendering reads immutable snapshots.

Feature collections and diff text share immutable payloads across snapshots;
state writes detach changed payloads, and rendering borrows snapshot metadata.
Allocation measurements and the repeatable profiling command are documented in
the application architecture guide below.

Background reads carry one complete request from registration through execution.
A single task registry accepts generations, and completion events share immutable
payloads with feature state while retaining epoch and source checks.

Files, diff, revisions and search own feature state and pure update decisions.
Application coordinators adapt cross-feature data and apply navigation or scheduling
intentions; search queries and matches are separate from editor input buffers.

`src/git/repository.rs` composes the single Git2 backend. Private modules under
`src/git/repository/` own the existing worktree, patch, history, reference, stash,
commit and operation capabilities, with persistent worker routing kept explicit.

Ratagit follows a layered runtime design:

- UI -> `Action` -> `Dispatcher`/`Stores` -> Effect Runtime -> `GitRepository`
- Reducers remain pure; Git I/O is executed in the effect runtime
- UI renders from immutable snapshots, keeping rendering decoupled from mutation logic

Read more in:

- [Application architecture, module boundaries and refactoring roadmap](src/app/README.md)
- [Rust practices from uv and adoption priorities for Ratagit](docs/uv-rust-practices.md)
- [Profiling builds and end-to-end interaction latency](docs/profiling.md)
- [Diff cache memory budget, identity and freshness](docs/diff-cache.md)

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

CI runs the four checks on Ubuntu and Windows with the committed lockfile.
`cargo test process_tests` exercises isolated child processes for configuration
recovery, Unicode paths, CRLF patches, Git diagnostics, and worker shutdown.
`cargo test --test terminal_lifecycle` checks startup failures on both platforms
and real terminal cleanup and profiling on Unix. Each process fixture owns its
Git configuration, home/config directories, and temporary files; children have
bounded lifetimes. Windows console lifecycle coverage still needs a dedicated
console harness. See [platform test details](docs/uv-rust-practices.md#extend-existing-snapshots-and-platform-tests).

## License

MIT
