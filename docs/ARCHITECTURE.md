# Architecture

## Event-Driven System

Ratagit uses an event-driven architecture where user input flows unidirectionally through the system:

```
User Input → Component → AppEvent → App::process_event → Processor → Backend/State
```

This design provides:
- Clear separation of concerns (UI, business logic, Git operations)
- Predictable data flow
- Easy testing and debugging
- Type-safe event handling

## Event Flow

```
┌─────────────┐
│ User Input  │
└──────┬──────┘
       │
       ▼
┌─────────────────────┐
│ Component::         │
│ handle_key_event    │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│    AppEvent         │
└──────┬──────────────┘
       │
       ▼
┌─────────────────────┐
│ App::process_event  │
└──────┬──────────────┘
       │
       ├─────────────────┬──────────────────┐
       ▼                 ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│GitProcessor  │  │ModalProcessor│  │ Direct State │
└──────┬───────┘  └──────┬───────┘  └──────┬───────┘
       │                 │                  │
       ▼                 ▼                  ▼
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│BackendCommand│  │ State Update │  │ Panel Switch │
└──────────────┘  └──────────────┘  └──────────────┘
```

## Key Types

### AppEvent (src/app/events.rs)

Top-level event enum returned by all components:

- `Git(GitEvent)` - Git operations (stage, commit, discard, etc.)
- `Modal(ModalEvent)` - Modal dialogs (help, confirmations, text input)
- `SwitchPanel(Panel)` - Navigate between panels
- `ActivatePanel` - Activate current panel (Enter key)
- `SelectionChanged` - Selection changed, refresh main view
- `None` - Event handled internally by component

### GitEvent

Git operation events:

- `ToggleStageFile` - Stage/unstage selected file(s)
- `StageAll` - Stage all unstaged files
- `CommitWithMessage(String)` - Create commit with message
- `DiscardSelected` - Discard changes to selected file(s)
- `StashSelected` - Stash selected file(s)
- `AmendCommit` - Amend last commit
- `ExecuteReset(usize)` - Execute git reset (soft/mixed/hard)
- `IgnoreSelected` - Add selected file to .gitignore
- `RenameFile(String)` - Rename selected file

### ModalEvent

Modal dialog events:

- `ShowHelp` - Show context-sensitive help
- `ShowCommitDialog` - Show commit message input
- `ShowRenameDialog` - Show file rename input
- `ShowResetMenu` - Show reset mode menu
- `ShowDiscardConfirmation` - Confirm discard operation
- `ShowStashConfirmation` - Confirm stash operation
- `ShowAmendConfirmation` - Confirm amend operation
- `ShowResetConfirmation(usize)` - Confirm reset operation
- `ShowNukeConfirmation` - Confirm repository deletion
- `Close` - Close active modal

## Processors

### GitProcessor (src/app/processors/git_processor.rs)

Converts `GitEvent` to one or more `BackendCommand` instances:

- Handles multi-select logic (stage/unstage multiple files)
- Determines stage/unstage direction based on anchor file
- Filters out directories (only operates on files)
- Returns empty vec if no valid targets

Example flow:
```
GitEvent::ToggleStageFile
  → GitProcessor::toggle_stage_file()
  → Check anchor file status
  → Generate BackendCommand::StageFile or UnstageFile
  → Send to backend via channel
```

### ModalProcessor (src/app/processors/modal_processor.rs)

Updates `AppState` to show/hide modal dialogs:

- Creates appropriate modal type (help, confirmation, text input, menu)
- Stores modal in `AppState.active_modal`
- Modal handles its own input and returns `AppEvent`
- Closing modal sets `active_modal` to `None`

Example flow:
```
ModalEvent::ShowCommitDialog
  → ModalProcessor::process()
  → Create ModalDialogV2::text_input()
  → Store in state.active_modal
  → Modal renders on next frame
  → User types message, presses Enter
  → Modal returns AppEvent::Git(CommitWithMessage)
  → GitProcessor converts to BackendCommand::Commit
```

