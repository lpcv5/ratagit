# Active development plan

## Active Slice

Name: Improve panel separation, compressed height, and color readability
Status: completed and verified

## Problem

The user requests independent panel borders, readable foreground/background
combinations throughout the UI, and at least three visible content rows in
compressed panels. Current documentation describes shared inactive borders and
an unfocused Stash with only one content row. Semantic foreground spans can
interact poorly with selected-row backgrounds.

## Smallest Slice

Keep rendering pure and change presentation in ratagit-ui. Give each panel its
own full rounded border; reserve three content rows plus both border rows for
compressed left panels whenever terminal dimensions permit. Audit all active
palette roles, row-span composition, native ANSI text, diff rows, and modal
selections. Use consistent readable selection treatments while retaining useful
semantic colors on ordinary rows. Inspect the allocator before selecting a
bounded fallback for terminals too short to fit all panels.

Acceptance criteria:
- Every panel owns all four border edges without sharing or overwriting an edge.
- Every compressed left panel has at least three content rows when height permits;
  focused panels can borrow only height above that reserve.
- Small terminal layouts stay within bounds, without overlapping panels or panic.
- Cursor, batch, search, diff, commit metadata and modal selections have readable
  foreground/background combinations, including explicitly styled spans.
- Deterministic snapshots cover the changed geometry; buffer styles separately
  prove readability across panels and modal selections.

## Non-Goals

No Git/backend/runtime behavior changes, theme selector, dependency changes,
commit, push, or external publication. Preserve unrelated working-tree changes.

## Expected Files

- libs/ratagit-ui/src/layout.rs and terminal.rs: sizing and panel frames.
- libs/ratagit-ui/src/theme.rs and relevant row renderers: semantic colors and
  selection composition as identified by the audit.
- libs/ratagit-ui/tests and snapshots: geometry and cell-style regressions.
- docs/DESIGN.md and docs/PRODUCT.md: updated rendering contracts.
- docs/EXEC_PLAN.md: scope, decisions and completion evidence.

## Tests

Run focused UI tests, inspect each changed snapshot against intended geometry,
and add meaningful buffer-style assertions where existing coverage misses
explicit foreground spans over selected-row backgrounds. Exercise each panel
focus and small terminal sizes. Where feasible, capture the rebuilt actual
Windows TUI through the existing local ConPTY driver to inspect borders/colors.

## Harness Decision

This is a presentation change. UI snapshots and cell assertions provide primary
regression coverage. Existing full workspace harness scenarios must continue to
pass; no new Git interaction scenario is required.

## Validation

Run cargo fmt --check, cargo clippy --locked --workspace --lib --bins -- -D
warnings, cargo test --locked --workspace, and git diff --check after focused
verification. The rtk wrapper and apply_patch command are unavailable in this
session; use underlying Cargo/Git commands and available file editing tools.
Use native Windows Rust tools through PowerShell.

## Completion Evidence

Implemented independent rounded borders for all six panels; initial allocation
and focus borrowing now preserve three content rows per left panel at terminal
heights >=21. Details/Log share a minimum-aware allocator with the viewport helper.
Smaller terminals conserve height and reduce the common reserve deterministically.

Batch foreground/background now overlays every semantic span and padding, while
search substrings retain underline. Ordinary author blue is brighter RGB. Diff
text/gutter pairs have tested contrast >4.5:1; modal scrims reset old backgrounds.
Native history ANSI semantics remain covered by existing tests.

Coverage: allocator conservation/minimum tests over heights 0..100, actual buffer
edges across all six focuses and seven sizes, exact marked commit/branch commit,
file/status/search and Commit Files glyph/padding styles. Reviewed snapshots
cover panels, Details entry, partial staging and dialogs. All 66 changed snapshot
baselines were reviewed, including twelve workflow dialogs with unchanged modal
rectangles/text and three additional partial staging snapshots.

The rebuilt actual Windows TUI passed navigation, visual commit selection,
white-on-blue glyph assertions, search underline, modal and resize verification
at 120x40, 80x24, 100x21 and 80x14. Evidence: tmp/ui-readability/live-ui.html,
results.json, per-screen text and cell-color JSON, plus ConPTY raw/keys in
 tmp/git-workflow-audit/screens/ui-readability/. The terminal process exited.

Final cargo fmt --check, strict production workspace Clippy, full locked
workspace tests (671 passed, zero failed/ignored), and git diff --check passed.
Full gate log: b5su6k9dw.output under the session task directory. An earlier
attempt encountered the live Windows executable file lock; the successful gate
ran after process exit. No pending .snap.new artifacts remain.
Concurrent AGENTS.md/CLAUDE.md edits were observed and preserved, not authored
as part of this UI change. No commit or push.
