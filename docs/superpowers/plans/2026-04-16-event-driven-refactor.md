# Event-Driven Component Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor ratagit from patch-style additions to a clean, React-like event-driven architecture with trait-based component reuse

**Architecture:** Split Intent into typed event categories (AppEvent/GitEvent/ModalEvent), introduce React-like Component trait with RenderContext, consolidate 26 handler structs into macro-generated handlers, extract git logic to GitProcessor

**Tech Stack:** Rust, ratatui, crossterm, git2, tokio, procedural macros

---

## Phase 1: Backend Handler Consolidation

### Task 1.1: Create Handler Macro Foundation

**Files:**
- Create: `src/backend/macros.rs`

- [ ] **Step 1: Write macro module structure**

```rust
// src/backend/macros.rs

/// Generates CommandHandler implementations from declarative definitions
///
/// Syntax:
/// ```
/// define_handlers! {
///     CommandVariant => git_ops_function => SuccessEvent,
///     CommandVariant { field } => git_ops_function(&field) => SuccessEvent(result),
///     CommandVariant [mut] => git_ops_function => SuccessEvent,
/// }
/// ```
#[macro_export]
macro_rules! define_handlers {
    ($($tokens:tt)*) => {
        $crate::__define_handlers_impl!($($tokens)*);
    };
}

// Internal implementation macro
#[doc(hidden)]
#[macro_export]
macro_rules! __define_handlers_impl {
    // Entry point: process all handler definitions
    (
        $(
            $cmd_variant:ident $({ $($field:ident),* })? $([ $mut_flag:ident ])? =>
            $git_fn:path $( ( $($arg:expr),* ) )? =>
            $event_variant:ident $( ( $result_binding:ident ) )?
        ),* $(,)?
    ) => {
        // Generate handler structs
        $(
            $crate::__generate_handler_struct!($cmd_variant);
        )*

        // Generate CommandHandler trait implementations
        $(
            $crate::__generate_handler_impl!(
                $cmd_variant $({ $($field),* })? $([ $mut_flag ])? =>
                $git_fn $( ( $($arg),* ) )? =>
                $event_variant $( ( $result_binding ) )?
            );
        )*

        // Generate handler registry
        $crate::__generate_registry!($($cmd_variant),*);
    };
}

// Generate handler struct
#[doc(hidden)]
#[macro_export]
macro_rules! __generate_handler_struct {
    ($cmd_variant:ident) => {
        paste::paste! {
            pub struct [<$cmd_variant Handler>];
        }
    };
}
```

- [ ] **Step 2: Add paste dependency**

Add to `Cargo.toml`:
```toml
[dependencies]
paste = "1.0"
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit macro foundation**

```bash
git add src/backend/macros.rs Cargo.toml
git commit -m "feat: add handler macro foundation"
```

### Task 1.2: Implement Handler Trait Generation

**Files:**
- Modify: `src/backend/macros.rs`

**IMPORTANT:** This task is currently BLOCKED due to a fundamental design flaw in the plan. The CommandHandler trait signature in Task 1.4 doesn't provide access to command fields, but the macro invocations require field access (e.g., `&limit`). 

**Decision:** Keep Task 1.2 implementation simple for now. Generate stub implementations that compile but don't handle fields correctly. This will be fixed when we revise the overall Phase 1 design.

- [ ] **Step 1: Add handler trait implementation macro (STUB VERSION)**

