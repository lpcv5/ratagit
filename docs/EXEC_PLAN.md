# EXEC_PLAN.md

## Current Slice

Tiered logging and Git backend performance diagnostics.

## Goal

- install a real `tracing` file subscriber before the TUI starts
- keep default logs low-noise at `info`
- emit detailed Git backend timings at `debug` for large repository diagnosis
- avoid logging Git stdout, diff text, or commit message payloads
- keep rendering pure and `AppState` as the only source of truth

## Vertical Slice

1. Observability
- initialize file logging through `ratagit-observe`
- default to the platform user state directory
- support `RATAGIT_LOG`, `RATAGIT_TRACE=1`, and `RATAGIT_LOG_PATH`

2. Git performance events
- log stable command labels, mutation status, result labels, and elapsed time
- log async worker queue delay and execution time
- log Git CLI subprocess duration, stdout byte count, optional lock mode, and
  failure summaries without stdout payloads
- log status mode, parse/sort timings, index entry count, truncation, and git2
  fallback

3. Tests and harness
- cover observability env parsing
- capture debug tracing in a real Git status refresh without leaking file
  contents
- keep large-repo harness UI and Git-state assertions stable with tracing
  enabled

4. Validation
- run `cargo fmt`
- run `cargo clippy --workspace --lib --bins -- -D warnings`
- run `cargo test`
