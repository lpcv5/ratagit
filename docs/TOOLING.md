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
cargo clippy --all-targets -- -D warnings
cargo test
```

---

## Snapshot

```bash
cargo insta review
```
