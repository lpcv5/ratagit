## Overview

ratagit is a lazygit-like Git TUI built with Rust and ratatui.

The system follows a strict unidirectional data flow:

Input → Action → Update → AppState → Render

---

## Core Principles

### 1. Single Source of Truth

All UI must be derived from `AppState`.

- No hidden state
- No implicit global variables
- No UI-only state outside AppState

---

### 2. Pure Rendering

UI rendering must be pure:

```text
(AppState, TerminalSize) -> Frame
```

Forbidden:

- calling Git inside render
- reading env/time/random
- mutating state

---

### 3. Layer Separation

```text
CLI → Core → UI
        ↓
      Git
```

Rules:

- UI cannot call Git
- UI cannot mutate AppState
- Core owns all state transitions
- Git is accessed only via `GitBackend`

---

### 4. Side Effects via Commands

Update returns commands:

```rust
fn update(state: &mut AppState, action: Action) -> Vec<Command>
```

Commands:

- Git operations
- async tasks
- IO

---

### 5. Determinism

Same:

- AppState
- terminal size
- input sequence

Must produce identical UI.

---

## Modules

### ratagit-core

- AppState
- Action
- Reducer (update)
- Command

### ratagit-ui

- Pure rendering functions
- Widgets
- Layout

### ratagit-git

- GitBackend trait
- Implementations

### ratagit-observe

- tracing setup
- log sinks

### ratagit-testkit

- fixtures
- UI assertions

### ratagit-harness

- scenario runner
- input driver
- snapshot + assertions

---

## MVP Implementation Notes

- The repository is organized as a Cargo workspace with crates:
  - `ratagit-core`
  - `ratagit-ui`
  - `ratagit-git`
  - `ratagit-observe`
  - `ratagit-testkit`
  - `ratagit-harness`
  - `ratagit-app`
- Runtime command execution uses `ratagit-harness::Runtime` to preserve:
  - single source of truth in `AppState`
  - side effects only through `Command` + `GitBackend`
  - pure rendering in `ratagit-ui::render`

---

## Event Loop

```text
read input
→ map to Action
→ update(AppState)
→ run Commands
→ receive results
→ render
```

---

## Anti-Patterns (Forbidden)

- UI directly mutates state
- UI calls Git
- logic inside render()
- branching based on terminal state outside AppState
- hidden caches not in AppState

---

## Code Structure Rules

### File Size

- max 500 lines per file
- split when growing

### Module Rules

- one responsibility per module
- no cyclic dependencies

### Naming

- explicit > clever
- avoid abbreviations

---

## State Design Rules

AppState must:

- be serializable (for debugging)
- be inspectable
- avoid nested complexity explosion

---

## Action Design Rules

Actions must:

- be explicit
- not carry hidden meaning
- be testable
