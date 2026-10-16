# Active development plan

## Active Slice

Name: Refactor UI colors into a configurable, validated theme
Status: completed and verified

## Problem

Rebase, reset and command palette selection remains difficult to read because
modal selection uses terminal-dependent ANSI colors. Palette/workflow menus
also highlight only their text width. The user expanded scope to inspect and
refactor the color system so configuration can control the colors and prevent
unreadable foreground/background combinations.

## Smallest Slice

Centralize semantic UI colors, including existing scattered diff and commit
colors, in an immutable UiTheme. Load a user JSON configuration before terminal
initialization; allow a built-in base theme plus named semantic color overrides.
Use RATAGIT_CONFIG for an explicit path and platform user config directories for
the default. Pass the theme explicitly through RenderContext and rendering
helpers; no mutable global theme, filesystem or environment access in rendering.
Retain default convenience APIs for existing consumers/tests.

Use explicit dark RGB text on pale blue for modal selection and full-row painting
for palette/workflow lists. Validate configured foreground/background pairs for
selection, modal selection and diff highlights, reject malformed colors/unknown
roles/low contrast with actionable startup errors. Preserve unselected semantic
colors and native Git ANSI history output.

Acceptance:
- User config selects default or Tokyo Night base and overrides documented roles.
- All application-owned colors come from one theme (native Git ANSI is preserved).
- Same state/size/context yields deterministic output with no global mutation.
- Invalid or unreadable paired overrides report the field and error before raw mode.
- Selected modal labels, indicator and padding use one pair with contrast >=7:1;
  other enforced pairs meet >=4.5:1.
- Tests prove custom colors in real rendered cells across panels, modals and diffs,
  parsing/path/error behavior, and independence of two simultaneous themes.
- Existing text snapshots and navigation/Git behavior remain unchanged.

## Non-Goals

No Git/backend changes, live config watching, theme editor, remote writes,
automatic commit/push, or application state ownership changes.

## Expected Files

- libs/ratagit-ui/src/theme.rs: public immutable model, defaults, validation.
- UI render helpers, projections, modal and diff formatters: explicit theme inputs.
- src/config.rs and main.rs: startup config discovery/loading/errors.
- Cargo manifests/lockfile as required for existing serde/serde_json dependencies.
- UI/config regression tests and existing style expectations.
- docs/THEMES.md with runnable example config, DESIGN/PRODUCT/ARCHITECTURE updates.

## Tests

Exact rendered cells across reset/rebase/palette/workflow, panel selections,
commit/status colors and diff states; contrast table; parsed custom config and
invalid fields/pairs; platform path precedence tests with injected inputs. Run
actual Windows executable with default and custom RATAGIT_CONFIG in isolated
fixtures using existing ConPTY tools. Review any snapshot differences; color-only
changes should not change text snapshots.

## Harness Decision

Presentation/configuration change: focused UI and executable config tests plus
live terminal evidence. Existing harness coverage remains in the workspace gate.

## Validation

cargo fmt --check; cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace; git diff --check. Native Windows tools through
PowerShell. Stop live executable before rebuilding workspace targets.

## Completion Evidence

Implemented immutable UiTheme/RenderContext propagation through all panel,
projection, modal and diff render paths. File/commit/diff application colors now
come from theme roles. Native Git ANSI output remains intentionally unchanged.
63 semantic override keys and two built-in presets are documented in THEMES.md.
The executable loads JSON before terminal setup; missing default config uses
built-ins, explicit missing/invalid files return path-bearing startup errors.

Modal selections use RGB dark-on-pale-blue at >=7:1, full-row including palette
and workflow padding. Configured selection/batch/badge pairs and all native diff
pairs validate their resolved contrast. RGB semantic text is checked against its
base/modal surface. Known identical ANSI/index colors are rejected; inherited
legacy ANSI/default colors retain documented validation limitations. Removed the
unused modal.success role after independent review.

Tests include 14 parser/path/error cases, invalid configuration before terminal
setup, exact default modal selections across choices, per-context custom-theme
isolation, custom panels/status/commit/diff/modal colors and padding. Existing
text snapshots passed without any artifact changes. No .snap.new files remain.

Actual Windows TUI captures passed for default fix and a custom JSON loaded via
RATAGIT_CONFIG. Custom foreground/background and >=50 full-row selection cells
were checked in reset/rebase/palette before and after moving selection.
Evidence tmp/ui-readability/select-lists/{before,after,custom}/captures.html,
per-screen text/color JSON and results.json. Live executable exited normally.

Final cargo fmt --check, strict production workspace Clippy, full locked workspace
tests (699 passed, zero failed/ignored), and git diff --check passed. Final full
suite log: bsgrq2s7e.output in the session task directory. Earlier full gate also
passed; validation strengthening was followed by a complete successful rerun.
No snapshot baseline changes. No commit or push.
