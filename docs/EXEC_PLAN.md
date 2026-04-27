# EXEC_PLAN.md

## Current Slice

Synthetic repository and Git CLI baseline performance suite.

## Goal

- make large-repository performance tests reproducible without cloning public
  monster repositories
- generate deterministic repositories with configurable file count, commit
  count, binary file count, and binary file size
- compare raw Git CLI speed, parsed Git CLI speed, and `HybridGitBackend`
  structured-result speed for the same operations
- keep performance validation out of the default development and CI flow

## Vertical Slice

1. Tooling behavior
- add a local generator CLI for synthetic Git repositories
- support configurable scale, file count, directory fanout, text file size,
  commit count, binary file count, and binary file size
- initialize and commit through the real Git executable
- write a deterministic manifest beside the marker file

2. Safety and determinism
- write only to an explicitly provided target path
- guard destructive regeneration with a marker file and `--force`
- create deterministic text paths, binary paths, file contents, and commit
  groups

3. Performance suite
- add `perf-suite` as a manual CLI
- default to smoke, small, medium, and large scales, with large capped at
  200,000 files
- require explicit `--scales huge` for 1,000,000-file validation
- measure status, commit list, commit pagination, commit file list, commit diff,
  commit file diff, and worktree file diff
- write Markdown and JSON reports under `tmp/perf/results`

4. Tests
- add unit coverage for argument parsing, scale defaults, binary validation,
  baseline command construction, parsed baseline helpers, and report summaries
- add a smoke performance-suite test using a tiny synthetic repository
- avoid creating large or huge repositories during normal `cargo test`

5. Documentation
- document generator usage, perf-suite usage, output reports, and manual large
  validation commands

6. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
