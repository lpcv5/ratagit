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
