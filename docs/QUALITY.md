## Definition of Done

A feature is complete only if:

- logic implemented
- unit tests exist
- UI snapshot exists
- harness scenario exists
- documentation updated

---

## Invariants

- No panic in normal flow
- No UI overflow
- No broken layout
- No inconsistent state

---

## Regression Prevention

- snapshots must pass
- harness must pass

---

## CI Baseline

Every PR must pass:

- cargo fmt --check
- cargo clippy --all-targets -- -D warnings
- cargo test
- snapshot tests
- harness scenarios

---

## Failure Handling

- fix CI failures immediately
- do not merge partial fixes
- review all snapshot changes before merge
