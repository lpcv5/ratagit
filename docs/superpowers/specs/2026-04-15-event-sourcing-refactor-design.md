# Event Sourcing Architecture Refactor

**Date:** 2026-04-15  
**Status:** Design Approved  
**Goal:** Refactor ratagit from patch-style additions to a clean, React-like event-driven architecture with trait-based component reuse

## Context

Since the last major refactor, ratagit has grown from basic file/branch/commit panels to include:
- Commit operations with text input modals
- File operations (ignore, rename)
- Amend functionality (including non-HEAD commits)
- Help panel and modal system

This growth exposed architectural issues:
- `backend/handlers.rs`: 1151 lines with 26 nearly-identical handler structs
- `app/intent_executor.rs`: 902 lines split across TWO `impl App` blocks
- `backend/git_ops/working_tree.rs`: 625 lines mixing multiple concerns
- Components tightly coupled to App — can't handle their own navigation
- Adding new features requires touching 5+ files with repetitive boilerplate

The codebase has become difficult to navigate, slow to extend, and hard to test in isolation.

## Design Goals

1. **Cleaner code** — eliminate boilerplate, single source of truth for command/event mappings
2. **Better testability** — components testable in isolation, clear boundaries
3. **Easier feature addition** — new commands/events require minimal boilerplate
4. **React-like component model** — data flows down, events flow up, components own local state
5. **Future-proof** — foundation for undo/redo (via git reflog), component composition, dynamic panels

## Architecture Overview

### Event System (replaces Intent)

**Current problem:** `Intent` is a flat enum mixing navigation, UI state, and git operations. Everything funnels into one 900-line executor.

**New design:** Split into typed event categories:

```rust
// src/app/events.rs
pub enum AppEvent {
    Git(GitEvent),
    Modal(ModalEvent),
    SwitchPanel(Panel),
    None,  // component handled internally
}

pub enum GitEvent {
    ToggleStageFile,
    StageAll,
    CommitWithMessage(String),
    Discard,
    Stash,
    AmendCommit,
    ExecuteReset(usize),
    IgnoreSelected,
    RenameFile(String),
    // Branch operations (added during Phase 3 panel migration)
    CheckoutBranch(String),
    DeleteBranch(String),
    ApplyStash,
    PopStash,
    DropStash,
}

pub enum ModalEvent {
    ShowHelp,
    ShowCommitDialog,
    ShowRenameDialog,
    ShowResetMenu,
    Close,
}
```

**Key principle:** Components handle their own navigation (j/k/scroll) and return `AppEvent::None`. Only app-level coordination (git ops, panel switching, modals) bubbles up.

### React-Like Component Model

**Current problem:** Components can't handle their own state. Everything goes through App.

**New design:** Components own local state, receive read-only context:

```rust
// src/components/component.rs
pub struct RenderContext<'a> {
    pub data: &'a CachedData,   // git data (read-only)
    pub theme: &'a Theme,
    pub is_focused: bool,
}

pub trait Component {
    fn on_event(&mut self, event: &Event, ctx: &RenderContext) -> AppEvent;
    fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext);
}
```

**Data flow:**
- **Down:** App passes `RenderContext` (props) to components
- **Up:** Components return `AppEvent` for app-level coordination
- **Local:** Components own `ListState`, scroll, multi-select, expand/collapse

**Component composition:**
```rust
pub struct BranchListPanel {
    list: SelectableList<BranchInfo>,
    sub_panel: Option<CommitPanel>,  // child component
}

impl Component for BranchListPanel {
    fn on_event(&mut self, event: &Event, ctx: &RenderContext) -> AppEvent {
        // Delegate to child first (React-like bubbling)
        if let Some(sub) = &mut self.sub_panel {
            let child_event = sub.on_event(event, ctx);
            if child_event != AppEvent::None {
                return child_event;
            }
        }
        // Handle own events
        self.handle_own_event(event, ctx)
    }
}
```

### Trait-Based Component Reuse

**Pattern:** Generic behavior traits for common panel patterns:

