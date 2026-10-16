# Active development plan

## Active Slice

Name: Make agent workflow proportional to risk
Status: completed

## Problem

Agent instructions prescribe work independent of risk, duplicate policies across
documents, and CI reruns snapshot/harness subsets after the complete suite.

## Smallest Slice

Research current primary sources and adopt an outcome-driven, risk-based workflow.
Update AGENTS.md, development/testing/CI/tooling guidance and CI commands together
so contributors receive one consistent contract. Record research and an evaluation
protocol in docs/AGENT_WORKFLOW_REVIEW.md.

## Non-Goals

No application behavior, architecture, runtime, dependencies, product assertions,
snapshot baselines, model configuration, or remote repository changes.

## Expected Files

AGENTS.md, docs/DEVELOPMENT_LOOP.md, docs/TESTING.md, docs/CI.md,
docs/TOOLING.md, docs/AGENT_WORKFLOW_REVIEW.md, docs/EXEC_PLAN.md,
.github/workflows/ci.yml, tests/exec_plan.rs. The discovered working-tree gate
requires a narrow test-infrastructure update to implement optional small plans.

## Acceptance criteria

- Short tasks need no plan file; complex/high-risk tasks retain a durable plan.
- Changed behavior has relevant evidence; new tests follow coverage gaps.
- UI snapshots, real-Git isolation, async regressions and failure artifacts remain.
- Full workspace tests run once per CI platform; strict production lint matches
  repository policy. Ubuntu and Windows remain required.
- Research distinguishes vendor evidence from project-specific hypotheses.

## Tests

Retain plan structure/completion tests; remove the obsolete test requiring every
dirty worktree to edit its plan. This rule cannot classify risk and is bypassed
by a clean CI checkout. Product assertions are unchanged.
Validate CI test selection against Cargo's test inventory, review workflow and
documentation consistency, and run the complete existing suite. Existing harness
tests verify artifact generation and scenario manifest consistency.

## Harness Decision

No new product scenario: only workflow, CI and plan-test infrastructure changes.
Run focused plan tests and existing manifest/artifact tests in the full suite.

## Validation

cargo fmt; cargo fmt --check;
cargo clippy --locked --workspace --lib --bins -- -D warnings;
cargo check --locked --workspace; cargo test --locked --workspace;
git diff --check. Record actual results after execution.

## Completion Evidence

Windows validation passed: cargo fmt, cargo fmt --check, production Clippy,
cargo check --locked --workspace, and cargo test --locked --workspace (605 tests,
zero failures or ignored tests). Focused plan tests passed. Cargo test inventory
confirmed snapshot, harness, manifest and failure-artifact coverage in the full
workspace command. Local documentation links and git diff --check passed.

The first run exposed the old plan-shape requirement and working-tree policy;
the plan retains its structured headings and only the obsolete dirty-worktree
test was removed. Product tests and assertions remain intact. Ubuntu CI and
agent A/B efficiency measurements have not been run locally.
