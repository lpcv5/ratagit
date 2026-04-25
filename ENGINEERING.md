# ENGINEERING.md

## Purpose

This document defines engineering practices for ratagit.

It ensures the project remains:
- maintainable
- testable
- observable
- evolvable

---

## Golden Rules

1. Always keep the system working
2. Always keep the system testable
3. Always keep the system observable
4. Never sacrifice determinism
5. Prefer small, safe iterations

---

## Change Strategy

All changes must be:

- incremental
- reversible
- test-covered
- observable

---

## No Big Bang Changes

Forbidden:

- large refactors without tests
- cross-layer rewrites
- introducing multiple concepts at once

---

## Vertical Slice First

Always implement:

```text
State -> Logic -> UI -> Test -> Harness
```

Not:

```text
All UI first
All logic first
```

---

## Stability First

Before adding features:

- fix flaky tests
- fix snapshot instability
- fix harness failures

---

## Determinism Enforcement

All outputs must be reproducible:

- UI
- logs
- test results

---

## Debuggability

Every failure must be:

- reproducible
- inspectable
- explainable
