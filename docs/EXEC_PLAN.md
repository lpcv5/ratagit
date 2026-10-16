# Active development plan

## Active Slice

Name: Audit Git execution efficiency
Status: completed

## Problem

Measure Git read latency and inspect repeated work in status, commit pagination,
and diff loading. Commit reachability classification may scan all of main history
when a page contains commits that are not reachable from main.

## Smallest Slice

Run the release performance suite on smoke, small, and medium isolated fixtures.
Supplement it with packed histories (1k/10k/50k/250k commits) and synthetic indexes
(1k/10k/100k/500k entries). Apply the user's 150 ms tolerance: leave small costs
alone and identify material delays with evidence. Keep this delivery an audit;
the exploratory index optimization and its test were withdrawn after that scope
clarification. Historical patches also load Git attributes through libgit2, so
removing just the explicit index refresh does not remove every index dependency.
Record timings, limitations, and follow-up findings in docs/GIT_PERFORMANCE.md.

## Non-Goals

No mutations in the workspace repository, network benchmarks, UI behavior changes,
new caches outside AppContext, or changes to async ownership/mutation barriers.
Do not generate million-file fixtures for this audit.

## Expected Files

docs/EXEC_PLAN.md and docs/GIT_PERFORMANCE.md. Disposable measurement source and
raw reports stay under ignored workspace tmp/; production code stays unchanged.

## Acceptance criteria

- Reproducible release measurements compare raw CLI and backend read latency.
- Distinguish measured regressions from static risks and benchmark limitations.
- Rank material delays using the user's tolerance; do not optimize small costs.
- Preserve application behavior and working-tree changes.

## Tests

Inspect existing real-Git commit classification and pagination tests. Use isolated
repositories under tmp/ for synthetic fixtures. Prefer deterministic result
assertions over wall-clock test thresholds. Specialized measurements assert
commit counts, classifications, and native patch file count.

## Harness Decision

No code or interaction/mutation semantics change remains. Backend timings and
source inspection suffice for the audit; no new product harness scenario.

## Validation

Release perf-suite and specialized measurements; check reports for failed records;
content/link review and git diff --check. No Rust rebuild gate for the final
documentation-only diff (cargo fmt and cargo test --workspace are not required).
Run the focused execution-plan structure tests for the updated handoff document.

## Completion Evidence

Release suite completed: 315 measurements and 450 profiles, zero failed records.
Current workspace reads were below 80 ms median. Supplemental final measurements:
250k-history feature refresh 223.733 ms versus raw log 40.435 ms; 500k-index native
Commit patch 260.129 ms versus 0.994 ms at 1k index entries. Supplemental result
assertions passed. Details and local artifact paths are in docs/GIT_PERFORMANCE.md.

The user's 150 ms tolerance changed the task to a focused audit. The exploratory
index change/test were removed, and the final diff contains documentation only.
Final content/local-link review and git diff --check passed. The focused command
`cargo test --locked --test exec_plan` passed both tests. No production source,
test, dependency, or design changes remain in the working tree.
