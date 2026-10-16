# Active development plan

## Active Slice

Name: Real terminal review of historical patch views
Status: completed and verified

## Problem

Run the actual native Windows TUI against an isolated real Git repository and
check Commit Files, hunk/line/range selection, aggregate preview and options at
120x36 and 80x24. Native PTY captures revealed that passive Commit Files diffs
still use raw Git ANSI colors and two-space indentation, while hunk and custom
preview use themed backgrounds and a three-column gutter. Historical selection
also gives no visible indication when a cycles hunk/line/block modes.

## Smallest Slice

Route passive main/Branch Commit Files diffs through the existing read-only
preview row parser and shared diff renderer, preserving metadata, source text,
scrolling and error/loading fallbacks. Display current historical selection mode
and file in the Details bottom border using the existing caption projection.
Match workflow-menu footer hints to the menu's own j/k and arrow navigation.
Retain all input/Git/state ownership semantics and prior uncommitted work.

Acceptance: passive file preview, hunk view and custom preview share stable
content columns and themed add/remove/hunk styles; mode changes are visible;
selection/inclusion/search retain precedence. Inspect the rebuilt executable
again, including narrow windows, preview scroll/return and patch options.

## Non-Goals

No new Git operations, dependencies, state model or input changes; no workspace
Git mutations, commits/pushes, OS settings or Windows terminal UI automation.

## Expected Files

Pure UI Details projection, historical border captions and regression fixtures;
PRODUCT/DESIGN/architecture wording for unified historical previews; this plan.
Runtime inspection scripts, isolated repository and output remain under tmp/.

## Tests

Focused UI snapshots and exact buffer-cell assertions comparing passive,
interactive and aggregate previews under a configured theme. Verify mode captions
for hunk/line/block plus Branch Commit Files; retain real keyboard and harness
coverage. Review native PTY text/color captures and PNG renderings of that output.

## Harness Decision

Presentation-only changes use deterministic UI/style regressions. Existing
cross-file harness and real-Git keyboard tests cover unchanged operations.
Native binary inspection supplements them and checks HEAD/index/worktree remain
unchanged. Temporary Python PTY/emulator/image dependencies stay outside product.

## Validation

Build and inspect the real executable; focused UI/keyboard/harness tests;
cargo fmt --check; cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace; git diff --check. Preserve failure artifacts.

## Completion Evidence

Before captures: tmp/patch-ui-live/20261016-170320/03-commit-files.png,
07-hunk-selection.png, 12-preview.png, 14-options-narrow.png and terminal.ansi.
Initial monochrome run inherited the tool shell's NO_COLOR=1; the controlled
color run removes that variable only from the child process and uses installed
Maple Mono Nerd Font for rendering captured terminal output. Actual process,
ConPTY input, native Git data and async loading are exercised.

Repaired passive diff colors/alignment, historical mode/file captions and menu
footer hints. Focused UI suite passes, including two new regressions comparing
all three diff views' exact columns and custom-theme cell styles, native ANSI
override, main/Branch projection, mode cycling and preview caption ownership.
Affected snapshots reviewed, with no pending .snap.new files. Existing source
file/range/preview fixtures retain selection and inclusion style assertions.

Rebuilt executable inspected under native ConPTY at 120x36 and 80x24. Evidence:
tmp/patch-ui-live/20261016-170956/{03-commit-files,07-hunk-selection,
11-line-range-extend,12-preview,14-preview-narrow,16-options-narrow,
19-block-mode-narrow,20-file-list-narrow,23-branch-commit-files,
26-branch-preview-return}.png. Corresponding .txt files, terminal.ansi,
input.jsonl and verification.json retain exact output, keys and Git state.
Verified cross-file whole+partial inclusion, hunk/line/range/block selection,
preview scroll, menu and Tab returns in main and Branch Commit Files. Native
inspection left HEAD, index entries and clean worktree unchanged; its process
was closed. The user's pre-existing ratagit process remains running. Build and
gate use isolated target directories under tmp/patch-ui-live to avoid its exe
lock. Native Windows final gate passed on 2026-10-16: cargo fmt --check;
cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo test --locked --workspace (748 passed, zero failed/ignored);
git diff --check. Logs: tmp/patch-ui-live-final-{fmt,clippy,tests}.log.
Both prior real-keyboard flows, main/Branch commit-file harness flows and the
cross-file accumulation UI/trace/state scenario passed. No new product
dependencies, input/state transitions or Git behavior; prior uncommitted work
is preserved. No commit or push.
