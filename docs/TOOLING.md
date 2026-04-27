# TOOLING.md

## Required Tools

- rustfmt
- clippy
- insta (snapshots)
- tracing

---

## Optional

- cargo-nextest
- cargo-udeps
- cargo-audit

---

## Commands

```bash
cargo fmt
cargo clippy --workspace --lib --bins -- -D warnings
cargo test
```

`clippy -D warnings` is scoped to production library and binary targets. Test
code is allowed to favor readable fixtures and scenario flow over strict lint
polish, provided `cargo test`, snapshots, and harness scenarios pass.

---

## Snapshot

```bash
cargo insta review
```

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

Reports are written to `tmp/perf/results/<run-id>.json` and
`tmp/perf/results/<run-id>.md`. The suite is intentionally not part of the
default `cargo test` or CI flow.
