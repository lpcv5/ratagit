## Purpose

This repository is designed for iterative development by Codex.

The system is spec-driven and harness-validated.

---

## Required Loop

For every feature:

1. Create or update an exec plan
2. Implement smallest vertical slice
3. Add unit tests
4. Add snapshot tests (if UI changes)
5. Add or update harness scenario
6. Run all checks
7. Fix failures before continuing

---

## You MUST

- Follow ARCHITECTURE.md strictly
- Keep rendering pure
- Use AppState as the only source of truth
- Add tests before expanding scope
- Update docs when behavior changes

---

## You MUST NOT

- Bypass GitBackend
- Access external state in UI
- Introduce hidden state
- Modify multiple layers in one step without justification
- Skip tests

---

## UI Rules

- All UI must be snapshot-tested
- Every panel must have a fixture
- Rendering must be deterministic

---

## Harness Rules

- Every user-visible feature requires a scenario
- Scenarios must assert:
  - UI
  - Git state
- Failures must produce artifacts

---

## Code Quality

Before finishing:

- cargo fmt
- cargo clippy --all-targets -- -D warnings
- cargo test

---

## Documentation

- Behavior changes → update PRODUCT.md
- Design changes → update DESIGN.md
- Architecture changes → update ARCHITECTURE.md
