## Goals

- Debug UI mismatches
- Reproduce failures
- Inspect state transitions

---

## Logging

Use `tracing`.

Events:

- input.key
- action.dispatched
- state.updated
- render.frame
- git.command
- git.refresh step timings: head, status, commits, branches, stashes
- git.diff step timings: unstaged, untracked, staged

---

## Render Tracing

Each frame:

- terminal size
- focused panel
- selection state

---

## Log Output

Write to file:

~/.local/state/ratagit/ratagit.log

Never print logs to terminal during TUI.

---

## Debug Mode

Env:

RATAGIT_LOG=debug
RATAGIT_TRACE=1

---

## State Dump

Support:

ratagit --dump-state
