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
- Use AppContext as the only source of truth
- Add tests before expanding scope
- Update docs when behavior changes
- Use the `apply_patch` tool for code edits instead of command-line file writes

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
- cargo clippy --workspace --lib --bins -- -D warnings
- cargo test

Strict clippy is required for production library and binary targets. Test code is
validated by `cargo test`, UI snapshots, and harness scenarios; clippy findings
inside test-only targets are advisory unless they indicate a broken test flow,
nondeterminism, or invalid assertions.

---

## Git Commit Rules

Commit messages must use Conventional Commits:

```text
type(scope): summary
```

- `type` must be one of: `feat`, `fix`, `refactor`, `test`, `docs`,
  `chore`, `style`, `perf`, `build`, `ci`, or `revert`
- `scope` is required and should name the package or subsystem, such as
  `ui`, `core`, `git`, `harness`, `docs`, or `repo`
- `summary` must be concise, imperative, lowercase after the scope, and must
  not end with a period
- commit body is optional

Examples:

- `refactor(ui): add reusable modal system`
- `docs(repo): add commit message rules`

---

## Documentation

- Behavior changes → update PRODUCT.md
- Design changes → update DESIGN.md
- Architecture changes → update ARCHITECTURE.md
