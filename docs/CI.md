# CI.md

## Required Checks

Every PR must pass:

- cargo fmt --check
- cargo clippy --all-targets -- -D warnings
- cargo test
- snapshot tests
- harness scenarios

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
