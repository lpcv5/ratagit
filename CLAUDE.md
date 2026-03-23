# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Ratagit is a terminal Git client written in Rust, built on ratatui, inspired by lazygit. M1-M3 are complete, with M4 (Release Readiness) in draft stage.

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

### Flux + Tokio Architecture (ADR-015)

Unidirectional data flow:

```
UI → Action → Dispatcher → Stores → Effect Runtime → AppStateSnapshot → UI
```

**Core components**:
1. **Action**: User input or system events (`DomainAction`, `UiAction`, `SystemAction`)
2. **Dispatcher**: Routes `ActionEnvelope` through ordered list of `Store` reducers
3. **Stores**: Domain-partitioned reducers in `src/flux/stores/`
4. **Effects**: Async Git I/O via `EffectRequest` → `effects::run()` → `EffectResultAction`
5. **AppStateSnapshot**: Read-only view of `App` passed to UI renderer

### Module Structure

```
src/
├── app/
│   ├── app.rs            # App struct and state model
│   ├── command.rs        # Command definitions
│   ├── input_mode.rs     # Input mode state (Commit/Branch/Stash/Search)
│   ├── selection.rs      # Selection state management
│   ├── search.rs         # Search query management
│   ├── revision_tree.rs  # Commit tree expansion state
│   ├── panel_nav.rs      # Panel navigation utilities
│   ├── selectors.rs      # Selector trait and implementations
│   ├── diff_loader.rs    # Diff loading utilities
│   ├── refresh.rs        # State refresh utilities
│   ├── hints.rs          # UI hint generation
│   └── test_dispatch.rs  # Test-only Flux helpers for action/key dispatch
├── flux/
│   ├── action.rs         # Action/DomainAction/SystemAction enums
│   ├── dispatcher.rs     # Dispatcher: routes ActionEnvelope → Stores
│   ├── effects.rs        # EffectRequest + async run() for Git I/O
│   ├── input_mapper.rs   # Key events → Action mapping
│   ├── snapshot.rs       # AppStateSnapshot (read-only view for UI)
│   └── stores/           # Domain stores (files, branches, commits, stash, etc.)
├── git/
│   └── repository.rs     # GitRepository trait + Git2Repository impl
├── ui/
│   ├── layout.rs         # Layout rendering
│   ├── theme.rs          # Color theme
│   ├── highlight.rs      # Search highlighting
│   ├── panels/           # Panel renderers (files, diff, branches, commits, stash, revision_tree, etc.)
│   └── widgets/          # Reusable widgets (file_tree)
├── config/
│   ├── mod.rs            # Config loading
│   └── keymap.rs         # Keymap config (global + per-panel)
└── main.rs
```

## Key Technical Decisions

### GitRepository Trait Abstraction

**Important**: All Git operations must go through the `GitRepository` trait. Never call git2 directly.

