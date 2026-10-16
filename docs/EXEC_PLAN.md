# Active development plan

## Active Slice

Name: Command palette slash search and j/k navigation
Status: completed and verified

## Problem

The command palette currently treats ordinary typing as search input. The user
wants `/` to enter search explicitly and `j` / `k` to navigate the command list.

## Smallest Slice

Add an explicit palette search-editing state owned by AppContext and driven by
reducer actions. Open the palette in navigation mode; `j` / `k` and arrow keys
move selection. `/` enters search, where characters (including j/k) and
Backspace edit the filter. Enter/Esc return from editing to navigation without
executing a command.
Retain the filter when returning to navigation so users can select a match.
In navigation, Enter executes and Esc closes the palette. Backspace in navigation
must not edit the filter; reopening / resumes editing the retained query.
Update command palette hints and relevant behavior/design documentation.

Acceptance:
- Opening the palette starts in navigation mode with an empty query.
- j/k navigate without changing the query; other ordinary typing does not filter.
- / explicitly enters search; j/k are literal query text while editing.
- Search can return to navigation while retaining the current filter.
- Execution, cancellation, quit, external editor routing and arrow navigation
  continue to respect palette ownership and search mode.
- Tests cover key mapping, reducer transitions, rendered hints and a harness
  flow with operation trace and unchanged Git state for navigation/search.

## Non-Goals

No backend/Git mutation changes, global key rebinding, other panel search changes,
new dependencies, automatic commit or push.

## Expected Files

- src/input.rs and input/workflow tests: palette key handling and execution routing.
- libs/ratagit-core: palette state/actions and reducer transitions.
- libs/ratagit-ui: mode-aware palette hint text and UI snapshot coverage.
- libs/ratagit-harness/tests: palette interaction scenario and scenario index.
- docs/PRODUCT.md and docs/DESIGN.md: palette interaction contract.

## Tests

Extend existing palette mapping, reducer, UI and harness coverage rather than
adding redundant test layers. Inspect exact snapshot changes and ensure focused
filters run the intended tests.

## Harness Decision

This changes an input flow across executable, core and UI. Extend a deterministic
palette harness scenario to verify search/navigation UI, operation trace and
absence of unintended backend mutation, alongside executable key mapping tests.

## Validation

Use the native Windows Rust toolchain through PowerShell. Focused palette tests
first, then cargo fmt --check; cargo clippy --locked --workspace --lib --bins --
-D warnings; cargo test --locked --workspace; git diff --check.

## Completion Evidence

Implemented palette-owned search_active state and explicit StartPaletteSearch /
FinishPaletteSearch actions. Navigation uses j/k, arrows and /; search owns literal
characters and Backspace, with Enter/Esc retaining the filter and selected row.
Open, close, execution and HostFinished reset search mode. Editing and execution
reducers reject actions outside their intended palette mode.

Executable input derives a separate palette-search mode, suppresses plain-q
fallthrough, and keeps search Enter out of Quit, file editing, path/branch/commit
copying and external commit editor routes. Global Ctrl+C and Details scrolling
retain their previous behavior.

UI hints reflect the active mode and show an empty filter row immediately after
/. Reviewed all five affected/new snapshots (navigation, empty search, confirmed
search, filtered search, no matches); no pending .snap.new artifacts. Existing
selection/theme checks pass. Product/design docs describe the interaction.

Extended harness_command_palette_executes_global_pull (same scenario index):
checks navigation/search hints, retained filter, no backend operations or Git
state changes before execution, and the selected pull operation afterward.
Focused checks passed: 184 core tests, 65 executable tests, 166 UI tests and the
exact palette harness scenario. Independent read-only review found no defects.
Final cargo fmt --check, strict production workspace Clippy, full locked
workspace tests (707 passed, zero failed/ignored) and git diff --check passed on
native Windows. Full suite log: bldommuke.output in the session task directory.
No commit or push.
