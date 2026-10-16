# Active development plan

## Active Slice

Name: Exercise the actual TUI across everyday Git workflows and fix defects
Status: completed with split-index compatibility follow-up

## Problem

The user requests scenario testing across everyday Git workflows and concrete bug
findings. Existing mock scenarios and passing tests may miss real-Git edge cases,
including staged/unstaged mixtures, rename/delete paths, conflict recovery, stash
identity, remote tracking, history operations, and asynchronous state transitions.

## Smallest Slice

The user clarified that primary evidence must come from opening and interacting
with the actual application, not test suites. Launch the built ratagit.exe in a
Windows pseudo-terminal, send real keys, capture the terminal screen, and compare
on-disk Git state in isolated fixtures. Exercise independent everyday workflow
groups. Use source inspection only to diagnose observed problems. Record keys,
expected/actual behavior, terminal captures and real repository state in a report.
The earlier suite-based audit and baseline were stopped after this clarification.
After independent verification, fix confirmed defects with focused regression
coverage, then run formatting, strict production Clippy and full workspace tests.
Acceptance: findings have concrete evidence and regression coverage; passing
coverage and untested areas are distinguished; the working repository's Git
history and user data are intact.

## Non-Goals

The user expanded scope to fix confirmed bugs alongside testing. No commit/push,
remote network writes, destructive operations on this checkout, exhaustive Git
parity claim, or bulk snapshot acceptance. Test Git mutations run only in
disposable repositories under tmp/git-tests/. Do not use rtk or apply_patch;
use Cargo/Git directly and dedicated file editing tools.

## Expected Files

- docs/EXEC_PLAN.md: scope and completion evidence.
- docs/GIT_WORKFLOW_AUDIT.md: scenario coverage and verified findings.
- tmp/git-workflow-audit/: ConPTY driver, real application key sequences,
  captured terminal screens, Git state observations and audit logs.
- Affected production source and focused regression tests for live-reproduced
  defects, selected after source diagnosis. Use dedicated editing tools, never
  rtk or apply_patch, as explicitly requested by the user.

## Tests

Primary verification: real executable, Windows ConPTY input/output, actual
terminal screen capture and Git state assertions after real key sequences.
Scenarios target file/partial staging, commits, branch changes, stash, local bare
remote sync, conflicts and history operations. Reproduce defects before fixing,
then repeat the same actual-app scenario after rebuilding. Regression tests and
the required full Cargo gate supplement, and do not replace, live interaction.

## Harness Decision

Actual TUI interaction is primary. Existing harness and focused regressions
supplement fixes to prevent recurrence, with rendered state/trace/Git state where
needed. First confirmed live defect: staged rename destination Space leaves old
path deletion staged; fix HybridGitBackend unstage semantics and exercise the
real key path again after rebuild. A second independently reproduced defect:
Files discard of [abc].txt uses Git glob pathspecs, discarding unrelated a.txt
instead. Apply literal pathspecs, add backend and rendered real-Git regression
coverage, then replay the actual TUI confirmation. No mock scenario manifest
change is expected.

Expanded after additional live evidence: selected-path stash snapshots unrelated
staged changes; stash preview omits third-parent untracked files; conflicting
stash pop leaves Files stale despite UU index state. Fix selected stash payload
scope/literal paths, append bounded untracked stash previews, and refresh after
failed pop. Independent review also exposed a Windows case-only rename order bug,
confirmed via actual a.txt -> A.txt UI scenario; use separate remove/restore
passes and preserve symlink rename sources. Repeat actual TUI scenarios for these
changes, then rerun final gates. Protect temporary split-index operations from
shared-index expiration without changing persistent repository config; independently
reviewed aggressive-expiration interaction requires scoped command configuration
and regression coverage.

## Validation

Use Cargo/Git directly per user instruction. Build ratagit.exe, run actual TUI
scenarios and inspect captures plus on-disk state. After fixes run cargo fmt
--check, cargo clippy --locked --workspace --lib --bins -- -D warnings, cargo test
--locked --workspace and git diff --check. Preserve evidence under
tmp/git-workflow-audit/. Report native Windows coverage and platform limitations.

## Completion Evidence

Actual native Windows TUI coverage completed across file/index operations,
partial hunks, first commit, branch/remote workflows, stash modes, auto-stash,
history rewrites and conflict recovery. Eight confirmed functional defects were
fixed and replayed successfully in the rebuilt executable: rename unstage,
literal discard, case-only rename, symlink-object rename, literal selected stash,
selected stash payload scope, untracked stash preview and failed-pop refresh.
Evidence and bounded scenario coverage: docs/GIT_WORKFLOW_AUDIT.md and
tmp/git-workflow-audit/live-evidence.html, keys/screens/raw output + Git state.

Final follow-up: shared-index expiration suppression added to temporary stash
commands, with isolated success/failure regression; final rebuilt ordinary stash
replay passed (parent-stash-final-results.json). Final full workspace gate passed
664 tests, zero failed/ignored (final-workspace-complete.log); cargo fmt --check,
strict production workspace Clippy, final build and git diff --check passed.
Final test command uses --target-dir tmp/git-workflow-audit/fixed-target to avoid
overwriting Windows executables held by early sessions. No snapshots changed or
pending .snap.new files.

An extra actual-app split-index limitation remains (libgit2 mandatory 'link'
extension rejects index, Files cannot load); separate follow-up chip offered.
Do not claim live selected-stash support for this configuration. No commit/push,
network mutation or checkout Git history changes. Early suite-based audit stopped
when user required actual app testing; all commands use plain Cargo/Git and
edits use dedicated tools as requested.