```rust
pub trait GitRepository {
    // Status and staging
    fn status(&self) -> Result<GitStatus, GitError>;
    fn stage(&self, path: &Path) -> Result<(), GitError>;
    fn stage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;
    fn unstage(&self, path: &Path) -> Result<(), GitError>;
    fn unstage_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;
    fn discard_paths(&self, paths: &[PathBuf]) -> Result<(), GitError>;

    // Diff operations
    fn diff_unstaged(&self, path: &Path) -> Result<Vec<DiffLine>, GitError>;
    fn diff_staged(&self, path: &Path) -> Result<Vec<DiffLine>, GitError>;
    fn diff_untracked(&self, path: &Path) -> Result<Vec<DiffLine>, GitError>;

    // Branch operations
    fn branches(&self) -> Result<Vec<BranchInfo>, GitError>;
    fn create_branch(&self, name: &str) -> Result<(), GitError>;
    fn checkout_branch(&self, name: &str) -> Result<(), GitError>;
    fn delete_branch(&self, name: &str) -> Result<(), GitError>;

    // Commit operations
    fn commits(&self, limit: usize) -> Result<Vec<CommitInfo>, GitError>;
    fn commit_files(&self, oid: &str) -> Result<Vec<FileEntry>, GitError>;
    fn commit_diff_scoped(&self, oid: &str, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError>;
    fn commit(&self, message: &str) -> Result<String, GitError>;

    // Stash operations
    fn stashes(&self) -> Result<Vec<StashInfo>, GitError>;
    fn stash_files(&self, index: usize) -> Result<Vec<FileEntry>, GitError>;
    fn stash_diff(&self, index: usize, path: Option<&Path>) -> Result<Vec<DiffLine>, GitError>;
    fn stash_push_paths(&self, paths: &[PathBuf], message: &str) -> Result<usize, GitError>;
    fn stash_apply(&self, index: usize) -> Result<(), GitError>;
    fn stash_pop(&self, index: usize) -> Result<(), GitError>;
    fn stash_drop(&self, index: usize) -> Result<(), GitError>;

    // Async operations
    fn fetch_default_async(&self) -> Result<Receiver<Result<String, GitError>>, GitError>;
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
- `Enter`/`Space` — toggle directory expand/collapse, stage/unstage file
- `-` — collapse all, `=` — expand all
- `v` — toggle visual selection mode
- `c` — commit staged changes
- `S` — stash selected files
- `D` — discard selected changes

### Flux Dispatcher and Stores

`Dispatcher::with_default_stores()` creates ordered stores; each `Store` implements:
```rust
pub trait Store {
    fn reduce(&mut self, action: &ActionEnvelope, ctx: &mut ReduceCtx<'_>) -> ReduceOutput;
}
```
Stores: `InputStore`, `QuitStore`, `OpsStore`, `RevisionStore`, `NavigationStore`,
`SelectionStore`, `SearchStore`, `DiffStore`, `OverlayStore`, `FilesStore`,
`BranchStore`, `StashStore`, `CommitStore`.

Git I/O is done exclusively in `effects.rs` via `EffectRequest` — stores must NOT do I/O.

For tests, `src/app/test_dispatch.rs` provides `dispatch_test_action()` and
`dispatch_test_key()` helpers, and key mapping must go through `flux::input_mapper`.

### Input Modes

The app supports multiple input modes managed by `InputMode` enum:
- `CommitEditor` — commit message input
- `CreateBranch` — new branch name input
- `StashEditor` — stash message input
- `Search` — search query input

When in input mode, key events go to the text input instead of normal navigation.

### Visual Selection Mode

Files panel supports visual selection (like vim):
- Press `v` to toggle visual mode
- Use `j`/`k` to extend selection from anchor point
- Batch operations (stage/unstage/discard/stash) apply to all selected files

### Search Functionality

- `/` or `s` starts search input
- Search highlights matches in current panel
- `n`/`N` navigates to next/previous match
- Search scope is panel-specific (files, branches, commits, stash)

### File Tree Widget

`src/ui/widgets/file_tree.rs` — reusable `StatefulWidget`:
- `FileTree::from_git_status_with_expanded()` builds flat visible node list from git status + expanded dir set
- Directories shown with `▼`/`▶` arrows, files with status icons (`✚ ✎ ● ✖ ➜ ?`)
- Colors: green=staged/new, yellow=modified, red=deleted, gray=untracked, blue=directory

### Diff Display

- File node selected → show file diff (supports untracked files via full-content read)
- Directory node selected → aggregate diff of all files under it (max 2000 lines)
- Commit node selected → show commit diff
- Stash node selected → show stash diff
- `diff_scroll` offset in `App` controls visible window; resets to 0 on selection change

### Revision Tree (Commit Panel)

Commits panel supports tree navigation:
- Press `Enter` to expand commit and show changed files
- File tree appears under the commit with expand/collapse support
- Select file to see commit-scoped diff for that file
- Press `Escape` to collapse tree

Managed by `revision_tree.rs` module, stores expanded state per commit.

### Async Git Operations

Async/side-effect operations are modeled as `Command::Effect(EffectRequest)` and executed by
the effect loop. Result actions are fed back into dispatcher as `Action::System(...)`.

## Configuration

Keymap: `~/.config/ratagit/keymap.toml` (auto-created with defaults if missing)

Future: `~/.config/ratagit/config.toml` for general config

## Testing Strategy

- Unit tests: `#[cfg(test)]` inside modules
- Integration tests: `tests/` directory (if needed)
- Use `tempfile` to create temporary Git repos for testing
- GitRepository tests in `src/git/repository.rs` demonstrate testing pattern

Coverage targets: M1-M2 > 50%, M3 > 70%, M4 (release) > 80%

## Notes

1. **Git ops**: Always via `GitRepository` trait, never call git2/gix directly
2. **State updates**: Follow Flux — update state only through `Action -> Dispatcher -> Stores`
3. **Async**: Heavy git ops use `Command::Effect(EffectRequest)` via effect runtime
4. **Errors**: Use `thiserror` for custom error types
5. **Comments**: All code comments and docs in English
6. **Batch operations**: Use `*_paths()` methods (e.g., `stage_paths`, `unstage_paths`, `discard_paths`) for multi-file operations
7. **External git**: Some operations (stash, commit diff, discard) shell out to `git` CLI for reliability

## Implementation Patterns

### Git Operations

- Most operations use git2 directly
- Complex operations (stash, scoped diffs) shell out to `git` CLI via `run_git()` helper
- Batch operations use `*_paths()` variants for efficiency
- Async operations use `std::sync::mpsc` channels (not tokio tasks yet)

```
