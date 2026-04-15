# Event-Driven Component Refactor

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
    ActivatePanel,         // Enter key: branch→load commits, commit→load files
    SelectionChanged,      // component selection changed, refresh main view
    None,                  // component handled internally
}

pub enum GitEvent {
    ToggleStageFile,
    StageAll,
    CommitWithMessage(String),
    DiscardSelected,      // Opens confirmation modal first
    StashSelected,        // Sends stash command directly
    AmendCommit,          // Opens confirmation modal first
    ExecuteReset(usize),  // Receives index after user selects from modal
    IgnoreSelected,
    RenameFile(String),
}

pub enum ModalEvent {
    ShowHelp,
    ShowCommitDialog,
    ShowRenameDialog,
    ShowResetMenu,
    ShowDiscardConfirmation,
    ShowStashConfirmation,
    ShowAmendConfirmation,
    ShowResetConfirmation(usize),  // index from reset menu
    ShowNukeConfirmation,
    Close,
}

```

**Key principle:** Components handle their own navigation (j/k/scroll) internally. When selection changes and the main view needs refresh, they return `AppEvent::SelectionChanged`. Only app-level coordination (git ops, panel switching, modals, detail refresh) bubbles up.

### React-Like Component Model

**Current problem:** Components can't handle their own state. Everything goes through App.

**New design:** Components own local state, receive read-only context:

```rust
// src/components/component.rs
pub struct RenderContext<'a> {
    pub data: &'a CachedData,   // git data (read-only) — from state.data_cache
    pub theme: &'a Theme,
    pub is_focused: bool,
}

// Note: AppState fields are ui_state and data_cache, not ui and cache

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
        // Branch checkout is future feature work, not in this refactor
        AppEvent::SelectionChanged
    }
}

pub type BranchListPanel = ConfigurableList<BranchInfo, BranchBehavior>;
```

**Benefits:**
- Stash/Branch/Commit panels share 90% of code
- Custom behavior via trait implementation
- Easy to add new panel types

**Note:** This requires a new `StatefulList<T>` type that owns both data and `ListState`. Current `SelectableList` is render-only.

### GitProcessor (replaces intent_executor.rs)

**Current problem:** 902-line `intent_executor.rs` with 33 methods split across two `impl App` blocks.

**New design:** Focused `GitProcessor` owns git event → backend command translation:

```rust
// src/app/git_processor.rs
pub struct GitProcessor;

pub enum ProcessorOutcome {
    SendCommand(BackendCommand),
    ShowModal(ModalEvent),
    None,
}