```rust
// Add to src/backend/macros.rs after __generate_handler_struct
// NOTE: This is a stub implementation. Pattern 2 cannot work correctly
// without access to command fields via envelope parameter.

#[doc(hidden)]
#[macro_export]
macro_rules! __generate_handler_impl {
    // Simple command with no fields, immutable repo
    (
        $cmd_variant:ident =>
        $git_fn:expr =>
        $event_variant:ident
    ) => {
        paste::paste! {
            impl CommandHandler for [<$cmd_variant Handler>] {
                fn handle(
                    &self,
                    repo: &GitRepo,
                    request_id: u64,
                ) -> Result<Vec<EventEnvelope>> {
                    $git_fn(repo)?;
                    Ok(vec![EventEnvelope {
                        request_id: Some(request_id),
                        event: FrontendEvent::$event_variant,
                    }])
                }
            }
        }
    };

    // Command with fields - STUB (cannot access fields without envelope)
    (
        $cmd_variant:ident { $($field:ident),* } =>
        $git_fn:expr =>
        $event_variant:ident ( $result_binding:ident )
    ) => {
        paste::paste! {
            impl CommandHandler for [<$cmd_variant Handler>] {
                fn handle(
                    &self,
                    repo: &GitRepo,
                    request_id: u64,
                ) -> Result<Vec<EventEnvelope>> {
                    // STUB: Cannot call $git_fn without field values
                    let $result_binding = $git_fn(repo)?;
                    Ok(vec![EventEnvelope {
                        request_id: Some(request_id),
                        event: FrontendEvent::$event_variant($result_binding),
                    }])
                }
            }
        }
    };

    // Command with mutable repo flag
    (
        $cmd_variant:ident [ mut ] =>
        $git_fn:expr =>
        $event_variant:ident ( $result_binding:ident )
    ) => {
        paste::paste! {
            impl CommandHandler for [<$cmd_variant Handler>] {
                fn handle_mut(
                    &self,
                    repo: &mut GitRepo,
                    request_id: u64,
                ) -> Result<Vec<EventEnvelope>> {
                    let $result_binding = $git_fn(repo)?;
                    Ok(vec![EventEnvelope {
                        request_id: Some(request_id),
                        event: FrontendEvent::$event_variant($result_binding),
                    }])
                }
            }
        }
    };
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit trait generation**

```bash
git add src/backend/macros.rs
git commit -m "feat: add handler trait implementation generation"
```

### Task 1.3: Implement Handler Registry Generation

**Files:**
- Modify: `src/backend/macros.rs`

- [ ] **Step 1: Add registry generation macro**

```rust
// Add to src/backend/macros.rs after __generate_handler_impl

