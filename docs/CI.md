# CI.md

## Required Checks

Every PR must pass:

- cargo fmt --check
- cargo check --locked
- cargo clippy --locked --all-targets --all-features -- -D warnings
- cargo test --locked
- snapshot tests
- harness scenarios

The workflow runs on Ubuntu and Windows. Unix-only PTY tests verify normal exit
restoration; Windows runs isolated startup/redirection process regressions.
Builds without an interactive terminal fail before alternate-screen entry.

---

## Snapshot Policy

- snapshots must be committed
- snapshot changes must be reviewed

---

## Harness Policy

- new features require new scenarios
- scenarios must not be flaky

---

## Failure Handling

If CI fails:

- fix immediately
- do not merge partial fixes
