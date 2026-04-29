# DEVELOPMENT_LOOP.md

## Loop

Each iteration must follow:

1. Define goal (exec plan)
2. Implement minimal change
3. Add tests
4. Run:
   - unit tests
   - snapshot tests
   - harness
5. Fix all failures
6. Commit

---

## Exec Plan Sizing

Tiny changes do not require a standalone exec plan. Use a short goal, change
summary, and validation note for typo fixes, comments, fixture polish, and
internal-only test cleanup.

Use a lightweight plan for one-module or one-behavior changes. Include the goal,
acceptance criteria, test impact, harness impact, and validation commands.

Use a full exec plan when the change crosses layers, changes user-visible
behavior, changes architecture, changes Git state semantics, changes async or
runtime behavior, or needs multiple slices.

Do not expand a tiny task into a full harness workflow unless risk is discovered
while working. User-visible behavior changes still require harness coverage;
harness infrastructure changes require targeted harness/unit tests.

---

## Exit Conditions

You CANNOT proceed if:

- tests are failing
- snapshot is broken
- harness fails
- logs indicate inconsistency

---

## Allowed Failures

Only allowed during development:

- compile errors (temporary)
- snapshot mismatch (must fix before commit)

---

## Forbidden

- ignoring failing tests
- skipping harness
- committing broken UI