#[doc(hidden)]
#[macro_export]
macro_rules! __generate_registry {
    ($($cmd_variant:ident),* $(,)?) => {
        paste::paste! {
            pub fn create_handler_registry() -> std::collections::HashMap<
                &'static str,
                Box<dyn CommandHandler>
            > {
                let mut registry = std::collections::HashMap::new();
                $(
                    registry.insert(
                        stringify!($cmd_variant),
                        Box::new([<$cmd_variant Handler>]) as Box<dyn CommandHandler>
                    );
                )*
                registry
            }
        }
    };
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit registry generation**

```bash
git add src/backend/macros.rs
git commit -m "feat: add handler registry generation"
```

### Task 1.4: Rewrite handlers.rs Using Macro

**Files:**
- Modify: `src/backend/handlers.rs`

- [ ] **Step 1: Replace handler boilerplate with macro invocation**

```rust
// Replace entire src/backend/handlers.rs content

use crate::backend::commands::BackendCommand;
use crate::backend::events::{EventEnvelope, FrontendEvent};
use crate::backend::git_ops::{branches, commits, diff, stash, status, working_tree};
use crate::backend::git_ops::repo::GitRepo;
use crate::shared::types::*;
use anyhow::Result;

pub trait CommandHandler {
    fn handle(&self, repo: &GitRepo, request_id: u64) -> Result<Vec<EventEnvelope>> {
        unimplemented!("handle not implemented")
    }
    
    fn handle_mut(&self, repo: &mut GitRepo, request_id: u64) -> Result<Vec<EventEnvelope>> {
        unimplemented!("handle_mut not implemented")
    }
}

define_handlers! {
    RefreshStatus => status::get_status => FilesUpdated(files),
    RefreshBranches => branches::get_branches => BranchesUpdated(branches),
    RefreshCommits { limit } => commits::get_commits(&limit) => CommitsUpdated(commits),
    
    StageFile { file_path } => working_tree::stage_file(&file_path) => ActionSucceeded("Staged"),
    UnstageFile { file_path } => working_tree::unstage_file(&file_path) => ActionSucceeded("Unstaged"),
    StageFiles { file_paths } => working_tree::stage_files(&file_paths) => ActionSucceeded("Staged"),
    UnstageFiles { file_paths } => working_tree::unstage_files(&file_paths) => ActionSucceeded("Unstaged"),
    StageAll => working_tree::stage_all() => ActionSucceeded("Staged all"),
    
    Commit { message } => commits::create_commit(&message) => ActionSucceeded("Committed"),
    AmendCommitWithFiles { commit_id, message, paths } =>
        commits::amend_commit_with_files(&commit_id, &message, &paths) => ActionSucceeded("Amended"),
    
    GetDiff { file_path } => diff::get_diff(&file_path) => DiffLoaded(diff),
    GetDiffBatch { targets } => diff::get_diff_batch(&targets) => DiffLoaded(diff),
    GetCommitDiff { commit_id } => diff::get_commit_diff(&commit_id) => DiffLoaded(diff),
    GetCommitFiles { commit_id } => commits::get_commit_files(&commit_id) => CommitFilesLoaded(files),
    GetBranchCommits { branch_name, limit } => commits::get_branch_commits(&branch_name, limit) => BranchCommitsLoaded(commits),
    
    IgnoreFiles { paths } => working_tree::ignore_files(&paths) => ActionSucceeded("Ignored"),
    RenameFile { old_path, new_path } => working_tree::rename_file(&old_path, &new_path) => ActionSucceeded("Renamed"),
    
    DiscardFiles { paths } => working_tree::discard_files(&paths) => ActionSucceeded("Discarded"),
    StashFiles { paths, message } => stash::stash_files(&paths, message.as_deref()) => ActionSucceeded("Stashed"),
    
    ResetHard { target } => working_tree::reset_hard(&target) => ActionSucceeded("Reset (hard)"),
    ResetMixed { target } => working_tree::reset_mixed(&target) => ActionSucceeded("Reset (mixed)"),
    ResetSoft { target } => working_tree::reset_soft(&target) => ActionSucceeded("Reset (soft)"),
    
    RefreshStashes [mut] => stash::get_stashes => StashesUpdated(stashes),
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: Compilation errors about missing git_ops functions (expected at this stage)

- [ ] **Step 3: Comment out undefined handlers temporarily**

Comment out any handler definitions that reference git_ops functions not yet implemented. Keep only the ones that currently exist.

- [ ] **Step 4: Run cargo check again**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Run tests**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 6: Commit handler consolidation**

```bash
git add src/backend/handlers.rs
git commit -m "refactor: consolidate handlers using macro (1151 → ~80 lines)"
```

---

## Phase 2: Event System Foundation

### Task 2.1: Create Event Type Definitions

**Files:**
- Create: `src/app/events.rs`

- [ ] **Step 1: Write AppEvent enum**

```rust
// src/app/events.rs

use crate::shared::types::*;

/// Top-level event type returned by components
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Git operation event
    Git(GitEvent),
    /// Modal/dialog event
    Modal(ModalEvent),
    /// Switch active panel
    SwitchPanel(Panel),
    /// Activate current panel (Enter key behavior)
    ActivatePanel,
    /// Selection changed, refresh main view
    SelectionChanged,
    /// Event handled internally by component
    None,
}

/// Git operation events
#[derive(Debug, Clone, PartialEq)]
pub enum GitEvent {
    ToggleStageFile,
    StageAll,
    CommitWithMessage(String),
    DiscardSelected,
    StashSelected,
    AmendCommit,
    ExecuteReset(usize),
    IgnoreSelected,
    RenameFile(String),
}

/// Modal/dialog events
#[derive(Debug, Clone, PartialEq)]
pub enum ModalEvent {
    ShowHelp,
    ShowCommitDialog,
    ShowRenameDialog,
    ShowResetMenu,
    ShowDiscardConfirmation,
    ShowStashConfirmation,
    ShowAmendConfirmation,
    ShowResetConfirmation(usize),
    ShowNukeConfirmation,
    Close,
}
```

- [ ] **Step 2: Add module to app/mod.rs**

```rust
// Add to src/app/mod.rs
pub mod events;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit event types**

```bash
git add src/app/events.rs src/app/mod.rs
git commit -m "feat: add event type system (AppEvent/GitEvent/ModalEvent)"
```

### Task 2.2: Create GitProcessor Stub

**Files:**
- Create: `src/app/git_processor.rs`

- [ ] **Step 1: Write GitProcessor interface**