```rust
// src/components/core/list_behavior.rs
pub trait ListBehavior {
    type Item;
    
    fn on_activate(&self, item: &Self::Item) -> AppEvent;
    fn on_delete(&self, item: &Self::Item) -> AppEvent { AppEvent::None }
    fn on_custom_key(&self, key: KeyCode, item: &Self::Item) -> AppEvent { AppEvent::None }
}

pub struct ConfigurableList<T, B: ListBehavior<Item = T>> {
    list: SelectableList<T>,
    behavior: B,
}

// Usage
struct BranchBehavior;
impl ListBehavior for BranchBehavior {
    type Item = BranchInfo;
    
    fn on_activate(&self, item: &BranchInfo) -> AppEvent {
        AppEvent::Git(GitEvent::CheckoutBranch(item.name.clone()))
    }
    
    fn on_delete(&self, item: &BranchInfo) -> AppEvent {
        AppEvent::Git(GitEvent::DeleteBranch(item.name.clone()))
    }
}

pub type BranchListPanel = ConfigurableList<BranchInfo, BranchBehavior>;
```

**Benefits:**
- Stash/Branch/Commit panels share 90% of code
- Custom behavior via trait implementation
- Easy to add new panel types

### GitProcessor (replaces intent_executor.rs)

**Current problem:** 902-line `intent_executor.rs` with 33 methods split across two `impl App` blocks.

**New design:** Focused `GitProcessor` owns git event → backend command translation:

```rust
// src/app/git_processor.rs
pub struct GitProcessor {
    cmd_tx: Sender<CommandEnvelope>,
    request_tracker: RequestTracker,
}

impl GitProcessor {
    pub fn execute(&mut self, event: GitEvent, state: &mut AppState) -> Result<()> {
        let cmd = match event {
            GitEvent::ToggleStageFile => {
                let path = state.get_selected_file_path()?;
                if state.cache.is_file_staged(&path) {
                    BackendCommand::UnstageFile { file_path: path }
                } else {
                    BackendCommand::StageFile { file_path: path }
                }
            }
            GitEvent::StageAll => BackendCommand::StageAll,
            GitEvent::CommitWithMessage(msg) => BackendCommand::Commit { message: msg },
            // ... other events
        };
        
        let request_id = state.send_command(cmd)?;
        self.request_tracker.track(request_id);
        Ok(())
    }
}
```

**App's main loop becomes trivial:**

```rust
// src/app/runtime.rs
impl App {
    pub async fn run(&mut self) -> Result<()> {
        loop {
            self.drain_backend_events();
            self.render()?;
            
            if let Some(event) = self.poll_input()? {
                let app_event = self.route_to_component(event);
                self.process_event(app_event)?;
            }
        }
    }
    
    fn process_event(&mut self, event: AppEvent) -> Result<()> {
        match event {
            AppEvent::Git(git_event) => {
                self.git_processor.execute(git_event, &mut self.state)?;
            }
            AppEvent::Modal(modal_event) => {
                self.modal_processor.execute(modal_event, &mut self.state)?;
            }
            AppEvent::SwitchPanel(panel) => {
                self.state.ui.set_active_panel(panel);
            }
            AppEvent::None => {}
        }
        Ok(())
    }
}
```

### Backend Handler Consolidation

**Current problem:** 26 nearly-identical handler structs (1151 lines of boilerplate).

**New design:** Macro-generated handlers from declarative definitions:

