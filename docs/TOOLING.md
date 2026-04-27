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