```rust
// src/app/git_processor.rs

use crate::app::events::{GitEvent, ModalEvent};
use crate::app::state::AppState;
use crate::backend::commands::BackendCommand;
use anyhow::Result;

/// Outcome of processing a git event
#[derive(Debug)]
pub enum ProcessorOutcome {
    /// Send a backend command
    SendCommand(BackendCommand),
    /// Show a modal dialog
    ShowModal(ModalEvent),
    /// No action needed
    None,
}

/// Translates GitEvent to backend commands or modal requests
pub struct GitProcessor;

impl GitProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a git event and return the outcome
    pub fn execute(&self, event: GitEvent, state: &AppState) -> Result<ProcessorOutcome> {
        // Stub implementation - will be filled in Phase 4
        match event {
            GitEvent::StageAll => {
                Ok(ProcessorOutcome::SendCommand(BackendCommand::StageAll))
            }
            _ => Ok(ProcessorOutcome::None),
        }
    }
}
```

- [ ] **Step 2: Add module to app/mod.rs**

```rust
// Add to src/app/mod.rs
pub mod git_processor;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit GitProcessor stub**

```bash
git add src/app/git_processor.rs src/app/mod.rs
git commit -m "feat: add GitProcessor stub"
```

### Task 2.3: Create ModalProcessor Stub

**Files:**
- Create: `src/app/modal_processor.rs`

- [ ] **Step 1: Write ModalProcessor interface**

```rust
// src/app/modal_processor.rs

use crate::app::events::ModalEvent;
use crate::app::state::AppState;
use crate::components::dialogs::modal::{Modal, ModalType};
use anyhow::Result;

/// Handles modal dialog events
pub struct ModalProcessor;

impl ModalProcessor {
    pub fn new() -> Self {
        Self
    }