```rust
// src/backend/handlers.rs
use crate::define_handlers;

define_handlers! {
    // Syntax: CommandVariant => git_ops_function => SuccessEvent
    
    RefreshStatus => status::get_status => FilesUpdated(files),
    RefreshBranches => branches::get_branches => BranchesUpdated(branches),
    RefreshCommits { limit } => commits::get_commits(limit) => CommitsUpdated(commits),
    
    StageFile { file_path } => working_tree::stage_file(&file_path) => ActionSucceeded("Staged"),
    UnstageFile { file_path } => working_tree::unstage_file(&file_path) => ActionSucceeded("Unstaged"),
    
    Commit { message } => commits::create_commit(&message) => ActionSucceeded("Committed"),
    AmendCommitWithFiles { commit_id, message, paths } => 
        commits::amend_commit_with_files(&commit_id, &message, &paths) => ActionSucceeded("Amended"),
    
    GetDiff { file_path } => diff::get_diff(&file_path) => DiffLoaded(diff),
    GetDiffBatch { targets } => diff::get_diff_batch(&targets) => DiffLoaded(diff),
    
    IgnoreFiles { paths } => working_tree::ignore_files(&paths) => ActionSucceeded("Ignored"),
    RenameFile { old_path, new_path } => working_tree::rename_file(&old_path, &new_path) => ActionSucceeded("Renamed"),
    
    // Special case: needs mutable repo
    RefreshStashes [mut] => stash::get_stashes => StashesUpdated(stashes),
}
```

**Macro implementation** (in `src/backend/macros.rs`):

The `define_handlers!` macro generates:
1. Handler structs for each command
2. `CommandHandler` trait implementations
3. Handler registry initialization
4. Error handling boilerplate

**Result:**
- 1151 lines → ~200 lines
- Adding new command: 1 line instead of 40+ lines
- Single source of truth for command → git_ops → event mapping

## File Structure

```
src/
├── main.rs
├── shared/
│   ├── mod.rs
│   └── path_utils.rs          (unchanged)
│
├── backend/
│   ├── mod.rs
│   ├── commands.rs            (unchanged)
│   ├── events.rs              (unchanged)
│   ├── runtime.rs             (unchanged)
│   ├── macros.rs              (NEW — define_handlers! macro)
│   ├── handlers.rs            (1151 → ~200 lines, macro-driven)
│   └── git_ops/               (unchanged — already clean)
│       ├── mod.rs
│       ├── repo.rs
│       ├── status.rs
│       ├── branches.rs
│       ├── commits.rs
│       ├── working_tree.rs
│       ├── diff.rs
│       ├── stash.rs
│       └── reflog.rs          (NEW — for future undo feature)
│
├── app/
│   ├── mod.rs
│   ├── runtime.rs             (simplified main loop)
│   ├── state.rs               (unchanged)
│   ├── cache.rs               (unchanged)
│   ├── ui_state.rs            (unchanged)
│   ├── request_tracker.rs     (unchanged)
│   ├── renderer.rs            (unchanged)
│   ├── input_handler.rs       (unchanged)
│   ├── keyhints.rs            (unchanged)
│   ├── events.rs              (NEW — replaces intent.rs)
│   ├── git_processor.rs       (NEW — handles GitEvent)
│   ├── modal_processor.rs     (NEW — handles ModalEvent)
│   └── intent_executor.rs     (DELETED after migration)
│
└── components/
    ├── mod.rs
    ├── component.rs           (UPDATED — new trait with RenderContext)
    ├── core/
    │   ├── mod.rs
    │   ├── selectable_list.rs (unchanged)
    │   ├── tree_component.rs  (unchanged)
    │   ├── multi_select.rs    (unchanged)
    │   ├── simple_list.rs     (unchanged)
    │   ├── theme.rs           (unchanged)
    │   └── list_behavior.rs   (NEW — trait for reusable list patterns)
    ├── panels/                (UPDATED — all panels migrate to new trait)
    │   ├── file_list.rs
    │   ├── branch_list.rs
    │   ├── commit_panel.rs
    │   ├── stash_list.rs
    │   ├── main_view.rs
    │   └── log.rs
    └── dialogs/
        └── modal.rs           (unchanged)
```

## Testing Strategy

### Backend Tests (unchanged)
```rust
#[test]
fn test_stage_file() {
    let (temp, repo) = create_test_repo();
    working_tree::stage_file(&repo, "test.txt").unwrap();
    assert!(is_staged(&repo, "test.txt"));
}
```

