# Development workflow

[AGENTS.md](../AGENTS.md) owns repository invariants and the final validation
gate. [TESTING.md](TESTING.md) owns coverage selection; [TOOLING.md](TOOLING.md)
contains tool recipes. Read relevant sections on demand.

## Implement and verify

1. Define the observable outcome in the conversation and inspect affected code
   and tests. No plan file is required.
2. Choose coverage for the changed contract. Reuse existing assertions when
   they prove the case; add a regression when a gap remains.
3. Implement a coherent change and run focused checks. For bugs, a failing
   reproduction is useful; code and test edits have no mandatory ordering.
4. Inspect failures and intended snapshot changes. Fix task regressions;
   report unrelated failures or unavailable environments with evidence.
5. Run the AGENTS.md final gate once the change is ready. Repeat passed checks
   only after relevant changes or new evidence justifies it.

Cross-layer changes may be one complete, verifiable change. Avoid unrelated
refactors and tests added only to satisfy a checklist.

## Continuity and delivery

For long tasks, keep decisions, actual results, remaining work and blockers in
the conversation. Use source and test output to resolve stale documentation.

Before delivery, review the diff for scope and invariants. Report the result,
actual validation and unresolved limitations. Commit, push and merge only within
the user's requested scope.

Change workflow rules in response to observed friction or failures. Prefer a
small, verifiable improvement over another mandatory process. Historical
rationale is in [AGENT_WORKFLOW_REVIEW.md](AGENT_WORKFLOW_REVIEW.md).
