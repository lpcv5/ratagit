# TOOLING.md

## Required Tools

- Stable Rust with Cargo and support for the 2024 edition
- rustfmt and Clippy components for the selected Rust toolchain
- Git, available on PATH for backend operations and real-Git tests

The application and test suite build through Cargo; Node.js and a JavaScript
package manager are not required. Cargo manages libraries such as `ratatui`,
`insta` (snapshot assertions), and `tracing` through the workspace manifests and
`Cargo.lock`; they are not separate tools to install.

---

## Optional

- cargo-insta (provides `cargo insta review` for snapshot review)
- cargo-nextest
- cargo-udeps
- cargo-audit

---

## Validation

Use the final gate in [AGENTS.md](../AGENTS.md) and the coverage guidance in
[TESTING.md](TESTING.md). Run `cargo fmt` when formatting needs correction.
Clippy and workspace tests compile the affected targets; a separate
`cargo check --locked --workspace` is useful during iteration but is not an
additional final-gate step.

---

## Snapshot review

Run the affected UI test, inspect any `.snap.new` against the intended behavior,
and review changes with cargo-insta if installed:

```sh
cargo insta review
```

Rerun the affected test after accepting intended changes. Do not bulk-accept
unexplained differences. Text snapshots do not prove cursor/selection colors;
use buffer-cell style assertions as described in [TESTING.md](TESTING.md).

Harness failures write diagnostic bundles under
`target/harness-artifacts/` (or `$CARGO_TARGET_DIR/harness-artifacts/`); some
specialized tests use `tmp/harness-artifacts/`. Inspect the screen, state,
operation trace and `failure_report.json` before retrying. CI uploads failure
artifacts from both OS jobs.

---

## Synthetic Large Repositories

Use the `make-large-repo` helper when performance testing needs a predictable
large Git repository without cloning a public monster repository:

```bash
cargo run --bin make-large-repo -- --scale large --path tmp/perf/large-repo --force
```

Useful scales:

- `--scale smoke` creates a tiny validation repo.
- `--scale large` creates 200,000 files and exercises large-repository fast
  status behavior.
- `--scale huge` creates 1,000,000 files and exercises metadata-only huge
  repository behavior.

The generator accepts explicit overrides such as `--files`, `--commits`,
`--binary-files`, and `--binary-bytes`. It writes deterministic text and binary
files under `data/`, initializes Git, commits the generated files, and places
both `.ratagit-large-repo-marker` and `.ratagit-large-repo.json` in the target
directory. Passing `--force` will only regenerate a directory that contains the
marker.

---

## Performance Suite

Run the manual performance suite when comparing ratagit backend behavior against
plain Git CLI baselines:

```bash
cargo run --bin perf-suite -- --regenerate
```

By default this runs `smoke,small,medium,large`, with `large` capped at 200,000
files. The 1,000,000-file scale is opt-in:

```bash
cargo run --bin perf-suite -- --scales huge --regenerate
```

Each operation records:

- `git_cli_raw`: raw Git command output time
- `git_cli_parsed`: Git command output plus local parsing/conversion time
- `backend`: `HybridGitBackend` method time returning structured data

The `commit-files-directory-diff` operation models the Commit Files subpanel
path used in the TUI: load changed files for a commit, build/select a directory
row, then load that directory's patch.

The `commit-files-navigation` operation models Commit Files keyboard movement:
load the changed-file list, build the tree projection, repeatedly apply `j`
movement through the reducer, and render the terminal buffer after each move.

Reports are written to `tmp/perf/results/<run-id>.json` and
`tmp/perf/results/<run-id>.md`. The suite is intentionally not part of the
default `cargo test` or CI flow.