### Component Tests (much simpler)
```rust
#[test]
fn test_branch_selection_returns_checkout_event() {
    let mut panel = BranchListPanel::new();
    let ctx = mock_render_context();
    
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = panel.on_event(&event, &ctx);
    
    assert!(matches!(result, AppEvent::Git(GitEvent::CheckoutBranch(_))));
}

#[test]
fn test_navigation_handled_internally() {
    let mut panel = BranchListPanel::new();
    let ctx = mock_render_context();
    
    // j/k keys should return None (handled internally)
    let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(panel.on_event(&event, &ctx), AppEvent::None);
}
```

### GitProcessor Tests (new)
```rust
#[test]
fn test_stage_file_sends_correct_command() {
    let mut processor = GitProcessor::new(mock_cmd_tx());
    let mut state = mock_app_state();
    
    processor.execute(GitEvent::ToggleStageFile, &mut state).unwrap();
    
    let sent_cmd = state.last_sent_command();
    assert!(matches!(sent_cmd, BackendCommand::StageFile { .. }));
}
```

### Integration Tests (simplified)
```rust
#[tokio::test]
async fn test_stage_commit_workflow() {
    let app = TestApp::new();
    
    app.send_event(AppEvent::Git(GitEvent::StageAll)).await;
    app.wait_for_backend_response().await;
    
    app.send_event(AppEvent::Git(GitEvent::CommitWithMessage("test".into()))).await;
    app.wait_for_backend_response().await;
    
    assert_eq!(app.state().cache.commits.len(), 1);
}
```

## Migration Strategy

### Phase 1: Backend Cleanup (Low Risk)
**Goal:** Eliminate handler boilerplate without changing behavior

**Steps:**
1. Create `src/backend/macros.rs` with `define_handlers!` macro
2. Rewrite `src/backend/handlers.rs` using the macro
3. Update `src/backend/runtime.rs` to use generated registry

**Validation:**
```bash
cargo test --lib backend
cargo clippy -- -D warnings
```

**Expected result:** All backend tests pass, no functional changes

**Rollback:** `git reset --hard` if tests fail

---

### Phase 2: Event System Foundation (Medium Risk)
**Goal:** Introduce new event types alongside existing Intent system

**Steps:**
1. Create `src/app/events.rs` with `AppEvent`, `GitEvent`, `ModalEvent` enums
2. Keep `src/app/intent.rs` alongside it (both coexist temporarily)
3. Create `src/app/git_processor.rs` (empty struct, just the interface)
4. Create `src/app/modal_processor.rs` (empty struct, just the interface)

**Validation:**
```bash
cargo check
cargo test
```

**Expected result:** Code compiles, no behavior changes (new code not used yet)

**Rollback:** Delete new files if compilation fails

---

### Phase 3: Component Trait Migration (High Risk)
**Goal:** Migrate components one at a time to new trait

**Steps:**

**3.1: Update Component trait**
1. Update `src/components/component.rs`:
   - Add `RenderContext` struct
   - Change `handle_event` → `on_event`
   - Update signature to use `RenderContext`
2. Create `src/components/core/list_behavior.rs` trait

**Validation:**
```bash
cargo check  # Will fail — components not updated yet
```

**3.2: Migrate StashListPanel (simplest)**
1. Update `StashListPanel` to new trait
2. Move j/k navigation into component
3. Return `AppEvent` instead of `Intent`
4. Update `App` to handle both old and new event types temporarily

**Validation:**
```bash
cargo test --test stash_panel_tests
cargo run  # Manual test: navigate stash panel, apply/pop/drop
```

**Expected result:** Stash panel works identically to before

**3.3: Migrate BranchListPanel (has sub-panel)**
1. Update `BranchListPanel` to new trait
2. Handle sub-panel delegation
3. Test branch checkout, delete, sub-panel navigation

**Validation:**
```bash
cargo test --test branch_panel_tests
cargo run  # Manual test: checkout branch, delete branch, view commits
```

**3.4: Migrate CommitPanel**
1. Update `CommitPanel` to new trait
2. Handle three modes (List, Loading, FilesTree)
3. Test commit selection, file tree navigation

**Validation:**
```bash
cargo test --test commit_panel_tests
cargo run  # Manual test: navigate commits, view files, view diffs
```

