# EXEC_PLAN.md

## Goal

Refresh the TUI with a conservative lazygit-like theme: selection is shown by
color only, inactive panels do not show cursor highlights, and panel/file state
uses a small semantic Nerd Font icon set.

## Vertical Slice

1. UI rendering modules
- keep rendering pure and derived only from `AppState`
- introduce semantic panel rows carrying text, selected state, and style role
- keep public rendering APIs stable and add a testable terminal buffer helper
- make fixed-width text rendering Unicode-width aware

2. Theme behavior
- remove visible `>` cursor markers from all panels and snapshots
- remove the focused-title `*` marker
- highlight only the selected row in the focused selectable panel
- use icons for panels, directories, files, current branch, staged/untracked
  state, multi-select, and search matches

3. UI tests
- update panel projection tests for marker-free icon text
- snapshot fixed terminal sizes `80x24`, `100x30`, and `120x40`
- assert selection and focus styles through `ratatui::TestBackend` buffer cells

4. Harness scenarios
- keep screen, Git operation, and Git state expectations
- add a style-aware selected-row assertion for a files scenario
- keep failure artifacts for compatibility text, real screen, state, Git trace,
  final mock state, and input sequence

5. Quality gates
- run `cargo fmt`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
- run `cargo test -p ratagit-ui`
- run `cargo test -p ratagit-harness`
