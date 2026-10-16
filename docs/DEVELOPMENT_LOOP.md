# Development workflow

## Choose the verification before the implementation

Define the observable outcome, inspect relevant code/tests, and choose the
smallest coherent change that satisfies it. Implement, run focused checks,
inspect results and repair failures. Test-first reproduction is useful for bugs;
there is no mandatory ordering of every code and test edit.

Use the coverage table in [TESTING.md](TESTING.md). A task need not add unit,
snapshot and harness tests independently if existing checks already prove the
acceptance criteria. Identify the relevant test or assertion in the handoff.

## Plans proportional to risk

A small documentation fix, pure presentation adjustment or bounded local bug
can use a short goal and validation note in the conversation. No plan file is
required solely because something is user-visible.

Use [EXEC_PLAN.md](EXEC_PLAN.md) before implementation for multi-step or ambiguous
work, cross-layer changes, Git mutation semantics, async/runtime changes, and
refactors with substantial regression risk. Include:

- Outcome and observable acceptance criteria.
- Scope, non-goals, affected areas, and risks.
- Verification: existing coverage, gaps, and commands.
- For long work: current state, decisions, remaining work, and blockers.

Keep the existing EXEC_PLAN headings (Active Slice, Problem, Smallest Slice,
Non-Goals, Expected Files, Tests, Harness Decision, Validation, Completion
Evidence) when replacing a plan; tests check this durable handoff structure.
This structure does not require updating the file for small tasks.

The plan is a living contract, not a fixed implementation recipe. Revise it as
evidence changes. Cross-layer edits may form one coherent slice; explain the
dependency rather than splitting changes into artificial incomplete steps.
Replace completed plans when starting a new substantial task; Git history
preserves earlier evidence. Routine authorized work needs no extra approval
checkpoint just because a plan exists.

## Context and continuity

Start with AGENTS.md, then load relevant architecture, product and test sections
on demand. Use actual source and test output to resolve stale documentation.
Leave enough evidence to resume long tasks: relevant files, decisions, actual
commands/results, remaining work. Do not copy raw tool transcripts into rules
or make a new permanent rule for every one-off failure.

Use one agent by default. A separate review or independently scoped delegation
may be useful when requested and supported; a mandatory planner/implementer/
reviewer pipeline is not part of this repository's workflow.

## Verification and delivery

During iteration, run the affected package/test first. Run the AGENTS.md final
gate once the code is ready. CI runs the whole workspace on Windows and Ubuntu.
Repeat a passed check only after relevant changes or new evidence justifies it.
Pure documentation work needs content/link review and git diff --check.

Investigate failures rather than blindly retrying. Fix regressions introduced
by the task; report unrelated baseline failures or unavailable environments
with evidence. Do not describe incomplete validation as passing. Review snapshot
changes against intended behavior, never bulk-accept unexplained differences.

Before delivery, review the diff for scope and architectural invariants. Report
what changed, which acceptance criteria were checked, and remaining uncertainty.
Commit/push/merge follows the user's requested delivery scope, not an automatic
step in every iteration.

## Improving the workflow

The rationale and a bounded comparison protocol are in
[AGENT_WORKFLOW_REVIEW.md](AGENT_WORKFLOW_REVIEW.md). Revisit rules after observed
failures or model/tool changes. Prefer evidence over additional process.
