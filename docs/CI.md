# Continuous integration

## Required checks

Every PR runs on Ubuntu and Windows:

- cargo fmt --check
- cargo clippy --locked --workspace --lib --bins -- -D warnings
- cargo check --locked --workspace
- cargo test --locked --workspace

The workspace test command includes unit tests, integration tests, UI snapshots,
mock harness scenarios, manifest/artifact checks and real-Git smoke tests.
Do not rerun name-filtered snapshot/harness subsets after a successful full run.
Keep the explicit workspace flag so future default-members changes cannot
silently reduce coverage. Test-only Clippy is advisory; production lint is strict.

Unix-only PTY tests verify normal exit restoration; Windows runs isolated
startup/redirection process regressions. Local Windows success does not establish
Ubuntu success. Builds without a terminal fail before alternate-screen entry.

## Coverage and failures

Use the risk-based coverage policy in [TESTING.md](TESTING.md). New interaction
flows and Git mutation semantics require harness coverage; existing scenarios
may be extended. Pure presentation changes need reviewed UI coverage.
Commit intended snapshot changes and inspect their diffs.

The workflow uploads failure artifacts, including snapshot diffs and harness
reports. Investigate failed checks before merging; do not suppress regressions.