**3.5: Migrate FileListPanel (most complex)**
1. Update `FileListPanel` to new trait
2. Handle tree navigation, multi-select
3. Test stage/unstage, discard, ignore, rename

**Validation:**
```bash
cargo test --test file_panel_tests
cargo run  # Manual test: stage files, commit, discard, rename, ignore
```

**Rollback per panel:** If a panel migration fails, revert that panel's changes and continue with others

---

### Phase 4: App Layer Refactor (Medium Risk)
**Goal:** Move git logic to GitProcessor, slim down intent_executor

**Steps:**
1. Implement `GitProcessor::execute()` with all git event handling
2. Implement `ModalProcessor::execute()` with all modal handling
3. Update `App::process_event()` to route by event type
4. Remove git/modal logic from `intent_executor.rs`
5. Delete `src/app/intent.rs` (no longer used)
6. Delete `src/app/intent_executor.rs` (replaced by processors)

**Validation:**
```bash
cargo test
cargo clippy -- -D warnings
cargo run  # Full manual testing of all workflows
```

**Test checklist:**
- [ ] Stage/unstage files
- [ ] Commit with message
- [ ] Amend commit
- [ ] Discard changes
- [ ] Stash changes
- [ ] Checkout branch
- [ ] Delete branch
- [ ] View commit details
- [ ] Apply/pop/drop stash
- [ ] Ignore files
- [ ] Rename files
- [ ] Reset (hard/mixed/soft)
- [ ] Help panel
- [ ] All modals (commit, rename, reset)

**Rollback:** `git reset --hard` to Phase 3 completion if critical bugs found

---

### Phase 5: Cleanup & Polish (Low Risk)
**Goal:** Remove dead code, update docs, ensure quality

**Steps:**
1. Remove any dead code flagged by compiler
2. Update `CLAUDE.md` with new architecture
3. Update `docs/arch.md` if it exists
4. Run full test suite
5. Run clippy with strict settings
6. Format all code

**Validation:**
```bash
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

**Expected result:**
- All tests pass
- Zero clippy warnings
- Code formatted
- Documentation updated

---

## Phase Completion Criteria

Each phase is considered complete when:
1. All validation commands pass
2. Manual testing (where applicable) confirms expected behavior
3. Changes are committed to git with descriptive message
4. No regressions in existing functionality

## Rollback Strategy

- Commit after each phase completes
- If a phase fails, `git reset --hard` to previous commit
- Each phase is independently testable
- Can pause between phases without breaking the app

## Estimated Timeline

- Phase 1: 2-3 hours
- Phase 2: 1-2 hours
- Phase 3: 6-8 hours (most time-consuming, per-panel migration)
- Phase 4: 3-4 hours
- Phase 5: 1-2 hours

**Total: 15-20 hours of focused work**

## Future Enhancements (Not in Scope)

These are enabled by the new architecture but not part of this refactor:

1. **Undo/Redo via git reflog**
   - Add `backend/git_ops/reflog.rs`
   - Add `UndoToReflog` backend command
   - Add `u`/`Ctrl+r` keybindings
   - Show reflog panel

2. **Component Registry**
   - Dynamic panel loading
   - Plugin system for custom panels

3. **Enhanced Testing**
   - Property-based tests for event sequences
   - Snapshot tests for UI rendering

4. **Performance Optimizations**
   - Incremental rendering
   - Event batching

## Success Metrics

After refactor completion:

- **Code size:** 
  - `handlers.rs`: 1151 → ~200 lines (83% reduction)
  - `intent_executor.rs`: 902 → deleted (replaced by ~300 lines across git_processor + modal_processor)
  - Total reduction: ~1500 lines

- **Maintainability:**
  - Adding new command: 1 line (macro definition) vs 40+ lines (handler struct + impl)
  - Adding new panel: Implement `ListBehavior` trait (~20 lines) vs full component (~300 lines)

- **Testability:**
  - Components testable in isolation (no App dependency)
  - Clear test boundaries (backend, processor, component, integration)

- **Quality:**
  - Zero clippy warnings
  - All tests passing
  - No regressions in functionality
