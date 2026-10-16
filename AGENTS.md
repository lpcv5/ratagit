# Working in ratagit

ratagit is a Rust Git TUI. Deliver the requested behavior with evidence that it
works; choose implementation steps to fit the task.

## Invariants

Follow [ARCHITECTURE.md](ARCHITECTURE.md): AppContext owns application state,
reducers own transitions, rendering is pure, and Git access goes through
GitBackend. Keep async request ownership and mutation barriers intact.
Use apply_patch for code edits. Preserve unrelated working-tree changes.

## Workflow

- Inspect the affected code and existing tests before choosing a solution.
- Define acceptance criteria. Small, bounded changes can keep the goal and
  validation in the conversation; they do not require a plan file.
- Use [docs/EXEC_PLAN.md](docs/EXEC_PLAN.md) for multi-step, ambiguous,
  cross-layer, Git mutation, or async/runtime changes. Record scope, risks,
  acceptance criteria and validation before implementation; update when scope
  changes. A coherent cross-layer change is allowed when needed for the outcome.
- Implement and verify in useful increments. Reuse or extend existing tests;
  add regression coverage for new behavior or uncovered bugs. Do not generate
  redundant tests just to complete a checklist.
- Run focused checks while iterating, then the final gate below. Inspect failures
  and snapshot diffs; do not weaken assertions to make a change pass.
- Report the outcome, actual checks, and any unresolved limitations.

See [development workflow](docs/DEVELOPMENT_LOOP.md) for planning and handoffs,
and [testing guidance](docs/TESTING.md) when selecting coverage.

## Validation

For code, dependency, build or CI changes, run:

```text
cargo fmt --check
cargo clippy --locked --workspace --lib --bins -- -D warnings
cargo test --locked --workspace
```

Run cargo fmt when formatting needs correction. Documentation-only changes
need content/link review and git diff --check, not a Rust rebuild.
Strict Clippy applies to production libraries/binaries; test-only lint findings
are advisory unless they expose a broken or unreliable test.

UI rendering changes require reviewed deterministic snapshot coverage; assert
buffer styles separately where text cannot prove the result. Every panel needs
a fixture. New interaction flows and Git mutations require harness coverage of
UI, operation trace and Git state. Pure presentation changes may use UI tests;
existing coverage can satisfy a requirement when it exercises the changed case.
Keep failure artifacts and isolate real Git tests under workspace tmp/.
See [testing guidance](docs/TESTING.md) for the coverage decision table.

## Documentation and commits

Update docs/PRODUCT.md for behavior, docs/DESIGN.md for design, and
ARCHITECTURE.md for architecture changes. Read relevant sections on demand.

When committing, use type(scope): summary with a concise, imperative, lowercase
summary and no final period. Types: feat, fix, refactor, test, docs, chore, style,
perf, build, ci, revert. Scope names the affected package or subsystem.
