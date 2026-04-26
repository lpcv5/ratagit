# EXEC_PLAN.md

## Goal

Enhance TUI verification so UI changes are validated through real panel
projection, full-screen `ratatui` rendering, and command-to-render harness
scenarios rather than only string-level guesses.

## Vertical Slice

1. UI rendering modules
- split `ratagit-ui` into focused modules for frame helpers, panel projection,
  terminal rendering, and compatibility text rendering
- keep the public API stable: `render`, `render_terminal`, `TerminalSize`, and
  `RenderedFrame`
- add `render_terminal_text` so tests and harness scenarios inspect the real
  `render_terminal` buffer

2. Panel unit tests
- test each panel's pure projection from `AppState`
- cover focus-derived details, search/multi-select file rows, log errors, and
  contextual keys

3. Full-screen UI snapshots
- add `insta` snapshots for fixed terminal sizes `80x24`, `100x30`, and
  `120x40`
- snapshot the real `ratatui::TestBackend` screen, including panel borders and
  the bottom keys area
- keep the old `render()` tests as compatibility checks until that path is
  removed by an explicit design change

4. Harness scenarios
- migrate scenarios to structured expectations for screen text, Git operation
  trace, and final mock Git state
- write failure artifacts for text render, real screen render, `AppState`, Git
  operation trace, final mock state, and input sequence

5. Quality gates
- run `cargo fmt`
- run `cargo clippy --all-targets -- -D warnings`
- run `cargo test`
- run `cargo test -p ratagit-ui`
- run `cargo test -p ratagit-harness`
