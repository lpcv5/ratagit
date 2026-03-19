# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ratagit is a terminal Git client written in Rust, built on ratatui, inspired by lazygit. Phase 1 (MVP) is actively under development.

## Language Rules

- **All code comments, documentation, and doc comments must be written in English**
- **CLAUDE.md and all files in `docs/` must be written in English**
- Communication with the user is in Chinese (per session instructions)

## Common Commands

```bash
cargo build
cargo run
cargo test
cargo test test_name
cargo build --release
cargo check
cargo fmt
cargo clippy
```

## Core Architecture

### The Elm Architecture (TEA)

Unidirectional data flow:

```
User Input → Event → Message → Update → Model → View → Render
```

**Three core components**:

1. **Model**: Application state (`App` struct)
2. **Message**: User actions or system events (`Message` enum)
3. **Update**: Pure function that updates Model based on Message (`update()`)

### Module Structure

```
src/
├── app/
│   ├── app.rs        # App struct, main loop, key handling
│   ├── message.rs    # Message/event definitions
│   ├── update.rs     # TEA update function
│   └── command.rs    # Command definitions
├── git/
│   └── repository.rs # GitRepository trait + Git2Repository impl
├── ui/
│   ├── layout.rs     # Layout rendering
│   ├── panels/       # Panel renderers (files, diff, branches, commits, stash, command_log)
│   ├── widgets/      # Reusable widgets (file_tree)
│   └── views/        # View components (unused stubs)
├── config/
│   └── keymap.rs     # Keymap config (global + per-panel)
└── main.rs
```

## Key Technical Decisions

### GitRepository Trait Abstraction

**Important**: All Git operations must go through the `GitRepository` trait. Never call git2 directly.

```rust
pub trait GitRepository {
    fn status(&self) -> Result<GitStatus, GitError>;
    fn stage(&self, path: &PathBuf) -> Result<(), GitError>;
    fn unstage(&self, path: &PathBuf) -> Result<(), GitError>;
    fn diff_unstaged(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError>;
    fn diff_staged(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError>;
    fn diff_untracked(&self, path: &PathBuf) -> Result<Vec<DiffLine>, GitError>;
    fn workdir(&self) -> Option<PathBuf>;
}
```

Rationale: start with git2 (stable, well-documented), migrate to gix (pure Rust) in Phase 4+.

### Keymap System

Two-layer keymap stored in `~/.config/ratagit/keymap.toml`:
- `[global]` — active in all panels
- `[files]`, `[branches]`, `[commits]`, `[stash]` — panel-local bindings

Default global keys (lazygit-inspired):
- `h`/`l` or `Left`/`Right` — previous/next panel
- `1`-`4` — jump to panel directly
- `j`/`k` or `Up`/`Down` — list navigation
- `q` — quit, `r` — refresh
- `Ctrl+U`/`Ctrl+D` — scroll diff

Default files-panel local keys:
- `Enter`/`Space` — toggle directory expand/collapse
- `-` — collapse all, `=` — expand all

### File Tree Widget

`src/ui/widgets/file_tree.rs` — reusable `StatefulWidget`:
- `FileTree::from_git_status_with_expanded()` builds flat visible node list from git status + expanded dir set
- Directories shown with `▼`/`▶` arrows, files with status icons (`✚ ✎ ● ✖ ➜ ?`)
- Colors: green=staged/new, yellow=modified, red=deleted, gray=untracked, blue=directory

### Diff Display

- File node selected → show file diff (supports untracked files via full-content read)
- Directory node selected → aggregate diff of all files under it (max 2000 lines)
- `diff_scroll` offset in `App` controls visible window; resets to 0 on selection change

### Async Git Operations (Phase 2)

```rust
enum Command {
    Async(tokio::task::JoinHandle<Message>),
    Sync(Message),
}
```

Currently unused — Phase 1 uses synchronous git operations only.

## Development Roadmap

### Phase 1: MVP (current)
- [x] Basic event loop
- [x] Git status display with file tree
- [x] Diff preview (unstaged, staged, untracked)
- [x] File tree expand/collapse
- [x] Configurable keymap
- [ ] Stage/Unstage files
- [ ] Tab bar UI

### Phase 2: Core Features
- Commit functionality
- Commit history view
- Branch management
- Async git operations

### Phase 3: Advanced Features
- Interactive Rebase, Cherry-pick, Stash, Remote operations

### Phase 4: Polish
- Config system, themes, performance, test coverage

## Configuration

Keymap: `~/.config/ratagit/keymap.toml` (auto-created with defaults if missing)

Future: `~/.config/ratagit/config.toml` for general config

## Testing Strategy

- Unit tests: `#[cfg(test)]` inside modules
- Integration tests: `tests/` directory
- Use `tempfile` to create temporary Git repos for testing

Coverage targets: Phase 1-2 > 50%, Phase 3 > 70%, release > 80%

## Notes

1. **Git ops**: Always via `GitRepository` trait, never call git2/gix directly
2. **State updates**: Follow TEA — update state only through Messages
3. **Async**: Heavy git ops use `Command::Async` (Phase 2+)
4. **Errors**: Use `thiserror` for custom error types
5. **Comments**: All code comments and docs in English