## Component System

### ComponentV2 Trait (src/components/component_v2.rs)

All UI panels implement this trait:

```rust
pub trait ComponentV2 {
    fn handle_key_event(&mut self, key: KeyEvent, state: &AppState) -> AppEvent;
    fn render(&self, area: Rect, buf: &mut Buffer, state: &AppState);
}
```

Components are stateless - they read from `AppState` and return events. No direct state mutation.

### Component Implementations

- `FileListPanel` - File tree with stage/unstage
- `BranchListPanel` - Branch list with checkout/delete
- `CommitPanel` - Commit history with details
- `StashListPanel` - Stash list with apply/pop/drop
- `MainView` - Diff/detail viewer
- `LogPanel` - Operation log

## Backend Communication

### Channel Protocol

- UI → Backend: `CommandEnvelope { request_id, command }`
- Backend → UI: `EventEnvelope { request_id, event }`

### Request Tracking

`RequestTracker` (src/app/request_tracker.rs) ensures:
- Each request gets a unique ID
- Stale responses are dropped
- Duplicate responses are ignored

### Backend Commands

Key commands sent to backend:

- `RefreshStatus` - Reload git status
- `RefreshBranches` - Reload branch list
- `RefreshCommits` - Reload commit history
- `RefreshStashes` - Reload stash list
- `StageFile/StageFiles` - Stage file(s)
- `UnstageFile/UnstageFiles` - Unstage file(s)
- `Commit { message }` - Create commit
- `GetDiff { file_path }` - Load file diff
- `GetCommitFiles { commit_id }` - Load commit files

### Frontend Events

Key events received from backend:

- `FilesUpdated { files }` - Status refreshed
- `BranchesUpdated { branches }` - Branches refreshed
- `CommitsUpdated { commits }` - Commits refreshed
- `StashesUpdated { stashes }` - Stashes refreshed
- `DiffLoaded { file_path, diff }` - Diff loaded
- `CommitFilesLoaded { commit_id, files }` - Commit files loaded
- `ActionSucceeded { message }` - Operation succeeded
- `Error { message }` - Operation failed

## Main Loop (src/app/runtime.rs)

```rust
async fn main_loop(&mut self, terminal: &mut Terminal) -> Result<()> {
    while !self.state.should_quit {
        // 1. Drain backend events
        self.drain_backend_events().await?;
        
        // 2. Render UI
        terminal.draw(|frame| self.render(frame))?;
        
        // 3. Poll input (100ms timeout)
        if event::poll(Duration::from_millis(100))? {
            let input = event::read()?;
            
            // 4. Route to modal or panel
            if let Some(modal) = &mut self.state.active_modal {
                let app_event = modal.handle_event_v2(&input);
                self.process_event(app_event);
            } else {
                self.handle_input_v2(input)?;
            }
        }
    }
}
```

## Design Principles

1. **Unidirectional Data Flow** - Events flow one way: Input → Event → Processor → State
2. **Separation of Concerns** - UI, business logic, and Git operations are decoupled
3. **Type Safety** - All events are strongly typed enums
4. **Testability** - Processors are pure functions (GitProcessor) or simple state updates (ModalProcessor)
5. **No Direct Mutation** - Components never mutate state directly, only return events
6. **Async Backend** - All Git I/O happens on a background task, UI stays responsive

## Migration Notes

The event-driven architecture replaced the previous Intent-based system:

- Old: `Component::handle_event() → Intent` → `IntentExecutor`
- New: `ComponentV2::handle_key_event() → AppEvent` → `App::process_event()` → Processors

Benefits of the new system:
- Clearer event semantics (Git vs Modal vs Panel)
- Easier to test (processors are simpler than intent executor)
- Better type safety (no generic Intent enum)
- More explicit control flow (no hidden side effects)