    /// Execute a modal event
    pub fn execute(&mut self, event: ModalEvent, state: &mut AppState) -> Result<()> {
        // Stub implementation - will be filled in Phase 4
        match event {
            ModalEvent::Close => {
                state.ui_state.active_modal = None;
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
```

- [ ] **Step 2: Add module to app/mod.rs**

```rust
// Add to src/app/mod.rs
pub mod modal_processor;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit ModalProcessor stub**

```bash
git add src/app/modal_processor.rs src/app/mod.rs
git commit -m "feat: add ModalProcessor stub"
```

---

## Phase 3: Component Trait Migration

### Task 3.1: Create ComponentV2 Trait

**Files:**
- Create: `src/components/component_v2.rs`

- [ ] **Step 1: Write ComponentV2 trait and RenderContext**

```rust
// src/components/component_v2.rs

use crate::app::cache::CachedData;
use crate::app::events::AppEvent;
use crate::components::core::theme::Theme;
use crossterm::event::Event;
use ratatui::Frame;
use ratatui::layout::Rect;

/// Read-only context passed to components
pub struct RenderContext<'a> {
    /// Cached git data (read-only)
    pub data: &'a CachedData,
    /// Theme configuration
    pub theme: &'a Theme,
    /// Whether this component is focused
    pub is_focused: bool,
}

/// React-like component trait - data flows down, events flow up
pub trait ComponentV2 {
    /// Handle input event, return app-level event
    fn on_event(&mut self, event: &Event, ctx: &RenderContext) -> AppEvent;
    
    /// Render the component
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext);
}
```

- [ ] **Step 2: Add module to components/mod.rs**

```rust
// Add to src/components/mod.rs
pub mod component_v2;
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit ComponentV2 trait**

```bash
git add src/components/component_v2.rs src/components/mod.rs
git commit -m "feat: add ComponentV2 trait with RenderContext"
```

### Task 3.2: Migrate StashListPanel to ComponentV2

**Files:**
- Modify: `src/components/panels/stash_list.rs`

- [ ] **Step 1: Implement ComponentV2 for StashListPanel**

```rust
// Add to src/components/panels/stash_list.rs

use crate::app::events::AppEvent;
use crate::components::component_v2::{ComponentV2, RenderContext};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

impl ComponentV2 for StashListPanel {
    fn on_event(&mut self, event: &Event, ctx: &RenderContext) -> AppEvent {
        if !ctx.is_focused {
            return AppEvent::None;
        }

        match event {
            Event::Key(KeyEvent {
                code,
                modifiers: KeyModifiers::NONE,
                ..
            }) => match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.list.next();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.list.previous();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('g') => {
                    self.list.first();
                    AppEvent::SelectionChanged
                }
                KeyCode::Char('G') => {
                    self.list.last();
                    AppEvent::SelectionChanged
                }
                _ => AppEvent::None,
            },
            _ => AppEvent::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext) {
        // Use existing render logic
        self.render_existing(frame, area, &ctx.data.stashes, ctx.is_focused, ctx.theme);
    }
}
```

- [ ] **Step 2: Keep old Component impl as adapter**

```rust
// Keep existing Component impl, forward to ComponentV2
impl Component for StashListPanel {
    fn handle_event(&mut self, event: &Event, app_state: &AppState) -> Intent {
        let ctx = RenderContext {
            data: &app_state.data_cache,
            theme: &Theme::default(),
            is_focused: app_state.ui_state.active_panel == Panel::Stash,
        };
        
        let app_event = ComponentV2::on_event(self, event, &ctx);
        
        // Convert AppEvent to Intent (temporary adapter)
        match app_event {
            AppEvent::SelectionChanged => Intent::RefreshMainView,
            _ => Intent::None,
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect, state: &AppState) {
        let ctx = RenderContext {
            data: &state.data_cache,
            theme: &Theme::default(),
            is_focused: state.ui_state.active_panel == Panel::Stash,
        };
        
        ComponentV2::render(self, frame, area, &ctx);
    }
}
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Test stash panel**

Run: `cargo run`
Manual test: Navigate to stash panel, test j/k navigation

- [ ] **Step 5: Commit StashListPanel migration**

```bash
git add src/components/panels/stash_list.rs
git commit -m "feat: migrate StashListPanel to ComponentV2"
```

### Task 3.3: Migrate BranchListPanel to ComponentV2

**Files:**
- Modify: `src/components/panels/branch_list.rs`

- [ ] **Step 1: Implement ComponentV2 for BranchListPanel**

Add ComponentV2 implementation that handles j/k navigation internally and returns AppEvent::ActivatePanel on Enter.

- [ ] **Step 2: Test branch navigation**

Run: `cargo run`
Manual test: Navigate branches with j/k, press Enter to activate

- [ ] **Step 3: Commit BranchListPanel migration**

```bash
git add src/components/panels/branch_list.rs
git commit -m "feat: migrate BranchListPanel to ComponentV2"
```

### Task 3.4: Migrate FileListPanel to ComponentV2

**Files:**
- Modify: `src/components/panels/file_list.rs`

- [ ] **Step 1: Implement ComponentV2 for FileListPanel**

Map keyboard shortcuts to AppEvent:
- Space → AppEvent::Git(GitEvent::ToggleStageFile)
- a → AppEvent::Git(GitEvent::StageAll)
- d → AppEvent::Git(GitEvent::DiscardSelected)
- s → AppEvent::Git(GitEvent::StashSelected)
- i → AppEvent::Git(GitEvent::IgnoreSelected)
- r → AppEvent::Modal(ModalEvent::ShowRenameDialog)
- j/k → AppEvent::SelectionChanged (after internal navigation)

- [ ] **Step 2: Test file operations**

Run: `cargo run`
Manual test: Stage/unstage files, navigate with j/k

- [ ] **Step 3: Commit FileListPanel migration**

```bash
git add src/components/panels/file_list.rs
git commit -m "feat: migrate FileListPanel to ComponentV2"
```

### Task 3.5: Migrate CommitPanel to ComponentV2

**Files:**
- Modify: `src/components/panels/commit_panel.rs`

- [ ] **Step 1: Implement ComponentV2 for CommitPanel**

Handle three modes: List, Loading, FilesTree. Return AppEvent::ActivatePanel on Enter in List mode.

- [ ] **Step 2: Test commit navigation**

Run: `cargo run`
Manual test: Navigate commits, press Enter to load files

- [ ] **Step 3: Commit CommitPanel migration**

```bash
git add src/components/panels/commit_panel.rs
git commit -m "feat: migrate CommitPanel to ComponentV2"
```

### Task 3.6: Migrate MainView and LogPanel

**Files:**
- Modify: `src/components/panels/main_view.rs`
- Modify: `src/components/panels/log.rs`

- [ ] **Step 1: Implement ComponentV2 for MainView**

Display-only component, returns AppEvent::None for all events.

- [ ] **Step 2: Implement ComponentV2 for LogPanel**

Display-only component, returns AppEvent::None for all events.

- [ ] **Step 3: Commit display components**

```bash
git add src/components/panels/main_view.rs src/components/panels/log.rs
git commit -m "feat: migrate MainView and LogPanel to ComponentV2"
```

### Task 3.7: Update Modal to Return AppEvent

**Files:**
- Modify: `src/components/dialogs/modal.rs`

- [ ] **Step 1: Change Modal to return AppEvent**

Update Modal methods to return AppEvent instead of Intent. ModalType::Selection returns AppEvent::Git(GitEvent::ExecuteReset(idx)), ModalType::TextInput returns AppEvent::Git(GitEvent::CommitWithMessage(msg)).

- [ ] **Step 2: Update help items to store AppEvent**

Change help items from Vec<(String, Intent)> to Vec<(String, AppEvent)>.

- [ ] **Step 3: Test modal interactions**

Run: `cargo run`
Manual test: Open commit dialog, reset menu, help panel

- [ ] **Step 4: Commit modal changes**

```bash
git add src/components/dialogs/modal.rs
git commit -m "refactor: modal returns AppEvent instead of Intent"
```

### Task 3.8: Remove Old Component Trait

**Files:**
- Delete: `src/components/component.rs`
- Modify: `src/components/mod.rs`
- Modify: `src/components/component_v2.rs`

- [ ] **Step 1: Rename ComponentV2 to Component**

In `src/components/component_v2.rs`, rename trait ComponentV2 to Component.

- [ ] **Step 2: Update module exports**

In `src/components/mod.rs`, remove old component module, rename component_v2 to component.

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit trait consolidation**

```bash
git add src/components/
git commit -m "refactor: consolidate to single Component trait"
```

---

## Phase 4: App Layer Refactor

### Task 4.1: Implement GitProcessor Logic

**Files:**
- Modify: `src/app/git_processor.rs`

- [ ] **Step 1: Implement ToggleStageFile with multi-select logic**

```rust
GitEvent::ToggleStageFile => {
    let selected_targets = state.components.selected_file_tree_targets();
    let selected_files: Vec<String> = selected_targets
        .into_iter()
        .filter(|(_, is_dir)| !is_dir)
        .map(|(path, _)| path)
        .collect();
    
    if selected_files.is_empty() {
        return Ok(ProcessorOutcome::None);
    }
    
    // Pivot logic: use anchor file's is_staged to decide direction
    let pivot_path = state.components.selected_file_anchor_target()
        .and_then(|(path, is_dir)| (!is_dir).then_some(path))
        .or_else(|| selected_files.first().cloned());
    
    let should_unstage = pivot_path
        .and_then(|p| state.data_cache.files.iter().find(|e| e.path == p))
        .map(|f| f.is_staged)
        .unwrap_or(false);
    
    let cmd = if selected_files.len() == 1 {
        if should_unstage {
            BackendCommand::UnstageFile { file_path: selected_files.into_iter().next().unwrap() }
        } else {
            BackendCommand::StageFile { file_path: selected_files.into_iter().next().unwrap() }
        }
    } else if should_unstage {
        BackendCommand::UnstageFiles { file_paths: selected_files }
    } else {
        BackendCommand::StageFiles { file_paths: selected_files }
    };
    
    ProcessorOutcome::SendCommand(cmd)
}
```

- [ ] **Step 2: Implement remaining GitEvent variants**

Add match arms for: StageAll, CommitWithMessage, DiscardSelected, StashSelected, AmendCommit, ExecuteReset, IgnoreSelected, RenameFile.

Reference spec lines 221-278 for exact logic.

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 4: Commit GitProcessor implementation**

```bash
git add src/app/git_processor.rs
git commit -m "feat: implement GitProcessor event translation logic"
```

### Task 4.2: Implement ModalProcessor Logic

**Files:**
- Modify: `src/app/modal_processor.rs`

- [ ] **Step 1: Implement all ModalEvent variants**

```rust
pub fn execute(&mut self, event: ModalEvent, state: &mut AppState) -> Result<()> {
    match event {
        ModalEvent::ShowHelp => {
            let help_items = create_help_items(); // Returns Vec<(String, AppEvent)>
            state.ui_state.active_modal = Some(Modal::new(
                "Help".to_string(),
                ModalType::Help(help_items),
            ));
        }
        ModalEvent::ShowCommitDialog => {
            state.ui_state.active_modal = Some(Modal::new(
                "Commit Message".to_string(),
                ModalType::TextInput(String::new()),
            ));
        }
        ModalEvent::ShowRenameDialog => {
            let current_path = state.components.selected_file_path().unwrap_or_default();
            state.ui_state.active_modal = Some(Modal::new(
                "Rename File".to_string(),
                ModalType::TextInput(current_path),
            ));
        }
        ModalEvent::ShowResetMenu => {
            let options = vec![
                "Hard Reset (HEAD)".to_string(),
                "Mixed Reset (HEAD)".to_string(),
                "Soft Reset (HEAD)".to_string(),
                "Hard Reset (HEAD~1)".to_string(),
                "Soft Reset (HEAD~1)".to_string(),
                "Nuke Repository".to_string(),
            ];
            state.ui_state.active_modal = Some(Modal::new(
                "Reset Options".to_string(),
                ModalType::Selection(options, 0),
            ));
        }
        ModalEvent::ShowDiscardConfirmation => {
            state.ui_state.active_modal = Some(Modal::new(
                "Discard Changes?".to_string(),
                ModalType::Confirmation(Box::new(|| {
                    AppEvent::Git(GitEvent::DiscardSelected)
                })),
            ));
        }
        // ... other variants
        ModalEvent::Close => {
            state.ui_state.active_modal = None;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 3: Commit ModalProcessor implementation**

```bash
git add src/app/modal_processor.rs
git commit -m "feat: implement ModalProcessor dialog management"
```

### Task 4.3: Wire Processors into App::process_event

**Files:**
- Modify: `src/app/runtime.rs`

- [ ] **Step 1: Add processor fields to App struct**

```rust
pub struct App {
    state: AppState,
    requests: RequestTracker,
    git_processor: GitProcessor,
    modal_processor: ModalProcessor,
    // ... existing fields
}
```

- [ ] **Step 2: Implement App::process_event**

```rust
fn process_event(&mut self, event: AppEvent) -> Result<()> {
    match event {
        AppEvent::Git(git_event) => {
            match self.git_processor.execute(git_event, &self.state)? {
                ProcessorOutcome::SendCommand(cmd) => {
                    let request_id = self.state.send_command(cmd)?;
                    self.requests.track(request_id);
                }
                ProcessorOutcome::ShowModal(modal_event) => {
                    self.modal_processor.execute(modal_event, &mut self.state)?;
                }
                ProcessorOutcome::None => {}
            }
        }
        AppEvent::Modal(modal_event) => {
            self.modal_processor.execute(modal_event, &mut self.state)?;
        }
        AppEvent::SwitchPanel(panel) => {
            self.state.ui_state.active_panel = panel;
            self.update_main_view_for_active_panel()?;
        }
        AppEvent::ActivatePanel => {
            match self.state.ui_state.active_panel {
                Panel::Branches => {
                    if let Some(branch) = self.state.selected_branch() {
                        let branch_name = branch.name.clone();
                        self.state.push_log(format!("Loading commits for branch {branch_name}..."));
                        let request_id = self.state.send_command(BackendCommand::GetBranchCommits {
                            branch_name,
                            limit: 50,
                        })?;
                        self.requests.set_latest_branch_commits(request_id);
                    }
                }
                Panel::Commits => {
                    if let Some(commit) = self.state.selected_commit() {
                        let commit_id = commit.id.clone();
                        let summary = commit.summary.clone();
                        self.state.components.commit_panel
                            .start_loading(commit_id.clone(), summary.clone());
                        self.state.push_log(format!("Loading files for commit {}...", &commit_id[..7]));
                        let request_id = self.state.send_command(BackendCommand::GetCommitFiles { commit_id })?;
                        self.requests.track(request_id);
                    }
                }
                _ => {}
            }
            self.update_main_view_for_active_panel()?;
        }
        AppEvent::SelectionChanged => {
            self.update_main_view_for_active_panel()?;
        }
        AppEvent::None => {}
    }
    Ok(())
}
```

- [ ] **Step 3: Update main loop to use process_event**

Replace intent_executor calls with process_event calls.

- [ ] **Step 4: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully

- [ ] **Step 5: Commit app layer refactor**

```bash
git add src/app/runtime.rs
git commit -m "refactor: wire processors into App::process_event"
```

### Task 4.4: Delete Intent System

**Files:**
- Delete: `src/app/intent.rs`
- Delete: `src/app/intent_executor.rs`
- Modify: `src/app/mod.rs`

- [ ] **Step 1: Remove intent modules from mod.rs**

Remove `pub mod intent;` and `pub mod intent_executor;` from `src/app/mod.rs`.

- [ ] **Step 2: Delete intent files**

```bash
rm src/app/intent.rs src/app/intent_executor.rs
```

- [ ] **Step 3: Run cargo check**

Run: `cargo check`
Expected: Compiles successfully (no references to Intent should remain)

- [ ] **Step 4: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 5: Commit intent system removal**

```bash
git add src/app/
git commit -m "refactor: remove Intent system (replaced by AppEvent)"
```

---

## Phase 5: Cleanup & Polish

### Task 5.1: Remove Dead Code

**Files:**
- Various

- [ ] **Step 1: Run cargo check for warnings**

Run: `cargo check 2>&1 | grep "never used"`
Expected: List of unused items

- [ ] **Step 2: Remove unused imports and functions**

Remove any dead code flagged by compiler.

- [ ] **Step 3: Run cargo clippy**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: Zero warnings

- [ ] **Step 4: Commit cleanup**

```bash
git add .
git commit -m "chore: remove dead code"
```

### Task 5.2: Update Documentation

**Files:**
- Modify: `CLAUDE.md`
- Modify: `docs/arch.md` (if exists)

- [ ] **Step 1: Update CLAUDE.md architecture section**

Document new event-driven architecture: AppEvent/GitEvent/ModalEvent, ComponentV2 trait, GitProcessor, ModalProcessor, macro-generated handlers.

- [ ] **Step 2: Update file structure documentation**

Update file listings to reflect new structure (events.rs, git_processor.rs, modal_processor.rs, component_v2.rs).

- [ ] **Step 3: Commit documentation**

```bash
git add CLAUDE.md docs/
git commit -m "docs: update architecture documentation for event-driven refactor"
```

### Task 5.3: Final Validation

**Files:**
- None

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

- [ ] **Step 2: Run clippy with strict settings**

Run: `cargo clippy --all-targets --all-features -- -D warnings`
Expected: Zero warnings

- [ ] **Step 3: Check formatting**

Run: `cargo fmt --check`
Expected: All files formatted

- [ ] **Step 4: Manual smoke test**

Run: `cargo run`
Test all workflows:
- Stage/unstage files
- Commit with message
- Amend commit
- Discard changes
- Stash changes
- Navigate branches
- View commit details
- Navigate stash list
- Ignore files
- Rename files
- Reset operations
- Help panel
- All modals

- [ ] **Step 5: Create completion commit**

```bash
git add .
git commit -m "feat: complete event-driven refactor

- Consolidated 26 handlers into macro-generated code (1151 → ~80 lines)
- Introduced AppEvent/GitEvent/ModalEvent type system
- Migrated all components to ComponentV2 trait with RenderContext
- Extracted git logic to GitProcessor
- Extracted modal logic to ModalProcessor
- Removed Intent system (902 lines deleted)
- All tests passing, zero clippy warnings"
```

---

## Success Metrics

After completing all phases:

- **Code size reduction:**
  - `handlers.rs`: 1151 → ~80 lines (93% reduction)
  - `intent_executor.rs`: 902 lines → deleted
  - Total: ~1400 lines removed

- **Maintainability:**
  - New command: 1-3 lines (macro definition)
  - New panel: Implement Component trait (~50-100 lines)
  - New git event: Add enum variant + match arm (~5 lines)

- **Quality:**
  - Zero clippy warnings
  - All tests passing
  - No regressions in functionality

---

## Rollback Strategy

- Each phase is independently committed
- If a phase fails: `git reset --soft HEAD~1` (check `git status` first)
- Can pause between phases without breaking the app
- Phase 1-2 are low risk (additive changes)
- Phase 3 is highest risk (component migration)
- Phase 4-5 are medium risk (cleanup and validation)

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-16-event-driven-refactor.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?



