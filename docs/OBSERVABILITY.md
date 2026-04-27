## Goals

- Debug UI mismatches
- Reproduce failures
- Inspect state transitions

---

## Logging

Use `tracing`.

Default level:

- `info`

Env:

- `RATAGIT_LOG=error|warn|info|debug|trace`
- `RATAGIT_LOG=ratagit=info,ratagit.git=debug` for full env-filter syntax
- `RATAGIT_TRACE=1` enables `trace` only when `RATAGIT_LOG` is unset
- `RATAGIT_LOG_PATH=/path/to/ratagit.log` overrides the file path

Events:

- input.key
- action.dispatched
- state.updated
- render.frame
- git.command start/finish/failure with command label, mutating flag, result
  label, and elapsed time
- async git worker queue delay and execution time
- git.refresh step timings: head, index count, status, status parse, status
  sort, commits, branches, stashes
- git.diff step timings: unstaged, untracked, staged
- git CLI subprocess duration, stdout byte count, optional-locks mode, status
  mode, truncation, and failure summaries

---

## Render Tracing

Each frame:

- terminal size
- focused panel
- selection state

---

## Log Output

Write to file:

- Windows: `%LOCALAPPDATA%\ratagit\ratagit.log`
- Unix: `$XDG_STATE_HOME/ratagit/ratagit.log`
- Unix fallback: `~/.local/state/ratagit/ratagit.log`

Never print logs to terminal during TUI.

Logs must not include Git stdout payloads, diff text, or commit message bodies.

---

## Debug Mode

Env:

RATAGIT_LOG=debug
RATAGIT_TRACE=1

`debug` is the recommended level for diagnosing slow Git backend behavior in
very large repositories.

---

## State Dump

Support:

ratagit --dump-state