impl GitProcessor {
    pub fn execute(&self, event: GitEvent, state: &AppState) -> Result<ProcessorOutcome> {
        let outcome = match event {
            GitEvent::ToggleStageFile => {
                // Multi-select aware: gets all selected targets (files + directory children)
                // Uses anchor file to determine stage/unstage direction (pivot logic)
                // See src/app/intent_executor.rs:347 for current implementation
                let selected_targets = state.components.selected_file_tree_targets();
                let selected_files: Vec<String> = selected_targets
                    .into_iter()
                    .filter(|(_, is_dir)| !is_dir)
                    .map(|(path, _)| path)
                    .collect();
                if selected_files.is_empty() {
                    return Ok(ProcessorOutcome::None);
                }
                // Pivot: use anchor file's is_staged to decide direction
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
            GitEvent::StageAll => {
                ProcessorOutcome::SendCommand(BackendCommand::StageAll)
            }
            GitEvent::CommitWithMessage(msg) => {
                ProcessorOutcome::SendCommand(BackendCommand::Commit { message: msg })
            }
            GitEvent::DiscardSelected => {
                ProcessorOutcome::ShowModal(ModalEvent::ShowDiscardConfirmation)
            }
            GitEvent::StashSelected => {
                // Note: Current behavior sends command directly without confirmation.
                // Keeping that behavior for now. If confirmation is desired, change to:
                // ProcessorOutcome::ShowModal(ModalEvent::ShowStashConfirmation)
                let selected_targets = state.components.selected_file_tree_targets();
                let paths: Vec<String> = selected_targets
                    .into_iter()
                    .filter(|(_, is_dir)| !is_dir)
                    .map(|(path, _)| path)
                    .collect();
                ProcessorOutcome::SendCommand(BackendCommand::StashFiles { 
                    paths, 
                    message: None 
                })
            }
            GitEvent::AmendCommit => {
                ProcessorOutcome::ShowModal(ModalEvent::ShowAmendConfirmation)
            }
            GitEvent::ExecuteReset(index) => {
                // Current reset menu order (from intent_executor.rs:687):
                // 0: Hard Reset (HEAD)
                // 1: Mixed Reset (HEAD)
                // 2: Soft Reset (HEAD)
                // 3: Hard Reset (HEAD~1)
                // 4: Soft Reset (HEAD~1)
                // 5: Nuke Repository (not implemented)
                let (target, cmd) = match index {
                    0 => ("HEAD", BackendCommand::ResetHard { target: "HEAD".into() }),
                    1 => ("HEAD", BackendCommand::ResetMixed { target: "HEAD".into() }),
                    2 => ("HEAD", BackendCommand::ResetSoft { target: "HEAD".into() }),
                    3 => ("HEAD~1", BackendCommand::ResetHard { target: "HEAD~1".into() }),
                    4 => ("HEAD~1", BackendCommand::ResetSoft { target: "HEAD~1".into() }),
                    5 => return Ok(ProcessorOutcome::ShowModal(ModalEvent::ShowNukeConfirmation)),
                    _ => return Ok(ProcessorOutcome::None),
                };
                
                // Hard resets need confirmation
                let needs_confirmation = matches!(&cmd, BackendCommand::ResetHard { .. });
                if needs_confirmation {
                    ProcessorOutcome::ShowModal(ModalEvent::ShowResetConfirmation(index))
                } else {
                    ProcessorOutcome::SendCommand(cmd)
                }
            }
            // ... other events
        };
        
        Ok(outcome)
    }
}
```

**Note:** `GitProcessor` is stateless — it translates events to outcomes (commands or modal requests). `App` owns `RequestTracker` and handles command sending + response filtering.

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
                // Note: Current implementation restores saved branch commits when switching
                // back to Panel::Branches. This logic should be preserved in the new design.
                // See src/app/intent_executor.rs:94 for current behavior.
                self.state.ui_state.active_panel = panel;
                self.update_main_view_for_active_panel()?;
            }
            AppEvent::ActivatePanel => {
                // Enter key behavior: branch→load commits, commit→load files
                // See src/app/intent_executor.rs:109 for current implementation
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
5. Request ID threading
6. Post-action refresh logic (e.g., stage → refresh status)

**Note:** The macro syntax shown is simplified. The actual implementation must handle:
- Request IDs from `CommandEnvelope`
- **CRITICAL INVARIANT:** Only the terminal action event (ActionSucceeded, FilesUpdated, etc.) should carry the command's request_id. Follow-up refresh events (e.g., stage → auto-refresh files) must use `None` as request_id, otherwise `RequestTracker` will treat them as stale after the first completion. See src/backend/handlers.rs:475 and src/app/runtime.rs:73 for current pattern.
- Mutable vs immutable repo access (`[mut]` flag)
- Post-action refreshes (stage/unstage → refresh files)
- Custom result shaping (batch diffs, commit files)
- Error handling and event emission

**Result:**
- 1151 lines → ~300 lines (more realistic with full macro complexity)
- Adding new command: 1-3 lines instead of 40+ lines
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
│   ├── handlers.rs            (1151 → ~300 lines, macro-driven)
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
        └── modal.rs           (UPDATED — returns AppEvent instead of Intent;
                                 ModalType::Selection returns AppEvent::Git(GitEvent::ExecuteReset(idx)),
                                 ModalType::TextInput returns AppEvent::Git(GitEvent::CommitWithMessage(msg)),
                                 ModalType::Help returns AppEvent::ActivatePanel for help items.
                                 See src/components/dialogs/modal.rs:116 for current Intent coupling.)
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
fn test_branch_selection_triggers_detail_refresh() {
    let mut panel = BranchListPanel::new();
    let ctx = mock_render_context();
    
    let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    let result = panel.on_event(&event, &ctx);
    
    assert!(matches!(result, AppEvent::SelectionChanged));
}

#[test]
fn test_navigation_triggers_selection_changed() {
    let mut panel = BranchListPanel::new();
    let ctx = mock_render_context();
    
    // j/k keys change selection, should return SelectionChanged
    let event = Event::Key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert_eq!(panel.on_event(&event, &ctx), AppEvent::SelectionChanged);
}

#[test]
fn test_unhandled_keys_return_none() {
    let mut panel = BranchListPanel::new();
    let ctx = mock_render_context();
    
    // Keys that don't affect app state return None
    let event = Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
    assert_eq!(panel.on_event(&event, &ctx), AppEvent::None);
}
```

### GitProcessor Tests (new)
```rust
#[test]
fn test_stage_file_returns_command() {
    let processor = GitProcessor;
    let state = mock_app_state_with_unstaged_file("test.txt");

    let outcome = processor.execute(GitEvent::ToggleStageFile, &state).unwrap();

    assert!(matches!(outcome, ProcessorOutcome::SendCommand(
        BackendCommand::StageFile { file_path } 
    ) if file_path == "test.txt"));
}

#[test]
fn test_discard_returns_modal() {
    let processor = GitProcessor;
    let state = mock_app_state();

    let outcome = processor.execute(GitEvent::DiscardSelected, &state).unwrap();

    assert!(matches!(outcome, ProcessorOutcome::ShowModal(
        ModalEvent::ShowDiscardConfirmation
    )));
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
    
    assert_eq!(app.state().data_cache.commits.len(), 1);
}
```

## Migration Strategy

### Phase 1: Backend Cleanup (Medium Risk)
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

**Rollback:** `git revert <commit>` or `git reset --soft HEAD~1` if tests fail. Do NOT use `git reset --hard` in a dirty worktree — check `git status` first to avoid discarding unrelated work.

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
**Goal:** Migrate components to new trait without breaking existing code

**Challenge:** The Component trait is global — all panels implement it. Changing the signature breaks all un-migrated panels immediately.

**Solution:** Parallel trait approach with adapter pattern

**Steps:**

**3.1: Create ComponentV2 trait alongside Component**
1. Create `src/components/component_v2.rs`:
   ```rust
   pub trait ComponentV2 {
       fn on_event(&mut self, ctx: &RenderContext, event: &Event) -> AppEvent;
       fn render(&mut self, frame: &mut Frame, area: Rect, ctx: &RenderContext);
   }
   ```
2. Create `src/components/core/list_behavior.rs` trait
3. Keep original `Component` trait unchanged

**Validation:**
```bash
cargo check  # Should pass — no breaking changes yet
```

**3.2: Migrate StashListPanel (simplest) to ComponentV2**
1. Implement `ComponentV2` for `StashListPanel`
2. Keep old `Component` impl temporarily (forward to V2)
3. Move j/k navigation into component
4. Return `AppEvent` instead of `Intent`

**Validation:**
```bash
cargo test stash_list
cargo run  # Manual test: navigate stash panel
```

**Expected result:** Stash panel works identically to before

**3.3: Migrate remaining panels one by one**
- BranchListPanel
- FileListPanel  
- CommitPanel
- MainView
- LogPanel

After each panel:
```bash
cargo test <panel_name>
cargo run  # Manual smoke test
```

**3.4: Remove Component trait and adapters**
1. Delete `src/components/component.rs` (old trait)
2. Rename `ComponentV2` → `Component`
3. Remove all adapter code
4. Update `modal.rs` to return `AppEvent`

**Validation:**
```bash
cargo test
cargo clippy -- -D warnings
```

**3.3: Migrate BranchListPanel (has sub-panel)**
1. Update `BranchListPanel` to new trait
2. Handle sub-panel delegation
3. Test branch checkout, delete, sub-panel navigation

**Validation:**
```bash
cargo test branch_list
cargo run  # Manual test: navigate branches, view commits sub-panel
```

**3.4: Migrate CommitPanel**
1. Update `CommitPanel` to new trait
2. Handle three modes (List, Loading, FilesTree)
3. Test commit selection, file tree navigation

**Validation:**
```bash
cargo test commit_panel
cargo run  # Manual test: navigate commits, view files, view diffs
```

**3.5: Migrate FileListPanel (most complex)**
1. Update `FileListPanel` to new trait
2. Handle tree navigation, multi-select
3. Test stage/unstage, discard, ignore, rename

**Validation:**
```bash
cargo test file_list
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
- [ ] Navigate branches (checkout/delete are future features)
- [ ] View commit details
- [ ] Navigate stash list (apply/pop/drop are future features)
- [ ] Ignore files
- [ ] Rename files
- [ ] Reset (hard/mixed/soft)
- [ ] Help panel
- [ ] All modals (commit, rename, reset)
- [ ] Selection changes refresh main view

**Rollback:** `git revert <commit>` or `git reset --soft HEAD~1` to Phase 3 completion if critical bugs found. Check `git status` before any reset to avoid discarding unrelated work.

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
- If a phase fails, use `git revert <commit>` or `git reset --soft HEAD~1` to previous commit. Always check `git status` before any reset — `git reset --hard` can discard unrelated work in a dirty worktree.
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

1. **Branch Operations**
   - Add `CheckoutBranch`, `DeleteBranch` to `GitEvent`
   - Add corresponding `BackendCommand` variants
   - Add git_ops functions in `branches.rs`
   - Add handlers and tests

2. **Stash Operations**
   - Add `ApplyStash`, `PopStash`, `DropStash` to `GitEvent`
   - Add corresponding `BackendCommand` variants
   - Add git_ops functions in `stash.rs`
   - Add handlers and tests

3. **Undo/Redo via git reflog**
   - Add `backend/git_ops/reflog.rs`
   - Add `UndoToReflog` backend command
   - Add `u`/`Ctrl+r` keybindings
   - Show reflog panel

4. **Component Registry**
   - Dynamic panel loading
   - Plugin system for custom panels

5. **Enhanced Testing**
   - Property-based tests for event sequences
   - Snapshot tests for UI rendering

6. **Performance Optimizations**
   - Incremental rendering
   - Event batching

## Success Metrics

After refactor completion:

- **Code size:** 
  - `handlers.rs`: 1151 → ~300 lines (74% reduction)
  - `intent_executor.rs`: 902 → deleted (replaced by ~300 lines across git_processor + modal_processor)
  - Total reduction: ~1400 lines

- **Maintainability:**
  - Adding new command: 1-3 lines (macro definition) vs 40+ lines (handler struct + impl)
  - Adding new panel: Implement `ListBehavior` trait (~20 lines) vs full component (~300 lines)
  - Adding new git event: Add enum variant + match arm in `GitProcessor::execute` (~5 lines)

- **Testability:**
  - Components testable in isolation (no App dependency)
  - Clear test boundaries (backend, processor, component, integration)

- **Quality:**
  - Zero clippy warnings
  - All tests passing
  - No regressions in functionality
