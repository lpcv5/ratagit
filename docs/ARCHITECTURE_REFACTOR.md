# Architecture Analysis & Refactor Plan

This document captures the architecture pain points in ratagit and proposes a hybrid TEA + Component refactoring strategy to improve maintainability and reduce feature-induced regressions.

---

## Table of Contents

- [Current Architecture Overview](#current-architecture-overview)
- [Problem Diagnosis](#problem-diagnosis)
  - [Critical Issues](#critical-issues)
  - [High Severity Issues](#high-severity-issues)
  - [Medium Severity Issues](#medium-severity-issues)
- [Architecture Pattern Comparison](#architecture-pattern-comparison)
  - [TEA (The Elm Architecture)](#tea-the-elm-architecture)
  - [Component Architecture](#component-architecture)
  - [Flux Architecture](#flux-architecture)
  - [Verdict for Ratagit](#verdict-for-ratagit)
- [Proposed Solution: Hybrid TEA + Component](#proposed-solution-hybrid-tea--component)
  - [Core Design](#core-design)
  - [Unified TreeModeState](#unified-treemodestate)
  - [Update Dispatch After Refactor](#update-dispatch-after-refactor)
- [Incremental Refactor Roadmap](#incremental-refactor-roadmap)
  - [Step 1: Extract PanelState Structs](#step-1-extract-panelstate-structs)
  - [Step 2: Introduce PanelComponent Trait](#step-2-introduce-panelcomponent-trait)
  - [Step 3: Decouple Rendering](#step-3-decouple-rendering)
  - [Step 4: Componentize Search and Input Modes](#step-4-componentize-search-and-input-modes)
- [Bug Prevention Mechanisms](#bug-prevention-mechanisms)

---

## Current Architecture Overview

Ratagit follows the Elm Architecture (TEA) with modular update handlers:

```
User Input -> Event -> Message -> update() -> App (Model) -> view() -> Render
```

Key files:

| File | Role |
|------|------|
| `src/app/app.rs:77-137` | `App` struct (central state, ~40+ fields) |
| `src/app/message.rs:1-59` | `Message` enum (~25 variants) |
| `src/app/update.rs:8-51` | Dispatch function routing messages to handlers |
| `src/app/update_handlers/*.rs` | Modular handlers (navigation, staging, commit, branch, stash, revision, search, quit) |
| `src/app/command.rs:1-14` | `Command` enum (None, Async, Sync) |
| `src/ui/layout.rs:16-111` | Layout rendering (30/70 split, panel composition) |
| `src/ui/panels/*.rs` | Individual panel render functions |
| `src/app/selection.rs:14-325` | Visual selection mode + batch operations |
| `src/app/diff_loader.rs:27-77` | Diff loading and caching |
| `src/app/revision_tree.rs` | Commit/stash tree expansion state |

---

## Problem Diagnosis

### Critical Issues

**1. God Object (`App` struct)**
- Location: `src/app/app.rs:77-137`
- The `App` struct contains 40+ fields managing all application state in a single struct.
- Every handler and every panel render function takes `&mut App` or `&App`, creating implicit dependencies on the entire state.
- Adding a new feature means adding more fields to this already bloated struct.

**2. State Duplication (commit_tree vs stash_tree)**
- Location: `src/app/app.rs:100-111`
- Nearly identical field sets exist for commit tree mode and stash tree mode:
  - `commit_tree_mode`, `commit_tree_nodes`, `commit_tree_files`, `commit_tree_expanded_dirs`, `commit_tree_commit_oid`
  - `stash_tree_mode`, `stash_tree_nodes`, `stash_tree_files`, `stash_tree_expanded_dirs`, `stash_tree_stash_index`
- Fixing a bug in one set requires remembering to fix the other. This is a major source of regressions.

**3. Tight Coupling Between Panels and App**
- Location: all handlers in `src/app/update_handlers/*.rs` and panels in `src/ui/panels/*.rs`
- Panel render functions receive `&App` and read multiple unrelated fields directly.
- Handlers receive `&mut App` and mutate fields across different concerns (e.g., navigation handler loads commits, staging handler changes input mode).
- No compile-time boundary prevents a handler from touching state it shouldn't.

### High Severity Issues

**4. Manual Tree Traversal is Error-Prone**
- Location: `src/app/selection.rs`
- Functions like `subtree_end_index()` do manual depth-based tree walking.
- The flat node list must stay perfectly in sync with the expanded directory set; any mismatch causes index bugs.
- Adding new node types or tree behaviors is risky.

**5. Monolithic Dispatch**
- Location: `src/app/update.rs:9-51`
- Adding a new message variant requires editing: (1) `Message` enum, (2) `update.rs` dispatch, (3) a handler file, (4) possibly `App` fields.
- Missing any step compiles fine but causes runtime bugs (unhandled message silently ignored).

**6. Scattered Concerns in Handlers**
- Location: `src/app/update_handlers/navigation.rs`, `staging.rs`
- Navigation handler does git ops (`ensure_commits_loaded`).
- Staging handler manages input mode transitions (`StartCommitInput`).
- No clear boundary of responsibility per handler.

**7. Missing Precondition Validation**
- Location: all handlers
- Handlers don't check preconditions (e.g., is `is_fetching_remote` already true? Is there already an open editor?).
- Some guards exist (e.g., `StartCommitInput` checks for open editor) but this is inconsistent.

### Medium Severity Issues

**8. Hardcoded Layout Constants**
- Location: `src/ui/layout.rs:24-59`
- The 30/70 horizontal split and 40:30:30 vertical split are magic numbers.
- Not configurable, hard to adjust for different terminal sizes or user preferences.

**9. Modal Editor Coupling**
- Location: `src/ui/layout.rs:109-110`
- Commit and stash editors render as overlays after all panels, tightly coupled to the layout function.
- Adding a new modal (e.g., confirmation dialog, merge conflict editor) requires editing the central layout function.

**10. No Panel Composition**
- Location: `src/ui/panels/*.rs`
- Panels are standalone render functions, not composable.
- Search highlighting is duplicated across panels.
- Cannot nest or reuse panel sub-components.

**11. Limited Async Model**
- Location: `src/app/command.rs:1-14`, `src/app/app.rs:545-557`
- Only supports one concurrent async operation (fetch).
- No cancellation, no progress tracking, no queuing.
- Adding more async ops (push, pull, rebase) would require significant rework.

**12. Manual Dirty Flag Management**
- Location: all handlers
- Every handler must manually call `app.dirty.mark()` after state changes.
- Forgetting this call means the UI doesn't redraw, causing stale display bugs.

**13. Repetitive Logging Pattern**
- Location: all handlers
- Every handler duplicates the same pattern: `app.push_log(format!(...), success)`.
- No centralized error handling or logging middleware.

**14. Manual Diff Cache Keys**
- Location: `src/app/diff_loader.rs`
- `files_hash` for directory diff cache keys is a SHA1 of all file paths, recomputed on every cache check.
- No invalidation strategy for directory diffs after file changes.

---

## Architecture Pattern Comparison

### TEA (The Elm Architecture)

**How it works:**
```
User Input -> Message -> update(model, message) -> new model -> view(model) -> render
```

**Strengths:**
- Single source of truth (centralized state)
- Predictable state transitions
- Excellent testability (pure update function)
- Message chaining via `Option<Message>` return

**Weaknesses:**
- Model grows into God Object as app scales
- Update function becomes massive dispatcher
- All state changes flow through one bottleneck

**Ratagit's current mitigation:** Split update into `update_handlers/` modules. This helps but doesn't solve the God Object problem.

### Component Architecture

**How it works:**
```rust
trait Component {
    fn handle_event(&mut self, event: Event) -> Option<Action>;
    fn update(&mut self, action: Action) -> Option<Action>;
    fn draw(&mut self, frame: &mut Frame, area: Rect);
}
```

Components are self-contained: each owns its state, handles its events, renders itself.
App stores `Vec<Box<dyn Component>>` and broadcasts actions to all.

**Strengths:**
- Strong isolation between components
- Add/remove UI sections without touching central code
- Each component is independently testable

**Weaknesses:**
- Cross-component coordination is harder (no single source of truth)
- Action broadcasting to all components is wasteful
- Ratagit has heavy inter-panel dependencies (file selection -> diff display, panel switch -> data loading)

### Flux Architecture

**How it works:**
```
User Input -> Action -> Dispatcher -> Store(s) -> View re-renders
```

Multiple stores handle different domains, dispatcher routes actions.

**Strengths:**
- Domain separation via multiple stores
- Good for async-heavy apps with multiple data sources
- Clean separation of mutation logic

**Weaknesses:**
- More boilerplate than TEA
- Dispatcher layer feels redundant for single-developer projects
- Overkill for ratagit's current complexity

### Verdict for Ratagit

**Pure TEA** is insufficient — the God Object problem will only worsen as we approach lazygit parity.

**Pure Component** is a poor fit — ratagit's panels are too interconnected (selecting a file must update the diff panel, switching panels must load data).

**Hybrid TEA + Component** is the best path:
- Keep TEA's centralized message flow for cross-cutting coordination
- Use Component trait to encapsulate per-panel state and rendering
- Share only what's needed via a controlled `PanelContext`

---

## Proposed Solution: Hybrid TEA + Component

### Core Design

```rust
/// Each panel implements this trait
trait PanelComponent {
    /// Handle a message relevant to this panel
    fn update(&mut self, msg: &Message, ctx: &mut PanelContext) -> Option<Message>;

    /// Render this panel
    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelContext);

    /// Whether this panel handles a given message
    fn handles(&self, msg: &Message) -> bool;

    /// Search support
    fn search(&self, query: &str) -> Vec<usize> { vec![] }
}

/// Shared context passed to panels (replaces direct &App access)
struct PanelContext<'a> {
    repo: &'a dyn GitRepository,
    active_panel: SidePanel,
    keymap: &'a Keymap,
    diff_cache: &'a mut DiffCache,
    command_log: &'a mut Vec<CommandLogEntry>,
    outbox: Vec<Message>,  // inter-panel communication
}
```

The refactored `App` struct:

```rust
struct App {
    // Shared state (cross-panel concerns)
    repo: Box<dyn GitRepository>,
    active_panel: SidePanel,
    keymap: Keymap,
    diff_cache: DiffCache,
    command_log: Vec<CommandLogEntry>,
    input_mode: Option<InputMode>,
    dirty: DirtyFlags,

    // Panel components (each owns its state)
    files_panel: FilesPanelComponent,
    branches_panel: BranchesPanelComponent,
    commits_panel: CommitsPanelComponent,
    stash_panel: StashPanelComponent,
    diff_panel: DiffPanelComponent,
}
```

Example panel component:

```rust
struct FilesPanelComponent {
    list_state: ListState,
    tree_nodes: Vec<FileTreeNode>,
    expanded_dirs: HashSet<PathBuf>,
    visual_mode: bool,
    visual_anchor: Option<usize>,
    search_query: String,
    search_matches: Vec<usize>,
}

impl PanelComponent for FilesPanelComponent {
    fn handles(&self, msg: &Message) -> bool {
        matches!(msg,
            Message::StageFile(_) | Message::UnstageFile(_) |
            Message::ToggleDir | Message::ToggleVisualSelectMode |
            Message::CollapseAll | Message::ExpandAll |
            Message::DiscardSelection | Message::DiscardPaths(_)
        )
    }

    fn update(&mut self, msg: &Message, ctx: &mut PanelContext) -> Option<Message> {
        match msg {
            Message::ToggleDir => {
                self.toggle_selected_dir();
                self.rebuild_tree_nodes();
                ctx.outbox.push(Message::DiffReloadNeeded);
                None
            }
            Message::StageFile(path) => {
                match ctx.repo.stage(path) {
                    Ok(()) => {
                        ctx.command_log.push(log_entry("staged", true));
                        ctx.outbox.push(Message::RefreshStatus);
                    }
                    Err(e) => ctx.command_log.push(log_entry(&e.to_string(), false)),
                }
                None
            }
            _ => None,
        }
    }

    fn draw(&self, frame: &mut Frame, area: Rect, ctx: &PanelContext) {
        let is_active = ctx.active_panel == SidePanel::Files;
        let tree = FileTree::new(&self.tree_nodes, is_active);
        frame.render_stateful_widget(tree, area, &mut self.list_state.clone());
    }
}
```

### Unified TreeModeState

Eliminates the duplication between `commit_tree_*` and `stash_tree_*`:

```rust
/// Generic tree mode state, shared by Commits and Stash panels
struct TreeModeState {
    active: bool,
    nodes: Vec<FileTreeNode>,
    files: Vec<FileEntry>,
    expanded_dirs: HashSet<PathBuf>,
    source_id: String,  // commit oid or stash index as string
    list_state: ListState,
}

struct CommitsPanelComponent {
    list_state: ListState,
    commits: Vec<CommitInfo>,
    tree_mode: Option<TreeModeState>,  // None = list mode
    commits_dirty: bool,
}

struct StashPanelComponent {
    list_state: ListState,
    stashes: Vec<StashInfo>,
    tree_mode: Option<TreeModeState>,  // Same type, no duplication
}
```

### Update Dispatch After Refactor

```rust
fn update(app: &mut App, msg: Message) -> Option<Command> {
    // 1. Handle global messages first
    match &msg {
        Message::Quit => return handle_quit(app),
        Message::PanelNext | Message::PanelPrev | Message::PanelGoto(_) => {
            return handle_panel_switch(app, &msg);
        }
        Message::RefreshStatus => return handle_refresh(app),
        _ => {}
    }

    // 2. Route to active panel's component
    let mut ctx = PanelContext::from_app(app);
    let result = match app.active_panel {
        SidePanel::Files => app.files_panel.update(&msg, &mut ctx),
        SidePanel::LocalBranches => app.branches_panel.update(&msg, &mut ctx),
        SidePanel::Commits => app.commits_panel.update(&msg, &mut ctx),
        SidePanel::Stash => app.stash_panel.update(&msg, &mut ctx),
    };

    // 3. Process outbox (inter-panel messages)
    for follow_up in ctx.outbox.drain(..) {
        update(app, follow_up);
    }

    // 4. Auto-mark dirty (no manual marking needed)
    app.dirty.mark();

    result.map(|m| Command::Sync(m))
}
```

---

## Incremental Refactor Roadmap

Each step keeps the project compilable and runnable. No big-bang rewrite.

### Step 1: Extract PanelState Structs

**Goal:** Group App fields into per-panel structs without changing behavior.

**Before:**
```rust
struct App {
    files_panel: PanelState,
    file_tree_nodes: Vec<FileTreeNode>,
    expanded_dirs: HashSet<PathBuf>,
    files_visual_mode: bool,
    files_visual_anchor: Option<usize>,
    // ... 35 more fields
}
```

**After:**
```rust
struct FilesPanelState {
    list_state: ListState,
    tree_nodes: Vec<FileTreeNode>,
    expanded_dirs: HashSet<PathBuf>,
    visual_mode: bool,
    visual_anchor: Option<usize>,
}

struct App {
    files: FilesPanelState,
    branches: BranchesPanelState,
    commits: CommitsPanelState,
    stash: StashPanelState,
    // shared state remains flat
}
```

Also unify `commit_tree_*` and `stash_tree_*` into `TreeModeState`.

**Risk:** Low. Pure struct reorganization. All existing code updates field paths (`app.file_tree_nodes` -> `app.files.tree_nodes`).

### Step 2: Introduce PanelComponent Trait

**Goal:** Define `PanelComponent` trait and implement it for each panel.

**Actions:**
1. Define the `PanelComponent` trait and `PanelContext` struct.
2. Implement the trait for `FilesPanelState` (renamed to `FilesPanelComponent`).
3. Update `update.rs` dispatch to route file-related messages through `files_panel.update()`.
4. Migrate one panel at a time: Files -> Branches -> Commits -> Stash.

**Risk:** Medium. Requires careful message routing. Test each panel migration independently.

### Step 3: Decouple Rendering

**Goal:** Move panel rendering from standalone functions to `PanelComponent::draw()`.

**Actions:**
1. Each panel's `draw()` receives only `&self`, `Frame`, `Rect`, and `&PanelContext`.
2. Remove direct `&App` parameter from render functions.
3. Update `render_layout()` to call `panel.draw()` methods.

**Risk:** Medium. Rendering code must be updated to read from component state instead of App fields. Potential for missed field access.

### Step 4: Componentize Search and Input Modes

**Goal:** Extract search state and input mode handling into standalone components.

**Actions:**
1. Create `SearchComponent` that handles `SearchSetQuery`, `SearchConfirm`, `SearchNext`, `SearchPrev`.
2. Create `InputModeComponent` that manages commit/branch/stash editors.
3. These components communicate with panels via the `outbox` message queue.

**Risk:** Medium-High. Search interacts with all panels. Input mode transitions are complex. Recommend thorough testing.

---

## Bug Prevention Mechanisms

These mechanisms specifically address the original problem of "adding features causes bugs":

**1. Component Isolation**
- Each panel can only modify its own state fields.
- Cross-panel effects go through `ctx.outbox` messages, making dependencies explicit.
- A new feature in the Files panel cannot accidentally corrupt Commits panel state.

**2. Type System Constraints**
- `PanelContext` exposes only the shared state a panel needs.
- The compiler prevents a panel from accessing another panel's state directly.
- New fields added to one panel don't affect others.

**3. Unified TreeModeState**
- One struct, two users (commits + stash).
- Bug fix in tree traversal applies to both automatically.
- No more "fixed it for commits but forgot stash" regressions.

**4. Explicit Message Chains**
- `update()` returns `Option<Message>` for follow-up actions.
- `ctx.outbox` makes inter-panel communication visible and traceable.
- No hidden side effects from one handler silently mutating another panel's state.

**5. Automatic Dirty Marking**
- Dirty flag is set automatically in the dispatch function after any `update()` call.
- Eliminates the entire class of "forgot to mark dirty" bugs.

**6. Precondition Guards (recommended addition)**
```rust
impl PanelComponent for FilesPanelComponent {
    fn update(&mut self, msg: &Message, ctx: &mut PanelContext) -> Option<Message> {
        // Guard: don't process file ops during active input mode
        if ctx.input_mode.is_some() {
            return None;
        }
        // ...
    }
}
```
Centralize precondition checks at the component level instead of scattering them across handlers.
